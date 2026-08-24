//! The fixed, hand-authored genesis world every new player starts from.
//!
//! Per `SPEC.md` section 9 and issue #71's "no RNG, anywhere" correction:
//! one door/room/key scenario, identical for every player -- not
//! generated, not varied per-DID. This is the exact content
//! `examples/build_something.rs` already proved parses, validates,
//! lowers, materializes, and fires its own transition; `write_genesis_
//! commit` (`server/src/atproto/genesis.rs`) writes it verbatim as a new
//! player's first commit, root of their `reachable_from` chain.

/// Two `machine` blocks (`edge/12`, a locked door; `player`, where the
/// player currently is) and a `commit` block (mints the door, the room
/// it opens to, the key that unlocks it, and the player's own starting
/// location) -- all top-level items in one document, so the written
/// record's `dmml` field carries both machine declarations, not just
/// the commit.
///
/// **`player` is a real declared machine, not a plain fact.** Location
/// reuses `state` -- the one predicate `commit_fires_transition`'s own
/// effect-checking understands (`machine.rs`'s `Effect::Assert` arm) --
/// the same way `edge/12` does, so moving between rooms goes through
/// the identical guard-and-retraction machinery #80 already built for
/// unlocking a door: `check_and_synthesize_transition_consumes` finds
/// `player` in `parse_all_machines`'s map exactly like it finds
/// `edge/12`, no new code needed, only new content. The one declared
/// transition mirrors the one `opensTo` fact genesis actually asserts
/// (`room/1 opensTo room/2`) -- gated on `edge/12` actually being
/// unlocked, not just on having been in `room/1`, so a commit claiming
/// to move through a locked door fails the guard check the same way an
/// unearned unlock would.
///
/// **State names are bare symbols, not node references** (`machine.rs`'s
/// `parse_state_decl` takes a plain identifier, the same lexical class
/// `edge/12`'s own `locked`/`unlocked` use -- confirmed the hard way, a
/// first draft tried `state room/1` and the parser correctly rejected
/// it: no `/` in a state name). `player`'s two states, `room1`/`room2`,
/// are the room nodes' own slugs with the slash stripped -- a real
/// `room/1` (the minted node `opensTo` references) and the symbolic
/// state `room1` (what `player`'s machine transitions between) are two
/// different identifiers that happen to correspond 1:1 by convention,
/// not the same thing under two names.
///
/// **v1 scope, deliberately**: only genesis's own fixed room/1↔room/2
/// layout is machine-governed. A room `write_dmml_commit`/Act mints at
/// runtime (the real transcript's own `room/3`, e.g.) has no transition
/// leading to it -- machines are still parsed from genesis's own fixed
/// text only (`check_and_synthesize_transition_consumes`'s own doc
/// comment), so movement into freshly-authored space isn't guard-checked
/// yet. Not a regression: nothing machine-governed existed for movement
/// at all before this.
///
/// Also mints the fixed seed-node marker fact (`SEED_NODE`/
/// `SEED_PREDICATE`, see `is_genesis_commit`) -- issue #79's own flagged
/// open question ("how is the root/genesis commit identified among a
/// player's fetched records?") is resolved by this explicit marker, not
/// by a commit's absence of `respondsTo` (true of every standalone
/// Transform, not just genesis -- see `commit.json`'s own `respondsTo`
/// description).
pub const GENESIS_DMML_SOURCE: &str = r#"
machine edge/12 {
  state locked
  state unlocked

  transition unlock {
    from: locked
    to: unlocked
    guard: EXISTS(player holds key/7)
  }
}

machine player {
  state room1
  state room2

  transition move_to_room_2 {
    from: room1
    to: room2
    guard: EXISTS(edge/12 state unlocked)
  }
}

commit mints {
  declare relation opensTo
  declare attribute state
  declare relation holds
  declare attribute isWorldSeed

  world/seed isWorldSeed true
  room/1 opensTo room/2
  edge/12 state locked
  player holds key/7
  player state room1
}
"#;

/// The fixed subject/predicate a genesis commit's `produces` always
/// carries: `(SEED_NODE, SEED_PREDICATE, true)`. Together they're an
/// explicit marker, not an inference -- see `is_genesis_commit`.
pub const SEED_NODE: &str = "world/seed";
pub const SEED_PREDICATE: &str = "isWorldSeed";

/// True if `commit`'s `produces` mints the seed-node marker fact --
/// i.e. `commit` is a genesis/world-root commit. The Perceive route
/// (#79) uses this to pick `reachable_from`'s `root_cid` out of a
/// player's fetched records, instead of guessing from the absence of
/// `respondsTo` (ambiguous: every standalone Transform lacks one too,
/// not just genesis).
pub fn is_genesis_commit(commit: &crate::lower::LoweredCommit) -> bool {
    commit.produces.iter().any(|t| {
        t.subject == SEED_NODE
            && t.predicate == SEED_PREDICATE
            && t.object == crate::lower::TripleValue::Boolean(true)
    })
}
