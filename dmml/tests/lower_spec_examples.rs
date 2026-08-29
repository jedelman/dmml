//! Independent verification of `dmml::lower::lower_commit` against
//! LOWERING_SPEC.md's three worked examples -- these expected values
//! were written into the spec BEFORE the implementation was dispatched
//! to Kimi, and are transcribed here unchanged from the spec document,
//! not derived from reading the implementation. A 4th test (multiple
//! `via` clauses) checks the last-wins tie-break rule the spec states
//! but didn't work a full example for.
//!
//! The first three examples go through `from_json::commit_from_json`,
//! same as real content. The 4th (`repeated_via_last_one_wins`) builds
//! `ast::CommitStmt` by hand: JSON authoring's `CommitInput` only has a
//! single `via` field (one `via` per commit is the only shape an agent
//! can actually produce), but `lower_commit`'s last-wins behavior over
//! multiple `CommitItem::Via` entries is still real AST-level behavior
//! worth pinning directly, the same way `validate_spec_examples.rs`
//! builds its explicit-`produces`-block case straight against the AST.

use dmml::ast::{self, CommitItem, Span, StrongRef as AstStrongRef};
use dmml::from_json::commit_from_json;
use dmml::lower::{lower_commit, ConsumeRef, FactRef, LoweredCommit, StrongRef, Triple, TripleValue};

fn lower_json(json: &str) -> LoweredCommit {
    let commit = commit_from_json(json).expect("should build");
    lower_commit(&commit)
}

#[test]
fn example_1_declare_then_assert_mint() {
    let json = r#"{
        "verb": "mints",
        "declares": [
            {"kind": "relation", "name": "opensTo"},
            {"kind": "attribute", "name": "dampness"}
        ],
        "facts": [
            {"subject": "room/42", "predicate": "a", "object": {"kind": "node", "value": "Room"}},
            {"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}},
            {"subject": "room/42", "predicate": "dampness", "object": {"kind": "number", "value": "0.4"}}
        ]
    }"#;
    let lowered = lower_json(json);
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
    let json = r#"{
        "verb": "becomes",
        "facts": [{"subject": "room/42", "predicate": "locked", "object": {"kind": "boolean", "value": false}}],
        "consumes": [
            {"kind": "fact",
             "commit": {"uri": "at://did:plc:aaaa1111/org.jason-edelman.writtenworld.commit/xyz789", "cid": "bafyabcxyz"},
             "subject": "room/42",
             "predicate": "locked"}
        ]
    }"#;
    let lowered = lower_json(json);
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
    let json = r#"{
        "verb": "grants",
        "via": {"uri": "at://did:plc:abc/org.foo.bar/rkey1", "cid": "bafyxyz1"},
        "respondsTo": {"uri": "at://did:plc:def/org.foo.bar/rkey2", "cid": "bafyxyz2"}
    }"#;
    let lowered = lower_json(json);
    assert_eq!(lowered.predicate_verb, "grants");
    assert_eq!(lowered.produces, vec![]);
    assert_eq!(
        lowered.via,
        Some(StrongRef {
            uri: "at://did:plc:abc/org.foo.bar/rkey1".to_string(),
            cid: "bafyxyz1".to_string(),
        })
    );
    assert_eq!(
        lowered.responds_to,
        Some(StrongRef {
            uri: "at://did:plc:def/org.foo.bar/rkey2".to_string(),
            cid: "bafyxyz2".to_string(),
        })
    );
}

#[test]
fn repeated_via_last_one_wins() {
    fn via(uri: &str, cid: &str) -> CommitItem {
        CommitItem::Via(AstStrongRef {
            uri: ast::AtUri {
                raw: uri.to_string(),
                did: uri.split('/').nth(2).unwrap().to_string(),
                nsid: uri.split('/').nth(3).unwrap().to_string(),
                rkey: uri.split('/').nth(4).unwrap().to_string(),
            },
            cid: cid.to_string(),
            span: Span::new(""),
        })
    }

    let commit = ast::CommitStmt {
        predicate_verb: "grants".to_string(),
        items: vec![
            via("at://did:plc:aaa/org.foo.bar/first", "bafyfirst"),
            via("at://did:plc:bbb/org.foo.bar/second", "bafysecond"),
        ],
        span: Span::new(""),
    };

    let lowered = lower_commit(&commit);
    assert_eq!(
        lowered.via,
        Some(StrongRef {
            uri: "at://did:plc:bbb/org.foo.bar/second".to_string(),
            cid: "bafysecond".to_string(),
        })
    );
}
