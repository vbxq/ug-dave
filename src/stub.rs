//! inert stub backet, de fault guild
//! a stub build can never accidentally drive a fake DAVE flow

use crate::{DaveError, GroupId, ProtocolVersion, Roster};

const UNAVAILABLE: &str = "libdave not linked (build without --features dave-ffi)";

pub(crate) fn max_protocol_version() -> ProtocolVersion {
    0
}

pub(crate) fn create_external_sender(
    _group_id: GroupId,
    _version: ProtocolVersion,
) -> Result<ExternalSender, DaveError> {
    Err(DaveError::Unavailable(UNAVAILABLE))
}

pub(crate) fn create_session(
    _group_id: GroupId,
    _self_user_id: &str,
    _version: ProtocolVersion,
) -> Result<Session, DaveError> {
    Err(DaveError::Unavailable(UNAVAILABLE))
}

pub(crate) enum ExternalSender {}

impl ExternalSender {
    pub(crate) fn marshalled_package(&self) -> Result<Vec<u8>, DaveError> {
        match *self {}
    }
    pub(crate) fn propose_add(&self, _epoch: u32, _key_package: &[u8]) -> Result<Vec<u8>, DaveError> {
        match *self {}
    }
    pub(crate) fn split_commit_welcome(
        &self,
        _commit_welcome: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), DaveError> {
        match *self {}
    }
}

pub(crate) enum Session {}

impl Session {
    pub(crate) fn protocol_version(&self) -> ProtocolVersion {
        match *self {}
    }
    pub(crate) fn set_external_sender(&mut self, _package: &[u8]) -> Result<(), DaveError> {
        match *self {}
    }
    pub(crate) fn marshalled_key_package(&mut self) -> Result<Vec<u8>, DaveError> {
        match *self {}
    }
    pub(crate) fn process_proposals(
        &mut self,
        _proposals: &[u8],
        _recognized_user_ids: &[&str],
    ) -> Result<Vec<u8>, DaveError> {
        match *self {}
    }
    pub(crate) fn process_commit(&mut self, _commit: &[u8]) -> Result<Roster, DaveError> {
        match *self {}
    }
    pub(crate) fn process_welcome(
        &mut self,
        _welcome: &[u8],
        _recognized_user_ids: &[&str],
    ) -> Result<Roster, DaveError> {
        match *self {}
    }
    pub(crate) fn last_epoch_authenticator(&mut self) -> Result<Vec<u8>, DaveError> {
        match *self {}
    }
}
