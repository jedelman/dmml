//! Confirms `dmml::genesis::GENESIS_DMML_SOURCE` -- the fixed world every
//! new player starts from (issue #78) -- stays parseable, self-declared,
//! and fireable exactly the way `examples/build_something.rs` proved by
//! hand. This is the one piece of content-not-code this crate ships, so
//! it gets a real test rather than only a manual `cargo run --example`
//! check.

use dmml::ast::TopLevelItem;
use dmml::genesis::{is_genesis_commit, GENESIS_DMML_SOURCE};
use dmml::interpret::Materialized;
use dmml::machine::{may_fire, parse_all_machines, EvalContext};
use dmml::validate::validate_declarations;
use dmml::lower;
use dmml_substrate_kit::atproto_cid as identity;

fn commit_stmt(doc: &dmml::Document) -> &dmml::ast::CommitStmt {
    doc.items
        .iter()
        .find_map(|item| match item {
            TopLevelItem::Commit(c) => Some(c),
            _ => None,
        })
        .expect("genesis source must contain a commit block")
}

#[test]
fn genesis_source_parses() {
    dmml::parse(GENESIS_DMML_SOURCE).expect("genesis source should parse");
}

#[test]
fn genesis_commit_self_declares_every_predicate() {
    let doc = dmml::parse(GENESIS_DMML_SOURCE).expect("genesis source should parse");
    let commit = commit_stmt(&doc);
    validate_declarations(commit).expect("every genesis predicate should be self-declared");
}

#[test]
fn genesis_commit_lowers_and_gets_a_cid() {
    let doc = dmml::parse(GENESIS_DMML_SOURCE).expect("genesis source should parse");
    let commit = commit_stmt(&doc);
    let lowered = lower::lower_commit(commit);
    assert!(!lowered.produces.is_empty(), "genesis should mint real content");
    let _cid = identity::compute_cid(&lowered, "2026-08-19T00:00:00Z");
}

#[test]
fn genesis_commit_is_identifiable_by_its_seed_node_marker() {
    let doc = dmml::parse(GENESIS_DMML_SOURCE).expect("genesis source should parse");
    let commit = commit_stmt(&doc);
    let lowered = lower::lower_commit(commit);
    assert!(
        is_genesis_commit(&lowered),
        "the genesis commit must mint the seed-node marker fact, since #79's Perceive route \
         identifies the root commit by this marker, not by the absence of respondsTo"
    );
}

#[test]
fn genesis_edge_starts_locked_and_may_fire_once_the_key_is_held() {
    let doc = dmml::parse(GENESIS_DMML_SOURCE).expect("genesis source should parse");
    let commit = commit_stmt(&doc);
    let lowered = lower::lower_commit(commit);
    let world = Materialized::from_commits(&[lowered]);

    let machines = parse_all_machines(&doc).expect("genesis machine body should parse");
    let ctx = EvalContext { self_node: "edge/12".to_string(), params: Default::default() };
    let result = may_fire(&machines["edge/12"], "unlock", &ctx, &world);
    assert_eq!(
        result,
        Some(true),
        "genesis should start with the door locked, the player already holding the key"
    );
}

#[test]
fn genesis_player_starts_in_room1() {
    let doc = dmml::parse(GENESIS_DMML_SOURCE).expect("genesis source should parse");
    let commit = commit_stmt(&doc);
    let lowered = lower::lower_commit(commit);
    let world = Materialized::from_commits(&[lowered]);
    assert_eq!(
        world.current_value("player", "state"),
        Some(&lower::TripleValue::Node("room1".to_string())),
        "genesis should place the player in room1, matching the narrated starting scene"
    );
}

#[test]
fn genesis_player_may_not_move_to_room2_while_the_door_is_locked() {
    let doc = dmml::parse(GENESIS_DMML_SOURCE).expect("genesis source should parse");
    let commit = commit_stmt(&doc);
    let lowered = lower::lower_commit(commit);
    let world = Materialized::from_commits(&[lowered]);

    let machines = parse_all_machines(&doc).expect("genesis machine body should parse");
    let ctx = EvalContext { self_node: "player".to_string(), params: Default::default() };
    let result = may_fire(&machines["player"], "move_to_room_2", &ctx, &world);
    assert_eq!(
        result,
        Some(false),
        "movement through edge/12 must stay gated on it actually being unlocked, not just on \
         having been in room1 -- the same guard discipline #80 already enforces for unlock itself"
    );
}

#[test]
fn genesis_player_may_move_to_room2_once_the_door_is_unlocked() {
    let doc = dmml::parse(GENESIS_DMML_SOURCE).expect("genesis source should parse");
    let commit = commit_stmt(&doc);
    let mut lowered = lower::lower_commit(commit);
    // Simulate the post-unlock world a real unlock commit would produce
    // (last-write-wins overwrites edge/12's state) without needing a
    // second real commit for this test.
    lowered.produces.push(lower::Triple {
        subject: "edge/12".to_string(),
        predicate: "state".to_string(),
        object: lower::TripleValue::Node("unlocked".to_string()),
    });
    let world = Materialized::from_commits(&[lowered]);

    let machines = parse_all_machines(&doc).expect("genesis machine body should parse");
    let ctx = EvalContext { self_node: "player".to_string(), params: Default::default() };
    let result = may_fire(&machines["player"], "move_to_room_2", &ctx, &world);
    assert_eq!(result, Some(true), "once edge/12 is unlocked, moving into room2 should be legitimate");
}
