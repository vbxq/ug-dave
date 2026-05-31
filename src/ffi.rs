//! libdave FFI binding
//! All `unsafe` lives in this module
//! memory rules from `dave.h`: every out `uint8_t**` / `uint64_t**` buffer is freed with`daveFree` (which is libc `free`),
//! and every handle with its `*Destroy`
//! libdave does not take ownership of input slices; the external-sender wrapper also `malloc`s its outputs, so `daveFree` frees them too; (`bindings_capi.cpp::daveFree` and the wrapper both use `malloc`/`free`)
#![allow(unsafe_code)]
#![allow(clippy::ptr_as_ptr, clippy::borrow_as_ptr)]
#![allow(clippy::unnecessary_wraps)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use std::sync::Once;

use crate::{DaveError, GroupId, ProtocolVersion, Roster};

// opaque handles (distinct zero-sized types for a little type safety)
#[repr(C)]
struct SessionT {
    _opaque: [u8; 0],
}
#[repr(C)]
struct CommitResultT {
    _opaque: [u8; 0],
}
#[repr(C)]
struct WelcomeResultT {
    _opaque: [u8; 0],
}
#[repr(C)]
struct ExternalSenderT {
    _opaque: [u8; 0],
}

type SessionHandle = *mut SessionT;
type CommitResultHandle = *mut CommitResultT;
type WelcomeResultHandle = *mut WelcomeResultT;
type ExternalSenderHandle = *mut ExternalSenderT;

type MlsFailureCb =
    extern "C" fn(source: *const c_char, reason: *const c_char, user_data: *mut c_void);
type LogSinkCb =
    extern "C" fn(severity: c_int, file: *const c_char, line: c_int, message: *const c_char);

extern "C" {
    fn daveMaxSupportedProtocolVersion() -> u16;
    fn daveFree(ptr: *mut c_void);

    fn daveSessionCreate(
        context: *mut c_void,
        auth_session_id: *const c_char,
        callback: MlsFailureCb,
        user_data: *mut c_void,
    ) -> SessionHandle;
    fn daveSessionDestroy(session: SessionHandle);
    fn daveSessionInit(session: SessionHandle, version: u16, group_id: u64, self_user_id: *const c_char);
    fn daveSessionGetProtocolVersion(session: SessionHandle) -> u16;
    fn daveSessionSetExternalSender(session: SessionHandle, external_sender: *const u8, length: usize);
    fn daveSessionGetMarshalledKeyPackage(
        session: SessionHandle,
        key_package: *mut *mut u8,
        length: *mut usize,
    );
    fn daveSessionProcessProposals(
        session: SessionHandle,
        proposals: *const u8,
        length: usize,
        recognized_user_ids: *const *const c_char,
        recognized_user_ids_len: usize,
        commit_welcome: *mut *mut u8,
        commit_welcome_len: *mut usize,
    );
    fn daveSessionProcessCommit(
        session: SessionHandle,
        commit: *const u8,
        length: usize,
    ) -> CommitResultHandle;
    fn daveSessionProcessWelcome(
        session: SessionHandle,
        welcome: *const u8,
        length: usize,
        recognized_user_ids: *const *const c_char,
        recognized_user_ids_len: usize,
    ) -> WelcomeResultHandle;
    fn daveSessionGetLastEpochAuthenticator(
        session: SessionHandle,
        authenticator: *mut *mut u8,
        length: *mut usize,
    );

    fn daveCommitResultIsFailed(handle: CommitResultHandle) -> bool;
    fn daveCommitResultGetRosterMemberIds(
        handle: CommitResultHandle,
        roster_ids: *mut *mut u64,
        roster_ids_len: *mut usize,
    );
    fn daveCommitResultDestroy(handle: CommitResultHandle);
    fn daveWelcomeResultGetRosterMemberIds(
        handle: WelcomeResultHandle,
        roster_ids: *mut *mut u64,
        roster_ids_len: *mut usize,
    );
    fn daveWelcomeResultDestroy(handle: WelcomeResultHandle);

    fn daveSetLogSinkCallback(callback: LogSinkCb);

    // external-sender wrapper (libexternal_sender.a, compiled from libdave's test tree; mlspp-linked), this is the server's MLS role
    fn daveExternalSenderCreate(group_id: u64) -> ExternalSenderHandle;
    fn daveExternalSenderDestroy(handle: ExternalSenderHandle);
    fn daveExternalSenderGetMarshalledExternalSender(
        handle: ExternalSenderHandle,
        marshalled: *mut *mut u8,
        length: *mut usize,
    );
    fn daveExternalSenderProposeAdd(
        handle: ExternalSenderHandle,
        epoch: u32,
        key_package: *mut u8,
        key_package_len: usize,
        proposal: *mut *mut u8,
        proposal_len: *mut usize,
    );
    fn daveExternalSenderSplitCommitWelcome(
        handle: ExternalSenderHandle,
        commit_welcome: *mut u8,
        commit_welcome_len: usize,
        commit: *mut *mut u8,
        commit_len: *mut usize,
        welcome: *mut *mut u8,
        welcome_len: *mut usize,
    );
}

// log/failure callback bridges (no secret material libdave diagnostics)

extern "C" fn mls_failure_cb(source: *const c_char, reason: *const c_char, _user_data: *mut c_void) {
    tracing::warn!(
        target: "ug_dave",
        source = %cstr_lossy(source),
        reason = %cstr_lossy(reason),
        "libdave MLS failure"
    );
}

extern "C" fn log_sink_cb(severity: c_int, _file: *const c_char, line: c_int, message: *const c_char) {
    // libdave's own diagnostic strings (status/flow). Not Celeste tokens/keys those never transit libdave.
    // Mapped to bounded tracing levels
    let msg = cstr_lossy(message);
    match severity {
        0 => tracing::trace!(target: "ug_dave", line, "{msg}"),
        1 => tracing::debug!(target: "ug_dave", line, "{msg}"),
        2 => tracing::warn!(target: "ug_dave", line, "{msg}"),
        3 => tracing::error!(target: "ug_dave", line, "{msg}"),
        _ => {} // NONE / unknown
    }
}

fn cstr_lossy(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: libdave passes a valid NUL-terminated C string for the duration of the callback
    // we copy it before returning (it frees the string after).
    unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
}

static LOG_SINK_ONCE: Once = Once::new();
fn ensure_log_sink() {
    LOG_SINK_ONCE.call_once(|| unsafe { daveSetLogSinkCallback(log_sink_cb) });
}

// owned-buffer helpers (free libdave/wrapper malloc'd outputs)

/// copy a libdave-allocated byte buffer into a `Vec` and free the original
/// SAFETY: `ptr`/`len` must be the `(out, out_len)` pair just written by a libdave/wrapper call (a `malloc`'d buffer of `len` bytes, or null/0)
unsafe fn take_bytes(ptr: *mut u8, len: usize) -> Vec<u8> {
    if ptr.is_null() {
        return Vec::new();
    }
    let out = if len == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(ptr, len).to_vec()
    };
    daveFree(ptr.cast());
    out
}

/// same as [`take_bytes`] for a `u64` roster-id array
unsafe fn take_u64s(ptr: *mut u64, len: usize) -> Vec<u64> {
    if ptr.is_null() {
        return Vec::new();
    }
    let out = if len == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(ptr, len).to_vec()
    };
    daveFree(ptr.cast());
    out
}

fn cstrings(ids: &[&str]) -> Result<Vec<CString>, DaveError> {
    ids.iter()
        .map(|s| CString::new(*s).map_err(|_| DaveError::Invalid("user id contains an interior NUL")))
        .collect()
}

// backend surface consumed by lib.rs (`imp::*`)

pub(crate) fn max_protocol_version() -> ProtocolVersion {
    // SAFETY: pure C call with no arguments.
    unsafe { daveMaxSupportedProtocolVersion() }
}

pub(crate) fn create_external_sender(
    group_id: GroupId,
    _version: ProtocolVersion,
) -> Result<ExternalSender, DaveError> {
    ensure_log_sink();
    // The wrapper fixes the protocol version to daveMaxSupportedProtocolVersion().
    // SAFETY: returns a heap-allocated handle or null.
    let handle = unsafe { daveExternalSenderCreate(group_id) };
    if handle.is_null() {
        return Err(DaveError::Lib {
            operation: "external_sender_create",
            detail: "null handle".to_string(),
        });
    }
    Ok(ExternalSender { handle })
}

pub(crate) fn create_session(
    group_id: GroupId,
    self_user_id: &str,
    version: ProtocolVersion,
) -> Result<Session, DaveError> {
    ensure_log_sink();
    // SAFETY: callback is a valid `extern "C" fn`; context/user_data may be null.
    let handle = unsafe { daveSessionCreate(ptr::null_mut(), ptr::null(), mls_failure_cb, ptr::null_mut()) };
    if handle.is_null() {
        return Err(DaveError::Lib {
            operation: "session_create",
            detail: "null handle".to_string(),
        });
    }
    // construct first so an error below still destroys the handle via Drop
    let session = Session { handle };
    let uid = CString::new(self_user_id)
        .map_err(|_| DaveError::Invalid("self_user_id contains an interior NUL"))?;
    // SAFETY: handle is non-null; uid is a valid NUL-terminated string.
    unsafe { daveSessionInit(handle, version, group_id, uid.as_ptr()) };
    Ok(session)
}

/// Server-side MLS external sender handle
pub(crate) struct ExternalSender {
    handle: ExternalSenderHandle,
}

// SAFETY: a handle is only ever accessed under external synchronization by the voice layer
// libdave handles are not internally thread-safe
// we grant Send but not Sync
unsafe impl Send for ExternalSender {}

impl Drop for ExternalSender {
    fn drop(&mut self) {
        // SAFETY: handle came from daveExternalSenderCreate and is dropped once.
        unsafe { daveExternalSenderDestroy(self.handle) };
    }
}

impl ExternalSender {
    pub(crate) fn marshalled_package(&self) -> Result<Vec<u8>, DaveError> {
        let mut out = ptr::null_mut();
        let mut len = 0usize;
        // SAFETY: out/len receive a malloc'd buffer freed by take_bytes
        let bytes = unsafe {
            daveExternalSenderGetMarshalledExternalSender(self.handle, &mut out, &mut len);
            take_bytes(out, len)
        };
        non_empty(bytes, "external_sender_marshalled")
    }

    pub(crate) fn propose_add(&self, epoch: u32, key_package: &[u8]) -> Result<Vec<u8>, DaveError> {
        let mut out = ptr::null_mut();
        let mut len = 0usize;
        // SAFETY: key_package is a valid slice; the C side does not retain it
        let bytes = unsafe {
            daveExternalSenderProposeAdd(
                self.handle,
                epoch,
                key_package.as_ptr().cast_mut(),
                key_package.len(),
                &mut out,
                &mut len,
            );
            take_bytes(out, len)
        };
        non_empty(bytes, "external_sender_propose_add")
    }

    pub(crate) fn split_commit_welcome(
        &self,
        commit_welcome: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), DaveError> {
        let (mut commit_ptr, mut welcome_ptr) = (ptr::null_mut(), ptr::null_mut());
        let (mut commit_len, mut welcome_len) = (0usize, 0usize);
        // SAFETY: commit_welcome is a valid slice; both outputs are malloc'd
        let (commit, welcome) = unsafe {
            daveExternalSenderSplitCommitWelcome(
                self.handle,
                commit_welcome.as_ptr().cast_mut(),
                commit_welcome.len(),
                &mut commit_ptr,
                &mut commit_len,
                &mut welcome_ptr,
                &mut welcome_len,
            );
            (take_bytes(commit_ptr, commit_len), take_bytes(welcome_ptr, welcome_len))
        };
        if commit.is_empty() {
            return Err(DaveError::Lib {
                operation: "split_commit_welcome",
                detail: "empty commit".to_string(),
            });
        }
        Ok((commit, welcome))
    }
}

/// client-side MLS member session (wrapped for tests/completeness)
pub(crate) struct Session {
    handle: SessionHandle,
}

unsafe impl Send for Session {}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: handle came from daveSessionCreate and is dropped once.
        unsafe { daveSessionDestroy(self.handle) };
    }
}

impl Session {
    pub(crate) fn protocol_version(&self) -> ProtocolVersion {
        // SAFETY: handle is valid for the session's lifetime
        unsafe { daveSessionGetProtocolVersion(self.handle) }
    }

    pub(crate) fn set_external_sender(&mut self, package: &[u8]) -> Result<(), DaveError> {
        // SAFETY: package is a valid slice the C side copies what it needs
        unsafe { daveSessionSetExternalSender(self.handle, package.as_ptr(), package.len()) };
        Ok(())
    }

    pub(crate) fn marshalled_key_package(&mut self) -> Result<Vec<u8>, DaveError> {
        let mut out = ptr::null_mut();
        let mut len = 0usize;
        // SAFETY: out/len receive a malloc'd buffer freed by take_bytes
        let bytes = unsafe {
            daveSessionGetMarshalledKeyPackage(self.handle, &mut out, &mut len);
            take_bytes(out, len)
        };
        non_empty(bytes, "marshalled_key_package")
    }

    pub(crate) fn process_proposals(
        &mut self,
        proposals: &[u8],
        recognized_user_ids: &[&str],
    ) -> Result<Vec<u8>, DaveError> {
        let cstrs = cstrings(recognized_user_ids)?;
        let ptrs: Vec<*const c_char> = cstrs.iter().map(|c| c.as_ptr()).collect();
        let mut out = ptr::null_mut();
        let mut len = 0usize;
        // SAFETY: proposals + the recognized-id pointer array are valid for the call
        let bytes = unsafe {
            daveSessionProcessProposals(
                self.handle,
                proposals.as_ptr(),
                proposals.len(),
                ptrs.as_ptr(),
                ptrs.len(),
                &mut out,
                &mut len,
            );
            take_bytes(out, len)
        };
        non_empty(bytes, "process_proposals")
    }

    pub(crate) fn process_commit(&mut self, commit: &[u8]) -> Result<Roster, DaveError> {
        // SAFETY: commit is a valid slice, result handle freed in roster_from_commit
        let handle = unsafe { daveSessionProcessCommit(self.handle, commit.as_ptr(), commit.len()) };
        if handle.is_null() {
            return Err(DaveError::Lib {
                operation: "process_commit",
                detail: "null result".to_string(),
            });
        }
        // SAFETY: handle is a fresh, non-null commit result
        unsafe { roster_from_commit(handle) }
    }

    pub(crate) fn process_welcome(
        &mut self,
        welcome: &[u8],
        recognized_user_ids: &[&str],
    ) -> Result<Roster, DaveError> {
        let cstrs = cstrings(recognized_user_ids)?;
        let ptrs: Vec<*const c_char> = cstrs.iter().map(|c| c.as_ptr()).collect();
        // SAFETY: welcome + recognized-id array valid, result handle freed below.
        let handle = unsafe {
            daveSessionProcessWelcome(
                self.handle,
                welcome.as_ptr(),
                welcome.len(),
                ptrs.as_ptr(),
                ptrs.len(),
            )
        };
        if handle.is_null() {
            return Err(DaveError::Lib {
                operation: "process_welcome",
                detail: "null result".to_string(),
            });
        }
        // SAFETY: handle is a fresh, non-null welcome result
        let members = unsafe {
            let mut ids = ptr::null_mut();
            let mut ids_len = 0usize;
            daveWelcomeResultGetRosterMemberIds(handle, &mut ids, &mut ids_len);
            let members = take_u64s(ids, ids_len);
            daveWelcomeResultDestroy(handle);
            members
        };
        Ok(Roster { member_ids: members })
    }

    pub(crate) fn last_epoch_authenticator(&mut self) -> Result<Vec<u8>, DaveError> {
        let mut out = ptr::null_mut();
        let mut len = 0usize;
        // SAFETY: out/len receive a malloc'd buffer freed by take_bytes
        let bytes = unsafe {
            daveSessionGetLastEpochAuthenticator(self.handle, &mut out, &mut len);
            take_bytes(out, len)
        };
        non_empty(bytes, "last_epoch_authenticator")
    }
}

/// SAFETY: `handle` must be a fresh, non-null commit-result handle; this consumes it
unsafe fn roster_from_commit(handle: CommitResultHandle) -> Result<Roster, DaveError> {
    if daveCommitResultIsFailed(handle) {
        daveCommitResultDestroy(handle);
        return Err(DaveError::Lib {
            operation: "process_commit",
            detail: "commit failed".to_string(),
        });
    }
    let mut ids = ptr::null_mut();
    let mut ids_len = 0usize;
    daveCommitResultGetRosterMemberIds(handle, &mut ids, &mut ids_len);
    let members = take_u64s(ids, ids_len);
    daveCommitResultDestroy(handle);
    Ok(Roster { member_ids: members })
}

fn non_empty(bytes: Vec<u8>, operation: &'static str) -> Result<Vec<u8>, DaveError> {
    if bytes.is_empty() {
        Err(DaveError::Lib {
            operation,
            detail: "empty output".to_string(),
        })
    } else {
        Ok(bytes)
    }
}
