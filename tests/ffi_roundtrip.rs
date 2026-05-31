//! real libdave roundtrip
//! an MLS external sender plus two member sessions establish a group end-to-end.
//! mirrors libdave's `cpp/test/capi/basic_tests.c::TestSession`
#![cfg(feature = "dave-ffi")]

use ug_dave::Dave;

#[test]
fn linked_libdave_reports_protocol_version() {
    assert!(
        Dave::max_protocol_version() >= 1,
        "linked libdave must report protocol version >= 1"
    );
    assert!(Dave::is_available());
}

#[test]
fn mls_group_establishes_between_two_members() {
    let group_id = 1_486_026_111_000_000_000u64; // a voice channel id
    let user_a = "1234123412341234";
    let user_b = "5678567856785678";

    let ext = Dave::create_external_sender(group_id, 1).expect("external sender");
    let mut a = Dave::create_session(group_id, user_a, 1).expect("session A");
    let mut b = Dave::create_session(group_id, user_b, 1).expect("session B");
    assert_eq!(a.protocol_version(), 1, "session negotiates protocol 1");

    // server advertises the external-sender package (OP 25); members install it.
    let package = ext.marshalled_package().expect("external sender package");
    a.set_external_sender(&package).expect("A installs external sender");
    b.set_external_sender(&package).expect("B installs external sender");

    // members produce key packages (OP 26).
    let _kp_a = a.marshalled_key_package().expect("A key package");
    let kp_b = b.marshalled_key_package().expect("B key package");

    // server proposes adding B (OP 27); A commits (OP 28).
    let proposal = ext.propose_add(0, &kp_b).expect("propose add B");
    let commit_welcome = a
        .process_proposals(&proposal, &[user_a, user_b])
        .expect("A processes proposals");

    // server splits into commit (OP 29 announce) + welcome (OP 30).
    let (commit, welcome) = ext.split_commit_welcome(&commit_welcome).expect("split commit/welcome");

    let roster_a = a.process_commit(&commit).expect("A processes commit");
    let roster_b = b
        .process_welcome(&welcome, &[user_a, user_b])
        .expect("B processes welcome");

    let mut expected = vec![1_234_123_412_341_234u64, 5_678_567_856_785_678u64];
    expected.sort_unstable();
    let mut got_a = roster_a.member_ids;
    got_a.sort_unstable();
    let mut got_b = roster_b.member_ids;
    got_b.sort_unstable();
    assert_eq!(got_a, expected, "A's roster contains both members");
    assert_eq!(got_b, expected, "B's roster contains both members");

    // both members agree on the epoch authenticator -> same group state.
    let auth_a = a.last_epoch_authenticator().expect("A epoch authenticator");
    let auth_b = b.last_epoch_authenticator().expect("B epoch authenticator");
    assert_eq!(auth_a, auth_b, "epoch authenticators must match across members");
}
