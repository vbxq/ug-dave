//! contract for the default (stub) DAVE backend: DAVE is reported unavailable and never fakeable
//! these run on the default build, no dave-ffi
#![cfg(not(feature = "dave-ffi"))]

use ug_dave::{Dave, DaveError};

#[test]
fn stub_reports_dave_unavailable() {
    assert_eq!(
        Dave::max_protocol_version(),
        0,
        "a build without libdave must report protocol version 0"
    );
    assert!(!Dave::is_available(), "stub build must not advertise DAVE");
}

#[test]
fn stub_external_sender_is_unavailable_not_fake() {
    // group_id = a voice channel id.
    match Dave::create_external_sender(1_486_026_111_000_000_000, 1) {
        Ok(_) => panic!("stub must not produce a usable external sender"),
        Err(e) => assert!(
            matches!(e, DaveError::Unavailable(_)),
            "stub must report Unavailable (so the gateway omits dave_protocol_version), got {e:?}"
        ),
    }
}

#[test]
fn stub_session_is_unavailable() {
    match Dave::create_session(1_486_026_111_000_000_000, "1000000000000000000", 1) {
        Ok(_) => panic!("stub must not produce a usable session"),
        Err(e) => assert!(matches!(e, DaveError::Unavailable(_)), "got {e:?}"),
    }
}
