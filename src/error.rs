//! Error type for the DAVE boundary.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DaveError {
    /// libdave is not linked into this build or it reported a maximum protocol version of 0
    /// Callers fall back to non-DAVE
    #[error("DAVE unavailable: {0}")]
    Unavailable(&'static str),

    /// a libdave call failed or returned a null/failed handle.
    /// `detail` carries a bounded non-secret description (never key/package/token bytes)
    #[error("DAVE lib error in {operation}: {detail}")]
    Lib {
        operation: &'static str,
        detail: String,
    },

    /// input that could not be marshalled across the FFI boundary (like a user id containing an interior NUL byte)
    #[error("invalid DAVE input: {0}")]
    Invalid(&'static str),
}
