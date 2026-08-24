//! Independent verification of `dmml::lower::lower_commit` against
//! LOWERING_SPEC.md's three worked examples -- these expected values
//! were written into the spec BEFORE the implementation was dispatched
//! to Kimi, and are transcribed here unchanged from the spec document,
//! not derived from reading the implementation. A 4th test (multiple
//! `via` clauses) checks the last-wins tie-break rule the spec states
//! but didn't work a full example for.

use dmml::ast::TopLevelItem;
use dmml::lower::{
    lower_commit, ConsumeRef, FactRef, LoweredCommit, StrongRef, Triple, TripleValue,
};

fn lower_first_commit(src: &str) -> LoweredCommit {
    let doc = dmml::parse(src).expect("should parse");
    let TopLevelItem::Commit(commit) = &doc.items[0] else {
        panic!("expected a commit");
    };
    lower_commit(commit)
}

#[test]
fn example_1_declare_then_assert_mint() {
    let src = r#"
commit mints {
  declare relation opensTo
  declare attribute dampness

  room/42 a Room
  room/42 opensTo room/43
  room/42 dampness 0.4
}
"#;
    let lowered = lower_first_commit(src);
    assert_eq!(
        lowered,
        LoweredCommit {
            predicate_verb: "mints".to_string(),
            consumes: vec![],
            produces: vec![
                Triple {
                    subject: "opensTo".to_string(),
                    predicate: "rdf:type".to_string(),
                    object: TripleValue::Node("Relation".to_string()),
                },
                Triple {
                    subject: "dampness".to_string(),
                    predicate: "rdf:type".to_string(),
                    object: TripleValue::Node("Attribute".to_string()),
                },
                Triple {
                    subject: "room/42".to_string(),
                    predicate: "rdf:type".to_string(),
                    object: TripleValue::Node("Room".to_string()),
                },
                Triple {
                    subject: "room/42".to_string(),
                    predicate: "opensTo".to_string(),
                    object: TripleValue::Node("room/43".to_string()),
                },
                Triple {
                    subject: "room/42".to_string(),
                    predicate: "dampness".to_string(),
                    object: TripleValue::Number("0.4".to_string()),
                },
            ],
            via: None,
            responds_to: None,
        }
    );
}

#[test]
fn example_2_consumes_and_produces_becomes() {
    let src = r#"
commit becomes {
  consumes {
    fact at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789
      (cid: bafyabcxyz) {
      subject: room/42
      predicate: locked
    }
  }
  produces {
    room/42 locked false
  }
}
"#;
    let lowered = lower_first_commit(src);
    assert_eq!(
        lowered,
        LoweredCommit {
            predicate_verb: "becomes".to_string(),
            consumes: vec![ConsumeRef::Fact(FactRef {
                commit: StrongRef {
                    uri: "at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789"
                        .to_string(),
                    cid: "bafyabcxyz".to_string(),
                },
                subject: "room/42".to_string(),
                predicate: "locked".to_string(),
                object: None,
            })],
            produces: vec![Triple {
                subject: "room/42".to_string(),
                predicate: "locked".to_string(),
                object: TripleValue::Boolean(false),
            }],
            via: None,
            responds_to: None,
        }
    );
}

#[test]
fn example_3_via_and_responds_to_grants() {
    let src = r#"
commit grants {
  via at://did:plc:abc/org.foo.bar/rkey1 (cid: bafyxyz1)
  respondsTo at://did:plc:def/org.foo.bar/rkey2 (cid: bafyxyz2)
}
"#;
    let lowered = lower_first_commit(src);
    assert_eq!(
        lowered,
        LoweredCommit {
            predicate_verb: "grants".to_string(),
            consumes: vec![],
            produces: vec![],
            via: Some(StrongRef {
                uri: "at://did:plc:abc/org.foo.bar/rkey1".to_string(),
                cid: "bafyxyz1".to_string(),
            }),
            responds_to: Some(StrongRef {
                uri: "at://did:plc:def/org.foo.bar/rkey2".to_string(),
                cid: "bafyxyz2".to_string(),
            }),
        }
    );
}

#[test]
fn repeated_via_last_one_wins() {
    let src = r#"
commit grants {
  via at://did:plc:aaa/org.foo.bar/first (cid: bafyfirst)
  via at://did:plc:bbb/org.foo.bar/second (cid: bafysecond)
}
"#;
    let lowered = lower_first_commit(src);
    assert_eq!(
        lowered.via,
        Some(StrongRef {
            uri: "at://did:plc:bbb/org.foo.bar/second".to_string(),
            cid: "bafysecond".to_string(),
        })
    );
}
