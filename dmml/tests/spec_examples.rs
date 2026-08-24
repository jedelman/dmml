//! Parses the actual example programs from `SPEC.md` section 10 ("Surface
//! syntax"), verbatim, and checks the resulting AST shape. If these stop
//! parsing, either the parser or the grammar itself has drifted from what
//! the spec actually documents.

use dmml::ast::*;

#[test]
fn declare_then_assert_mint() {
    let src = r#"
commit mints {
  declare relation opensTo
  declare attribute dampness

  room/42 a Room
  room/42 opensTo room/43
  room/42 dampness 0.4
}
"#;
    let doc = dmml::parse(src).expect("should parse");
    assert_eq!(doc.items.len(), 1);
    let TopLevelItem::Commit(commit) = &doc.items[0] else {
        panic!("expected a commit");
    };
    assert_eq!(commit.predicate_verb, "mints");
    assert_eq!(commit.items.len(), 5);

    assert!(matches!(
        &commit.items[0],
        CommitItem::Declare(DeclareStmt { kind: DeclKind::Relation, ident, .. }) if ident == "opensTo"
    ));
    assert!(matches!(
        &commit.items[1],
        CommitItem::Declare(DeclareStmt { kind: DeclKind::Attribute, ident, .. }) if ident == "dampness"
    ));

    let CommitItem::Fact(f) = &commit.items[2] else {
        panic!("expected a fact");
    };
    assert_eq!(f.subject.segments, vec!["room", "42"]);
    assert!(matches!(f.predicate, PredicateRef::RdfType));
    assert!(matches!(&f.value, Value::Node(n) if n.segments == vec!["Room".to_string()]));

    let CommitItem::Fact(f) = &commit.items[3] else {
        panic!("expected a fact");
    };
    assert!(matches!(&f.predicate, PredicateRef::Ident(p) if p == "opensTo"));
    assert!(
        matches!(&f.value, Value::Node(n) if n.segments == vec!["room".to_string(), "43".to_string()])
    );

    let CommitItem::Fact(f) = &commit.items[4] else {
        panic!("expected a fact");
    };
    assert!(matches!(&f.predicate, PredicateRef::Ident(p) if p == "dampness"));
    assert!(matches!(&f.value, Value::Literal(Literal::Number(n)) if n == "0.4"));
}

#[test]
fn reserved_machine_block_is_opaque() {
    let src = r#"
machine edge/12 {
  state locked
  state unlocked
  transition unlock {
    from: locked
    to: unlocked
    guard: holds(player, key/7)
    effect: retract locked, assert unlocked
  }
}
"#;
    let doc = dmml::parse(src).expect("should parse");
    assert_eq!(doc.items.len(), 1);
    let TopLevelItem::Machine(m) = &doc.items[0] else {
        panic!("expected a machine stmt");
    };
    assert_eq!(m.node.segments, vec!["edge", "12"]);
    assert!(m.body.contains("guard: holds(player, key/7)"));
}

#[test]
fn consumes_and_reference_are_visibly_distinct() {
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

reference at://did:plc:zzzz9999/org.jason-edelman.writtenworld.commit/qrs456
  (cid: bafyqrs456) as room/42.reach
"#;
    let doc = dmml::parse(src).expect("should parse");
    assert_eq!(doc.items.len(), 2);

    let TopLevelItem::Commit(commit) = &doc.items[0] else {
        panic!("expected a commit");
    };
    assert_eq!(commit.predicate_verb, "becomes");
    assert_eq!(commit.items.len(), 2);

    let CommitItem::Consumes(cb) = &commit.items[0] else {
        panic!("expected a consumes block");
    };
    assert_eq!(cb.entries.len(), 1);
    let ConsumeEntry::Fact(fc) = &cb.entries[0] else {
        panic!("expected a fact consume");
    };
    assert_eq!(fc.commit.uri.did, "did:plc:aaaa1111");
    assert_eq!(fc.commit.uri.nsid, "org.jason-edelman.writtenworld.commit");
    assert_eq!(fc.commit.uri.rkey, "xyz789");
    assert_eq!(fc.commit.cid, "bafyabcxyz");
    assert_eq!(fc.subject.segments, vec!["room", "42"]);
    assert_eq!(fc.predicate, "locked");
    assert!(fc.object.is_none());

    let CommitItem::Produces(pb) = &commit.items[1] else {
        panic!("expected a produces block");
    };
    assert_eq!(pb.facts.len(), 1);
    assert!(matches!(
        &pb.facts[0].value,
        Value::Literal(Literal::Boolean(false))
    ));

    let TopLevelItem::Reference(r) = &doc.items[1] else {
        panic!("expected a reference stmt");
    };
    assert_eq!(r.target.uri.rkey, "qrs456");
    assert_eq!(r.target.cid, "bafyqrs456");
    assert_eq!(
        r.as_name.as_ref().unwrap().segments,
        vec!["room".to_string(), "42.reach".to_string()]
    );
}

#[test]
fn empty_document_parses_ok() {
    let doc = dmml::parse("").expect("empty input should parse");
    assert!(doc.items.is_empty());

    let doc =
        dmml::parse("   \n // just a comment\n  ").expect("whitespace/comment-only should parse");
    assert!(doc.items.is_empty());
}

#[test]
fn via_and_responds_to_are_parsed() {
    let src = r#"
commit grants {
  via at://did:plc:abc/org.foo.bar/rkey1 (cid: bafyxyz1)
  respondsTo at://did:plc:def/org.foo.bar/rkey2 (cid: bafyxyz2)
}
"#;
    let doc = dmml::parse(src).expect("should parse");
    let TopLevelItem::Commit(commit) = &doc.items[0] else {
        panic!("expected a commit");
    };
    assert!(matches!(&commit.items[0], CommitItem::Via(_)));
    assert!(matches!(&commit.items[1], CommitItem::RespondsTo(_)));
}

#[test]
fn malformed_input_is_a_parse_error_not_a_panic() {
    for bad in [
        "commit",
        "commit mints",
        "commit mints {",
        "commit mints { room/42 }",
        "reference",
        "machine",
        "machine node/1",
        "machine node/1 {",
        "{}",
        "commit mints { via at://bad-uri (cid: x) }",
        "commit mints { consumes { fact at://a/b/c (cid: x) { subject: n predicate } } }",
    ] {
        let result = dmml::parse(bad);
        assert!(
            result.is_err(),
            "expected {bad:?} to fail to parse, got {result:?}"
        );
    }
}
