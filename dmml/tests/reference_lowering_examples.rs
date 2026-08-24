//! Independent verification of `dmml::lower::lower_reference` against
//! LOWERING_SPEC.md's "Reference statement lowering" section -- both
//! worked examples, plus the not-fully-worked ordering case (multi-
//! segment subject, foreignUri strictly before foreignCid).

use dmml::ast::TopLevelItem;
use dmml::lower::{lower_reference, Triple, TripleValue};

fn lower_first_reference(src: &str) -> Vec<Triple> {
    let doc = dmml::parse(src).expect("should parse");
    let TopLevelItem::Reference(r) = &doc.items[0] else {
        panic!("expected a reference statement");
    };
    lower_reference(r)
}

#[test]
fn example_1_as_given_two_triples_in_order() {
    let src = "reference at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456
  (cid: bafyqrs456) as room/42.reach";
    let triples = lower_first_reference(src);
    assert_eq!(
        triples,
        vec![
            Triple {
                subject: "room/42.reach".to_string(),
                predicate: "foreignUri".to_string(),
                object: TripleValue::Str(
                    "at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456"
                        .to_string()
                ),
            },
            Triple {
                subject: "room/42.reach".to_string(),
                predicate: "foreignCid".to_string(),
                object: TripleValue::Str("bafyqrs456".to_string()),
            },
        ]
    );
}

#[test]
fn example_2_as_omitted_empty_result() {
    let src = "reference at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456
  (cid: bafyqrs456)";
    let triples = lower_first_reference(src);
    assert_eq!(triples, vec![]);
}

/// Not a worked example in the spec -- only stated as rule 2's "in this
/// order" language, and only demonstrated with a dotted single-node
/// subject before. Tests a genuinely multi-SEGMENT `as`-name (segments
/// joined by `/`, not a dotted single segment) and checks foreignUri
/// still comes strictly before foreignCid.
#[test]
fn multi_segment_as_name_shares_subject_uri_before_cid() {
    let src = "reference at://did:plc:abc/org.foo.bar/rkey1 (cid: bafyxyz1) as key/7/label";
    let triples = lower_first_reference(src);
    assert_eq!(triples.len(), 2);
    assert_eq!(triples[0].subject, "key/7/label");
    assert_eq!(triples[1].subject, "key/7/label");
    assert_eq!(triples[0].predicate, "foreignUri");
    assert_eq!(triples[1].predicate, "foreignCid");
}
