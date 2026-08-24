//! Independent verification of `dmml::identity`'s triple-identity
//! functions against TRIPLE_IDENTITY_SPEC.md's five worked examples,
//! plus the one case stated only as rule 4's "both must hold" language
//! and never given a full worked example (right owner, wrong content).

use dmml_substrate_kit::atproto_cid::{make_triple_ref, triple_cid, triple_ref_matches};
use dmml::lower::{Triple, TripleValue};

fn t1() -> Triple {
    Triple {
        subject: "room/42".to_string(),
        predicate: "locked".to_string(),
        object: TripleValue::Boolean(true),
    }
}

fn t2() -> Triple {
    Triple {
        subject: "room/42".to_string(),
        predicate: "locked".to_string(),
        object: TripleValue::Boolean(false),
    }
}

const DID_A: &str = "did:plc:aaaa1111";
const DID_B: &str = "did:plc:bbbb2222";

#[test]
fn example_1_deterministic() {
    assert_eq!(triple_cid(&t1()), triple_cid(&t1()));
}

#[test]
fn example_2_different_object_different_cid() {
    assert_ne!(triple_cid(&t1()), triple_cid(&t2()));
}

#[test]
fn example_3_same_content_different_owner_same_triple_cid_different_ref() {
    let ref_a = make_triple_ref(DID_A, &t1());
    let ref_b = make_triple_ref(DID_B, &t1());
    assert_eq!(ref_a.triple, ref_b.triple);
    assert_ne!(ref_a, ref_b);
}

#[test]
fn example_4_matches_when_owner_and_content_both_correct() {
    let reference = make_triple_ref(DID_A, &t1());
    assert!(triple_ref_matches(&reference, DID_A, &t1()));
}

#[test]
fn example_5_fails_on_wrong_owner_same_content() {
    let reference = make_triple_ref(DID_A, &t1());
    assert!(!triple_ref_matches(&reference, DID_B, &t1()));
}

/// Not a worked example in the spec -- only stated as rule 4's "both must
/// hold" language. Tests whether the implementation actually checks
/// content, not just owner_did.
#[test]
fn fails_on_right_owner_wrong_content() {
    let reference = make_triple_ref(DID_A, &t1());
    assert!(!triple_ref_matches(&reference, DID_A, &t2()));
}
