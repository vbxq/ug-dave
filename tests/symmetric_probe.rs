//! ignore this file
//! libdave/mlps MLS-semantics regression
//! Run: `cargo test -p ug-dave --features dave-ffi --test symmetric_probe -- --nocapture`
#![cfg(feature = "dave-ffi")]

use ug_dave::Dave;

const GROUP: u64 = 1_486_026_111_000_000_000;
const VER: u16 = 1;
const UA: &str = "1000000000000000001";
const UB: &str = "1000000000000000002";

#[test]
fn key_packages_are_fresh_each_call() {
    let es = Dave::create_external_sender(GROUP, VER).expect("es");
    let pkg = es.marshalled_package().expect("pkg");
    let mut b = Dave::create_session(GROUP, UB, VER).expect("b");
    b.set_external_sender(&pkg).expect("set es");
    let kp1 = b.marshalled_key_package().expect("kp1");
    let kp2 = b.marshalled_key_package().expect("kp2");
    println!("PROBE kp1.len={} kp2.len={} equal={}", kp1.len(), kp2.len(), kp1 == kp2);
    assert_ne!(kp1, kp2, "expected a fresh key package per OP 26 retry");
}

#[test]
fn single_add_commits_ok() {
    let es = Dave::create_external_sender(GROUP, VER).expect("es");
    let pkg = es.marshalled_package().expect("pkg");
    let mut a = Dave::create_session(GROUP, UA, VER).expect("a");
    let mut b = Dave::create_session(GROUP, UB, VER).expect("b");
    a.set_external_sender(&pkg).expect("a es");
    b.set_external_sender(&pkg).expect("b es");
    let _ = a.marshalled_key_package().expect("a founds");
    let kp_b = b.marshalled_key_package().expect("kp_b");
    let add_b = es.propose_add(0, &kp_b).expect("add-b");
    let cw = a.process_proposals(&add_b, &[UA, UB]);
    println!("PROBE single_add: ok={} len={:?}", cw.is_ok(), cw.as_ref().map(Vec::len));
    assert!(cw.is_ok(), "a single add-B must commit");
}

#[test]
fn two_adds_same_member_outcome() {
    let es = Dave::create_external_sender(GROUP, VER).expect("es");
    let pkg = es.marshalled_package().expect("pkg");
    let mut a = Dave::create_session(GROUP, UA, VER).expect("a");
    let mut b = Dave::create_session(GROUP, UB, VER).expect("b");
    a.set_external_sender(&pkg).expect("a es");
    b.set_external_sender(&pkg).expect("b es");
    let _ = a.marshalled_key_package().expect("a founds");
    let kp_b1 = b.marshalled_key_package().expect("kp_b1");
    let kp_b2 = b.marshalled_key_package().expect("kp_b2");
    let add_b1 = es.propose_add(0, &kp_b1).expect("add-b1");
    let add_b2 = es.propose_add(0, &kp_b2).expect("add-b2");

    let r1 = a.process_proposals(&add_b1, &[UA, UB]);
    println!("PROBE two_adds: first ok={} len={:?}", r1.is_ok(), r1.as_ref().map(Vec::len));
    let r2 = a.process_proposals(&add_b2, &[UA, UB]);
    println!(
        "PROBE two_adds: second ok={} err={:?}",
        r2.is_ok(),
        r2.as_ref().err().map(std::string::ToString::to_string)
    );
    assert!(r2.is_err(), "hypothesis 2: a second add of the same member should poison the commit");
}

#[test]
fn loser_adopts_winner_welcome() {
    let es = Dave::create_external_sender(GROUP, VER).expect("es");
    let pkg = es.marshalled_package().expect("pkg");
    let mut a = Dave::create_session(GROUP, UA, VER).expect("a");
    let mut b = Dave::create_session(GROUP, UB, VER).expect("b");
    a.set_external_sender(&pkg).expect("a es");
    b.set_external_sender(&pkg).expect("b es");
    let kp_a = a.marshalled_key_package().expect("kp_a");
    let kp_b = b.marshalled_key_package().expect("kp_b");
    let recognized = [UA, UB];
    let add_b = es.propose_add(0, &kp_b).expect("add-b -> A");
    let add_a = es.propose_add(0, &kp_a).expect("add-a -> B");
    let cw_a = a.process_proposals(&add_b, &recognized).expect("A commits add-B");
    let cw_b = b.process_proposals(&add_a, &recognized);
    println!("PROBE race: B also produced a commit ok={}", cw_b.is_ok());
    let (commit_a, welcome_a) = es.split_commit_welcome(&cw_a).expect("split A");
    println!("PROBE winner: commit.len={} welcome.len={}", commit_a.len(), welcome_a.len());
    let roster_a = a.process_commit(&commit_a).expect("A applies own commit").member_ids;
    let welcome_b = b.process_welcome(&welcome_a, &recognized);
    println!("PROBE loser process_welcome ok={}", welcome_b.is_ok());
    let mut roster_a = roster_a;
    let mut roster_b = welcome_b.expect("B adopts winner welcome").member_ids;
    roster_a.sort_unstable();
    roster_b.sort_unstable();
    let mut expected = [
        UA.parse::<u64>().unwrap(),
        UB.parse::<u64>().unwrap(),
    ];
    expected.sort_unstable();
    assert_eq!(roster_a, expected, "A roster {{A,B}}");
    assert_eq!(roster_b, expected, "B (loser) roster {{A,B}} after adopting winner welcome");
    assert_eq!(
        a.last_epoch_authenticator().expect("a auth"),
        b.last_epoch_authenticator().expect("b auth"),
        "A and B converge to the same epoch authenticator"
    );
}
