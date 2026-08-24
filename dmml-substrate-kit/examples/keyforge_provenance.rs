//! "Any key minted with KEYFORGE_CID opens this door" -- demonstrating
//! that DMML already expresses this, with the primitives that exist
//! today, no grammar change: mint provenance as an ordinary triple
//! (`key/N mintedBy <cid>`), then write the guard as a two-hop chain
//! off the player: `EXISTS(player holds ?key mintedBy KEYFORGE_CID)`.
//! `?key` isn't unified against anything else -- it doesn't need to be,
//! because the chain *is* the conjunction: whatever key the player
//! holds, its own `mintedBy` fact is what gets checked next, using
//! `?key`'s materialized value as the next hop's subject.
//!
//! `cargo run -p dmml --example keyforge_provenance`

use dmml::interpret::Materialized;
use dmml::machine::{may_fire, parse_all_machines, EvalContext};
use dmml::validate::validate_declarations;
use dmml::lower;
use dmml_substrate_kit::atproto_cid as identity;

fn forge_cid() -> String {
    // A stand-in "recipe" commit -- in the real game this would be
    // whatever commit actually defines the Keyforge; here it's just
    // something to compute a real CID from, so KEYFORGE_CID below is a
    // genuine CID string, not a placeholder.
    let src = "commit mints {\n  declare relation isA\n  keyforge/1 isA Recipe\n}\n";
    let doc = dmml::parse(src).expect("should parse");
    let commit_stmt = match &doc.items[0] {
        dmml::ast::TopLevelItem::Commit(c) => c,
        _ => unreachable!(),
    };
    let lowered = lower::lower_commit(commit_stmt);
    identity::compute_cid(&lowered, "2026-08-18T00:00:00Z").to_string()
}

/// Builds the door machine + a minting commit that gives the player
/// `held_key`, minted by `key_provenance_cid`. Returns whether the
/// door's `unlock` transition may fire.
fn try_unlock_with(keyforge_cid: &str, held_key: &str, key_provenance_cid: &str) -> Option<bool> {
    let src = format!(
        r#"
machine edge/12 {{
  state locked
  state unlocked

  transition unlock {{
    from: locked
    to: unlocked
    guard: EXISTS(player holds ?key mintedBy {keyforge_cid})
  }}
}}

commit mints {{
  declare attribute state
  declare relation holds
  declare relation mintedBy

  edge/12 state locked
  player holds {held_key}
  {held_key} mintedBy {key_provenance_cid}
}}
"#
    );

    let doc = dmml::parse(&src).expect("should parse");
    let commit_stmt = doc
        .items
        .iter()
        .find_map(|item| match item {
            dmml::ast::TopLevelItem::Commit(c) => Some(c),
            _ => None,
        })
        .unwrap();
    validate_declarations(commit_stmt).expect("every predicate should be self-declared");

    let lowered = lower::lower_commit(commit_stmt);
    let world = Materialized::from_commits(&[lowered]);
    let machines = parse_all_machines(&doc).expect("machine bodies should parse");

    let ctx = EvalContext {
        self_node: "edge/12".to_string(),
        params: Default::default(),
    };
    may_fire(&machines["edge/12"], "unlock", &ctx, &world)
}

fn main() {
    let keyforge = forge_cid();
    println!("KEYFORGE_CID = {keyforge}\n");

    println!("guard: EXISTS(player holds ?key mintedBy {keyforge})\n");

    // A key genuinely minted by the Keyforge -- door should open.
    let genuine = try_unlock_with(&keyforge, "key/7", &keyforge);
    println!("player holds key/7, minted by the Keyforge -> may_fire = {genuine:?}");
    assert_eq!(genuine, Some(true));

    // A key minted by something else entirely -- same guard, different
    // provenance, door should NOT open, even though the player is
    // holding *a* key.
    let impostor_cid = "bafyreiaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let forged = try_unlock_with(&keyforge, "key/9", impostor_cid);
    println!("player holds key/9, minted elsewhere      -> may_fire = {forged:?}");
    assert_eq!(forged, Some(false));

    println!("\n=== the guard checks provenance, not just possession -- confirmed both ways ===");
}
