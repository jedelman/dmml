//! `dmml::interpret::reachable_from` -- the world-scoping fix for the
//! real gap `client/examples/real_pds_loop.rs` found running against a
//! live PDS: `listRecords` returns every record a DID ever wrote to a
//! collection, so a caller has to scope "this world" itself by walking
//! `respondsTo` back to a known root.

use dmml::ast::TopLevelItem;
use dmml::interpret::{reachable_from, IdentifiedCommit};
use dmml::validate::validate_declarations;
use dmml::lower;
use dmml_substrate_kit::atproto_cid as identity;

fn lower_one(src: &str) -> dmml::lower::LoweredCommit {
    let doc = dmml::parse(src).expect("should parse");
    let commit = doc
        .items
        .iter()
        .find_map(|item| match item {
            TopLevelItem::Commit(c) => Some(c),
            _ => None,
        })
        .expect("source should contain exactly one commit");
    validate_declarations(commit).expect("every predicate should self-declare");
    lower::lower_commit(commit)
}

fn identify(commit: dmml::lower::LoweredCommit, uri: &str) -> IdentifiedCommit {
    let cid = identity::compute_cid(&commit, "2026-08-18T00:00:00Z").to_string();
    IdentifiedCommit { uri: uri.to_string(), cid, commit }
}

#[test]
fn reachable_from_walks_responds_to_and_excludes_unrelated_records() {
    let root = identify(lower_one("commit mints {\n  declare relation isA\n  scene/1 isA Location\n}\n"), "at://did:example:test/coll/root");

    let child_src = format!(
        "commit asks {{\n  declare attribute state\n  respondsTo at://did:example:test/coll/root(cid: {})\n  question/1 state open\n}}\n",
        root.cid
    );
    let child = identify(lower_one(&child_src), "at://did:example:test/coll/child");

    let grandchild_src = format!(
        "commit fires {{\n  declare attribute state\n  respondsTo at://did:example:test/coll/child(cid: {})\n  question/1 state answered\n}}\n",
        child.cid
    );
    let grandchild = identify(lower_one(&grandchild_src), "at://did:example:test/coll/grandchild");

    // An unrelated record on the SAME collection/DID -- e.g. leftover
    // content from an earlier, unrelated session -- with no respondsTo
    // chain back to `root` at all. This is exactly what the real PDS
    // loop test's live interpreter incorrectly picked up.
    let unrelated = identify(
        lower_one("commit mints {\n  declare relation isA\n  room/99 isA Chamber\n}\n"),
        "at://did:example:test/coll/unrelated",
    );

    let all = vec![root.clone(), child.clone(), grandchild.clone(), unrelated.clone()];
    let scoped = reachable_from(&all, &root.cid);

    let scoped_uris: std::collections::BTreeSet<&str> = scoped.iter().map(|c| c.uri.as_str()).collect();
    assert_eq!(
        scoped_uris,
        std::collections::BTreeSet::from([root.uri.as_str(), child.uri.as_str(), grandchild.uri.as_str()]),
        "should include the root and every commit reachable by walking respondsTo back to it, \
         excluding a record with no chain back to root at all"
    );
    assert!(!scoped_uris.contains(unrelated.uri.as_str()), "unrelated record must be excluded");
}

#[test]
fn reachable_from_a_root_with_no_children_returns_just_the_root() {
    let root = identify(lower_one("commit mints {\n  declare relation isA\n  scene/1 isA Location\n}\n"), "at://did:example:test/coll/root");
    let unrelated = identify(
        lower_one("commit mints {\n  declare relation isA\n  room/99 isA Chamber\n}\n"),
        "at://did:example:test/coll/unrelated",
    );
    let all = vec![root.clone(), unrelated];
    let scoped = reachable_from(&all, &root.cid);
    assert_eq!(scoped.len(), 1);
    assert_eq!(scoped[0].uri, root.uri);
}
