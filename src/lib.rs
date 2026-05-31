//! libdave C ABI
//! some important info :
//! in the Celeste voice topology two distinct MLS roles exist:
//! - the server = MLS external sender. Celeste does not hold the group's encryption keys (that is what makes DAVE end-to-end). it only creates the external-sender credential ([`ExternalSender`]), proposes adds/removes, and splits the clients' commit/welcome for relay. this is what the voice gateway uses at runtime
//! - the client = MLS member ([`Session`]). the member generates key packages, processes proposals into commits, and derives per-sender key ratchets to encrypt/decrypt media. the Celeste server never does this
//! by the way [`Session`] is wrapped here only so the FFI boundary can be exercised end-to-end in tests (and for completeness)
//! important : the external-sender surface (`daveExternalSender*`) is not part of libdave's public `dave.h` C ABI. it lives in libdave's test tree (`cpp/test/capi/external_sender_wrapper.*`) and links `mlspp` directly. because of this the `dave-ffi` build compiles that wrapper alongside the public ABI
//! the voice gateway gates `dave_protocol_version: 1` on  [`Dave::is_available`] and successful state init. so a stub build (or a libdave that reports version 0) degrades to non-dave honestly. in the official discord client : « secured connection » means no dave, « end to end encrypted » means dave is active
pub mod error;

pub use error::DaveError;

#[cfg(not(feature = "dave-ffi"))]
mod stub;
#[cfg(not(feature = "dave-ffi"))]
use stub as imp;

#[cfg(feature = "dave-ffi")]
mod ffi;
#[cfg(feature = "dave-ffi")]
use ffi as imp;

/// DAVE/MLS protocol version.
/// `1` is the only version libdave currently supports; `0` means no dave
pub type ProtocolVersion = u16;

/// MLS group identifier. In Celeste this is the voice channel id.
pub type GroupId = u64;

/// The MLS roster (member user ids) after processing a commit or welcome.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roster {
    /// User ids of the members in the group after the transition.
    pub member_ids: Vec<u64>,
}

pub struct Dave;

impl Dave {
    #[must_use]
    pub fn max_protocol_version() -> ProtocolVersion {
        imp::max_protocol_version()
    }

    /// Whether DAVE can be negotiated at all in this build.
    #[must_use]
    pub fn is_available() -> bool {
        Self::max_protocol_version() >= 1
    }

    /// Create the server-side MLS external sender for a room
    /// (`group_id` = voice channel id).
    /// This is the role the Celeste voice gateway plays.
    pub fn create_external_sender(
        group_id: GroupId,
        version: ProtocolVersion,
    ) -> Result<ExternalSender, DaveError> {
        imp::create_external_sender(group_id, version).map(ExternalSender)
    }

    /// Create a client-side MLS session.
    /// Wrapped for boundary tests and completeness; the Celeste *server* uses [`ExternalSender`], not this
    pub fn create_session(
        group_id: GroupId,
        self_user_id: &str,
        version: ProtocolVersion,
    ) -> Result<Session, DaveError> {
        imp::create_session(group_id, self_user_id, version).map(Session)
    }
}

/// server-side MLS external sender for one room, holds no group key
pub struct ExternalSender(imp::ExternalSender);

impl ExternalSender {
    /// marshalled external-sender credential to advertise to clients
    /// (DAVE voice gateway OP 25 payload body)
    pub fn marshalled_package(&self) -> Result<Vec<u8>, DaveError> {
        self.0.marshalled_package()
    }

    /// propose adding a member (from its marshalled key package) at `epoch`
    /// returns the marshalled proposal (OP 27 payload body)
    pub fn propose_add(&self, epoch: u32, key_package: &[u8]) -> Result<Vec<u8>, DaveError> {
        self.0.propose_add(epoch, key_package)
    }

    /// propose removing the member at MLS `leaf_index` at `epoch`
    /// returns the marshalled proposal (OP 27 payload body).
    ///
    /// spec mandates the external sender propose the removal of a participant when it disconnects
    /// (unofficial documentation/userdoccers voice-connections.mdx L525-527)
    ///
    /// a remaining member commits it so the group advances one epoch without a destructive re-key
    pub fn propose_remove(&self, epoch: u32, leaf_index: u32) -> Result<Vec<u8>, DaveError> {
        self.0.propose_remove(epoch, leaf_index)
    }

    /// split a client's combined commit+welcome (OP 28 body) into the commit (relayed as OP 29 announce) and the welcome (relayed as OP 30)
    pub fn split_commit_welcome(
        &self,
        commit_welcome: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), DaveError> {
        self.0.split_commit_welcome(commit_welcome)
    }
}

/// Client-side MLS member session. Wrapped for tests/completeness only.
pub struct Session(imp::Session);

impl Session {
    pub fn protocol_version(&self) -> ProtocolVersion {
        self.0.protocol_version()
    }

    /// Install the group's external-sender credential (from OP 25)
    pub fn set_external_sender(&mut self, package: &[u8]) -> Result<(), DaveError> {
        self.0.set_external_sender(package)
    }

    /// Marshalled key package to send to the server (OP 26 body)
    pub fn marshalled_key_package(&mut self) -> Result<Vec<u8>, DaveError> {
        self.0.marshalled_key_package()
    }

    /// Process proposals (OP 27 body) and produce a commit+welcome (OP 28 body)
    pub fn process_proposals(
        &mut self,
        proposals: &[u8],
        recognized_user_ids: &[&str],
    ) -> Result<Vec<u8>, DaveError> {
        self.0.process_proposals(proposals, recognized_user_ids)
    }

    /// Process an MLS commit (from OP 29), returning the resulting roster
    pub fn process_commit(&mut self, commit: &[u8]) -> Result<Roster, DaveError> {
        self.0.process_commit(commit)
    }

    /// Process an MLS welcome (from OP 30), returning the resulting roster
    pub fn process_welcome(
        &mut self,
        welcome: &[u8],
        recognized_user_ids: &[&str],
    ) -> Result<Roster, DaveError> {
        self.0.process_welcome(welcome, recognized_user_ids)
    }

    /// the authenticator for the last MLS epoch (used by clients to verify the group is consistent; matching authenticators across members prove the same group state)
    pub fn last_epoch_authenticator(&mut self) -> Result<Vec<u8>, DaveError> {
        self.0.last_epoch_authenticator()
    }
}
