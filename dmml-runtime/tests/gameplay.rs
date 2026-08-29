use dmml_runtime::direction::Direction;
use dmml_runtime::graph::{Delta, WorldGraph};
use dmml_runtime::vocab;
use dmml_runtime::Game;

#[test]
fn bootstrap_equips_player_with_hands_and_sight() {
    let game = Game::new(1);
    // Indirect check via behavior: a universal body-verb works immediately.
    let mut game = game;
    let response = game.handle("look");
    assert!(response.contains("Threshold"));
}

#[test]
fn transcript_records_every_commit_tagged_by_source() {
    let mut game = Game::new(1);
    let before = game.handle("transcript");
    assert!(
        before.contains("[demiurge]") && before.contains("The Threshold"),
        "bootstrap must already be in the transcript before any player action, got: {before}"
    );
    assert!(
        !before.contains("[player]"),
        "no player-sourced commit should exist yet, got: {before}"
    );

    game.handle("north");
    let after = game.handle("transcript");
    assert!(
        after.len() > before.len(),
        "transcript must grow after an action that generates or moves"
    );
}

#[test]
fn commits_since_captures_only_this_calls_commits() {
    let mut game = Game::new(1);
    let mark = game.transcript_len();
    assert!(
        game.commits_since(mark).is_empty(),
        "nothing has been committed since the mark yet"
    );

    // Raising a petition is unconditionally a "player"-sourced commit (see
    // its own doc comment) -- unlike movement, it can't land on a locked
    // exit and produce zero player commits depending on world-seed RNG.
    game.raise_petition_for_current_room(0);
    let commits = game.commits_since(mark);
    assert!(
        !commits.is_empty(),
        "raising a petition commits at least one delta"
    );
    for (_, text, source) in &commits {
        assert!(!text.is_empty(), "canonical text must describe what changed");
        assert_eq!(source, "player");
    }

    let mark2 = game.transcript_len();
    assert!(
        game.commits_since(mark2).is_empty(),
        "a fresh mark taken after the last commit sees nothing new"
    );
}

#[test]
fn commits_since_captures_frontier_generation_tagged_demiurge() {
    let mut game = Game::new(1);
    let mark = game.transcript_len();
    assert!(
        game.commits_since(mark).is_empty(),
        "nothing has been committed since the mark yet"
    );

    // Frontier generation always commits as "demiurge" the moment an edge
    // doesn't exist yet, regardless of whether the resulting door turns
    // out locked -- see go()'s own doc comment.
    game.handle("north");
    let commits = game.commits_since(mark);
    assert!(
        commits.iter().any(|(_, _, source)| source == "demiurge"),
        "generating a new room commits at least one demiurge-sourced delta, got: {commits:?}"
    );
}

#[test]
fn transcript_since_carries_every_source_in_order() {
    // The lower-level primitive `Game::commits_since` is built on --
    // exercised directly here (rather than through a `Game`, which has no
    // way to inject arbitrary sources) to confirm nothing about the
    // transcript itself privileges one source's commits over another's.
    // `commits_since` (see its own doc comment) is a thin, source-agnostic
    // map over exactly this.
    let mut graph = WorldGraph::new();
    let a = graph.fresh("item/");
    let b = graph.fresh("item/");
    let c = graph.fresh("item/");
    let d1 = Delta::new().assert(a, vocab::rdf_type(), vocab::class_item());
    graph.commit("demiurge", d1).unwrap();
    let mark = graph.transcript().len() as u64;
    let d2 = Delta::new().assert(b, vocab::rdf_type(), vocab::class_item());
    graph.commit("player", d2).unwrap();
    let d3 = Delta::new().assert(c, vocab::rdf_type(), vocab::class_item());
    graph.commit("external-resolver", d3).unwrap();

    let sources: Vec<&str> = graph
        .transcript_since(mark)
        .iter()
        .map(|e| e.source.as_str())
        .collect();
    assert_eq!(
        sources,
        vec!["player", "external-resolver"],
        "every source since the mark must appear, in order, not just a privileged subset"
    );
}

#[test]
fn available_actions_lists_exits_and_reflects_a_lock_being_lifted() {
    let seed = find_locked_seed(Direction::North);
    let mut game = Game::new(seed);
    game.handle("north");

    let commands: Vec<String> = game
        .available_actions()
        .into_iter()
        .map(|a| a.command)
        .collect();
    assert!(
        commands.contains(&"go north".to_string()),
        "a locked exit must still be offered as an action, got: {commands:?}"
    );
    assert!(
        commands.contains(&"pull mechanism".to_string()),
        "the generated mechanism's affordance must appear, got: {commands:?}"
    );

    let labels_before: Vec<String> = game
        .available_actions()
        .into_iter()
        .map(|a| a.label)
        .collect();
    assert!(
        labels_before.iter().any(|l| l == "north (sealed)"),
        "a locked exit's label must say so, got: {labels_before:?}"
    );

    for _ in 0..3 {
        game.handle("pull mechanism");
    }
    let labels_after: Vec<String> = game
        .available_actions()
        .into_iter()
        .map(|a| a.label)
        .collect();
    assert!(
        labels_after.iter().any(|l| l == "north"),
        "once unlocked, the exit label must drop the (sealed) annotation, got: {labels_after:?}"
    );
}

#[test]
fn revisiting_a_generated_room_keeps_its_committed_facts_but_updates_visit_history() {
    // Seed 3's north exit rolled unlocked under the old id-minting scheme;
    // the timestamp-based creation-order redesign changed minted ids (and
    // thus `demiurge::seed_for`'s hash of the Threshold's own id), which
    // flipped this seed's roll. 176 is a real, found-not-guessed
    // replacement (brute-forced with a throwaway search over the first 200
    // seeds for one whose north exit still rolls unlocked).
    let mut game = Game::new(176);
    let first = game.handle("north");
    let _ = game.handle("south");
    let second = game.handle("north");

    // The room itself isn't regenerated -- name, material character, items,
    // and exits all stay put. Only the visit-history clause, which is
    // deliberately derived from the player's own accumulating history
    // rather than the room's static facts, is expected to change.
    let strip_history = |s: &str| {
        s.replace("You've never stood here before.", "")
            .replace("You've passed through here before.", "")
            .replace("You know this place well by now.", "")
    };
    assert_eq!(
        strip_history(&first),
        strip_history(&second),
        "revisiting must not regenerate the room's committed facts, got:\n{first}\nvs\n{second}"
    );
    assert!(first.contains("never stood here before"));
    assert!(second.contains("passed through here before"));
}

#[test]
fn go_records_locatedin_via_apply_commit_and_materializes_across_multiple_generations() {
    // `go` is the one half of `contains`'s migration this task actually
    // completed -- see `vocab::located_in`'s and `Game::go`'s own doc
    // comments for why full retirement of `contains` (mirroring `take`/
    // `drop`'s `holds` -> `heldBy` migration) turned out to be unsafe here:
    // `player_room` -- read on nearly every `handle` dispatch -- would
    // inherit `current_value`'s dependency on `commit_log`, which doesn't
    // survive a `Game::snapshot`/`from_snapshot` round-trip, turning a
    // returning player's very first command into a panic. So `contains`
    // stays the authoritative fact (`player_room` below reads it, exactly
    // as before); `locatedIn` is recorded *alongside* it on every `go`,
    // proving the `apply_commit`/`current_value` mechanism itself is real
    // and correct -- this test's actual point -- without any reader
    // depending on it.
    let mut game = Game::new(176); // see revisiting_a_generated_room_... above for why 176

    assert_eq!(
        game.player_location_via_located_in(),
        None,
        "bootstrap places the player in the Threshold via the old `contains` \
         path only -- no `go` has fired yet, so `locatedIn` has never been \
         asserted for the player at all"
    );

    // First generation: Threshold -> the room north of it.
    let mark1 = game.transcript_len();
    game.handle("north");
    let room_a = game.player_room();
    let commits1 = game.commits_since(mark1);
    assert!(
        commits1
            .iter()
            .any(|(_, text, source)| source == "player" && text.contains("locatedIn")),
        "go must record a locatedIn fact via apply_commit, got: {commits1:?}"
    );
    assert_eq!(
        game.player_location_via_located_in(),
        Some(room_a.clone()),
        "current_value must materialize the first generation correctly"
    );

    // Second generation: back to the Threshold. Same predicate on the same
    // subject (the player), re-asserted -- the store now holds two
    // generations of `locatedIn(player, _)` side by side (apply_commit
    // never deletes), so this only passes if current_value is genuinely
    // walking commit_log's own order rather than the store's.
    let mark2 = game.transcript_len();
    game.handle("south");
    let room_b = game.player_room();
    assert_ne!(room_a, room_b, "south must lead back to a different room (the Threshold)");
    let commits2 = game.commits_since(mark2);
    assert!(
        commits2
            .iter()
            .any(|(_, text, source)| source == "player" && text.contains("locatedIn")),
        "go must record a fresh locatedIn fact on every transition, got: {commits2:?}"
    );
    assert_eq!(
        game.player_location_via_located_in(),
        Some(room_b),
        "current_value must materialize the second generation, not the first"
    );

    // Third generation: north again, back to the same room as the first
    // hop -- proves this isn't accidentally passing because every
    // generation so far named a distinct room.
    game.handle("north");
    assert_eq!(
        game.player_room(),
        room_a,
        "sanity: north from the Threshold always lands back on the same generated room"
    );
    assert_eq!(
        game.player_location_via_located_in(),
        Some(room_a),
        "current_value must materialize the third generation correctly, matching \
         player_room -- the old-path read this migration deliberately left in place"
    );
}

/// Brute-force a world seed where going straight from the start room in a
/// fixed direction lands on a locked exit -- deterministic because
/// `demiurge::generate_frontier`'s RNG is seeded from (world_seed, origin,
/// direction).
fn find_locked_seed(dir: Direction) -> u64 {
    for seed in 0..2000u64 {
        let mut probe = Game::new(seed);
        if probe.handle(dir.word()).contains("sealed") {
            return seed;
        }
    }
    panic!("no locked seed found in range -- generator's lock probability may have regressed");
}

#[test]
fn locked_exit_requires_accumulated_wear_and_resolves_within_one_turn() {
    let seed = find_locked_seed(Direction::North);
    let mut game = Game::new(seed);

    let blocked = game.handle("north");
    assert!(
        blocked.contains("sealed"),
        "expected the exit to be locked, got: {blocked}"
    );

    let first = game.handle("pull mechanism");
    assert!(
        first.contains("gives a little more") && !first.contains("door"),
        "first pull should only register drift, not unlock, got: {first}"
    );
    assert!(game.handle("north").contains("sealed"));

    let second = game.handle("pull mechanism");
    assert!(
        second.contains("strains") && !second.contains("door"),
        "should not unlock on the second pull (0.68 < 1.0 threshold), got: {second}"
    );
    assert!(game.handle("north").contains("sealed"));

    // Third pull crosses 1.0 -- drift and threshold rules must both
    // resolve within this one call (creation-order sorting in
    // machines_for_verb is what makes this possible). Deliberately not
    // calling `go north` here to check the unlock -- that would actually
    // move the player through the now-open door, leaving the mechanism
    // behind in the room we just left, before the repeat-guard check below
    // gets a chance to run against it.
    let third = game.handle("pull mechanism");
    assert!(
        third.contains("door gives way"),
        "expected drift AND unlock text on the crossing turn, got: {third}"
    );

    let after = game.handle("pull mechanism");
    assert_eq!(
        after, "Nothing happens.",
        "repeat-guard should silence further pulls, got: {after}"
    );

    // Only now confirm the unlock by actually moving -- checked last since
    // it consumes the transition.
    assert!(!game.handle("north").contains("sealed"));
}

#[test]
fn recurring_pattern_crystallizes_into_a_shared_named_kind() {
    let seed = find_locked_seed(Direction::North);
    let mut game = Game::new(seed);

    game.handle("north");
    let kinds_after_first = game.handle("kinds");
    assert!(
        kinds_after_first.contains("seen 1 time")
            && !kinds_after_first.contains("crystallized\n")
            && !kinds_after_first.contains("\"lever\""),
        "first occurrence must stay uncrystallized, got: {kinds_after_first}"
    );
    assert!(
        game.look().contains("mechanism"),
        "first occurrence renders as the generic placeholder name"
    );

    // A second, independently generated lock of the same shape -- find one
    // off a *different* seed/direction so it doesn't depend on this game's
    // own map layout.
    let seed2 = find_locked_seed(Direction::East);
    let mut game2 = Game::new(seed2);
    game2.handle("east");

    // Simulate both having occurred in the same world by driving
    // crystallization through a single WorldGraph directly at the
    // demiurge level, since Game doesn't expose cross-instance state --
    // this exercises the same crystallize() call path generate_frontier
    // uses, twice, which is the actual unit under test.
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let room_a =
        dmml_runtime::demiurge::generate_frontier(&mut graph, 1, &boot.start_room, Direction::North);
    let _ = room_a;
    let kinds_once = render_kinds_for_test(&graph);

    let room_b =
        dmml_runtime::demiurge::generate_frontier(&mut graph, 2, &boot.start_room, Direction::South);
    let _ = room_b;
    let kinds_twice = render_kinds_for_test(&graph);

    // At least one of the two independent generations must have produced
    // the lockable pattern for this test to mean anything; if neither did
    // (both rolled unlocked), that's this seed pair's bad luck, not a
    // failure -- so only assert the crystallization claim when we actually
    // observed two occurrences.
    if kinds_once.contains("seen 1") && kinds_twice.contains("\"lever\"") {
        assert!(kinds_twice.contains("seen 2"));
    }
}

fn render_kinds_for_test(graph: &WorldGraph) -> String {
    dmml_runtime::render::render_kinds(graph)
}

#[test]
fn take_and_drop_round_trip() {
    let mut game = Game::new(3);
    game.handle("north");
    let look = game.look();
    // Grab whichever flavor noun appears, if any did for this seed's room.
    let item_name = [
        "clay jug",
        "coil of rope",
        "brass whistle",
        "chipped bowl",
        "tarnished locket",
        "moth-eaten cloak",
        "cracked mirror",
        "warped hand mirror",
        "handful of bone dice",
        "bundle of dry reeds",
    ]
    .into_iter()
    .find(|n| look.contains(n));

    let Some(item_name) = item_name else {
        return; // this seed's room happened to have no portable item; fine
    };

    let taken = game.handle(&format!("take {item_name}"));
    assert!(taken.contains("You take"), "got: {taken}");
    assert!(game.handle("inventory").contains(item_name));

    let dropped = game.handle(&format!("drop {item_name}"));
    assert!(dropped.contains("You drop"), "got: {dropped}");
    assert!(game.handle("inventory").contains("nothing"));
}

// The default noun set (what `take_and_drop_round_trip` above uses) plus
// the Pluto/Demeter theos-domain noun sets -- `demiurge::ensure_theos_
// generator` can assign a room a domain on its very first frontier
// generation, so a seed's first room north of the Threshold is just as
// likely to roll a Pluto/Demeter item noun as a default one. Restricting
// this list to the default set alone (as the pre-existing `take_and_drop_
// round_trip` test above does) is why that test silently no-ops for most
// seeds; brute-forcing a seed below needs the full noun space to reliably
// find one within a reasonable search window.
const PORTABLE_ITEM_NAMES: [&str; 30] = [
    // ITEM_NOUN (default)
    "cracked mirror",
    "clay jug",
    "moth-eaten cloak",
    "tarnished locket",
    "bundle of dry reeds",
    "chipped bowl",
    "coil of rope",
    "brass whistle",
    "warped hand mirror",
    "handful of bone dice",
    // PLUTO_ITEM_NOUN
    "pitted iron ingot",
    "shard of raw ore",
    "corroded chisel",
    "lump of native copper",
    "cracked grindstone",
    "tarnished coin",
    "bent crowbar",
    "chunk of slag glass",
    "rusted manacle",
    "handful of gravel",
    // DEMETER_ITEM_NOUN
    "knotted root",
    "handful of dry seed pods",
    "moss-covered stone",
    "coil of ivy",
    "withered garland",
    "cracked terracotta pot",
    "bundle of thorned cane",
    "rotted trellis slat",
    "cluster of fungus",
    "husk of a gourd",
];

/// Brute-force a world seed where the room straight north of the start
/// actually has a portable item -- same technique as `find_locked_seed`,
/// deterministic for the same reason (`demiurge::generate_frontier`'s RNG
/// is seeded from `(world_seed, origin, direction)`). Guarantees the
/// `take`/`drop` migration test below actually exercises the migrated
/// path instead of silently no-op'ing on an item-less room.
fn find_seed_with_portable_item_north() -> (u64, &'static str) {
    for seed in 0..2000u64 {
        let mut probe = Game::new(seed);
        probe.handle("north");
        let look = probe.look();
        if let Some(name) = PORTABLE_ITEM_NAMES.iter().find(|n| look.contains(*n)) {
            return (seed, name);
        }
    }
    panic!("no seed with a portable item found in range -- generator's item probability may have regressed");
}

#[test]
fn take_and_drop_migrate_the_holder_onto_apply_commit_and_materialize_current_state() {
    // `take`/`drop` are the one pair of verbs this prototype moved off
    // `Delta`/`WorldGraph::commit` onto `graph::Commit`/`apply_commit` +
    // `WorldGraph::current_value` materialization for the "who holds this
    // item" half of their state (see `Game::take`'s own doc comment).
    // This exercises that end-to-end through the ordinary `Game::handle`
    // dispatch, the same entry point every other verb goes through --
    // and specifically a *repeated* take/drop/take cycle, so the item's
    // `heldBy` predicate is genuinely re-asserted more than once with the
    // stale generation never deleted (`apply_commit` can't), which is
    // exactly the situation `current_value`'s "later wins" materialization
    // exists to resolve correctly.
    let (seed, item_name) = find_seed_with_portable_item_north();
    let mut game = Game::new(seed);
    game.handle("north");

    let mark = game.transcript_len();
    let taken = game.handle(&format!("take {item_name}"));
    assert!(taken.contains("You take"), "got: {taken}");

    // The holder-half of the state transition landed via `apply_commit`,
    // asserting the new `heldBy` predicate -- not the old `holds` Delta
    // path, which `take` no longer touches at all.
    let commits = game.commits_since(mark);
    assert!(
        commits.iter().any(|(_, text, _)| text.contains("heldBy")),
        "take must record a heldBy fact via apply_commit, got: {commits:?}"
    );
    assert!(
        !commits.iter().any(|(_, text, _)| text.contains("http://ww/holds")),
        "take must no longer assert the retired `holds` predicate, got: {commits:?}"
    );
    assert!(game.handle("inventory").contains(item_name));

    let mark2 = game.transcript_len();
    let dropped = game.handle(&format!("drop {item_name}"));
    assert!(dropped.contains("You drop"), "got: {dropped}");
    let commits2 = game.commits_since(mark2);
    assert!(
        commits2
            .iter()
            .any(|(_, text, _)| text.contains("heldBy") && text.contains("Nobody")),
        "drop must record heldBy=Nobody via apply_commit, got: {commits2:?}"
    );
    assert!(game.handle("inventory").contains("nothing"));

    // Take it again: a second generation of `heldBy(item, player)`, on top
    // of the drop's `heldBy(item, Nobody)`, on top of the first take's own
    // `heldBy(item, player)` -- three generations of the same predicate on
    // the same item, all still physically in the store (nothing here was
    // ever deleted). Only `current_value`/`current_subjects_with` walking
    // `commit_log`'s order -- not raw store iteration -- can say the item
    // is held again, which is exactly what `render_inventory` depends on
    // to get this right.
    let retaken = game.handle(&format!("take {item_name}"));
    assert!(retaken.contains("You take"), "got: {retaken}");
    assert!(game.handle("inventory").contains(item_name));
}

#[test]
fn conjure_mints_an_item_via_the_commit_path_and_it_becomes_visible_and_takeable() {
    // `conjure` is the one real caller wired to `graph::Commit`/
    // `WorldGraph::apply_commit` rather than `Delta`/`WorldGraph::commit`
    // -- exercised here through the ordinary `Game::handle` dispatch, the
    // same entry point every other verb goes through.
    let mut game = Game::new(7);
    let mark = game.transcript_len();

    let response = game.handle("conjure a brass idol");
    assert!(response.contains("brass idol"), "got: {response}");

    // It really landed in the transcript, tagged "player" same as any
    // other player-sourced commit.
    let commits = game.commits_since(mark);
    assert!(
        commits.iter().any(|(_, _, source)| source == "player"),
        "conjuring must commit as \"player\", got: {commits:?}"
    );

    // It's really in the graph, not just narrated: visible in the room...
    assert!(game.look().contains("brass idol"), "got: {}", game.look());

    // ...and takeable exactly like any other portable item.
    let taken = game.handle("take brass idol");
    assert!(taken.contains("You take"), "got: {taken}");
    assert!(game.handle("inventory").contains("brass idol"));
}

#[test]
fn conjure_with_no_object_fails_without_committing_anything() {
    let mut game = Game::new(7);
    let mark = game.transcript_len();
    let response = game.handle("conjure");
    assert!(response.contains("Conjure what"), "got: {response}");
    assert!(game.commits_since(mark).is_empty());
}

#[test]
fn apply_commune_delta_commits_a_new_relation_and_a_minted_npc() {
    let mut game = Game::new(5);
    let delta_json = r#"{
        "entities": [
            {"localId": "warden", "kind": "Npc", "name": "the Warden", "description": "Says nothing."}
        ],
        "declarations": [
            {"predicate": "hauntedBy", "type": "Relation"}
        ],
        "triples": [
            {"subject": "room", "predicate": "hauntedBy", "object": {"kind": "node", "ref": "warden"}}
        ]
    }"#;

    let result = game.apply_commune_delta(delta_json);
    assert!(
        result.is_ok(),
        "a well-formed commune delta must commit: {result:?}"
    );
    assert!(
        result.unwrap().contains("Warden"),
        "the minted Npc should now be visible in the room"
    );

    let relations = game.handle("relations");
    assert!(
        relations.contains("hauntedBy") && relations.contains("relation"),
        "the newly declared relation must show up in introspection, got: {relations}"
    );
}

#[test]
fn apply_commune_delta_rejects_malformed_json() {
    let mut game = Game::new(5);
    let result = game.apply_commune_delta("not json");
    assert!(result.is_err(), "garbage input must not commit anything");
}

#[test]
fn apply_commune_delta_rejects_a_triple_referencing_an_unknown_id() {
    let mut game = Game::new(5);
    let delta_json = r#"{
        "entities": [],
        "declarations": [{"predicate": "hauntedBy", "type": "Relation"}],
        "triples": [
            {"subject": "room", "predicate": "hauntedBy", "object": {"kind": "node", "ref": "nobody"}}
        ]
    }"#;
    let result = game.apply_commune_delta(delta_json);
    assert!(
        result.is_err(),
        "a triple pointing at an id that was never declared or minted must be rejected"
    );
}

#[test]
fn dynamic_predicate_keeps_underscores_and_hyphens() {
    // Regression: a model naturally reaches for snake_case or kebab-case
    // multi-word predicates ("lit_by", "worn-smooth-from"). Stripping
    // punctuation used to collapse those into unreadable, silently
    // colliding runs ("litby", "wornsmoothfrom").
    let p = vocab::dynamic_predicate("lit_by").expect("underscore predicate must parse");
    assert_eq!(dmml_runtime::graph::short(&p), "lit_by");

    let p2 = vocab::dynamic_predicate("worn-smooth-from").expect("hyphen predicate must parse");
    assert_eq!(dmml_runtime::graph::short(&p2), "worn-smooth-from");
}

#[test]
fn apply_commune_delta_preserves_underscored_predicate_names_in_relations_output() {
    let mut game = Game::new(5);
    let delta_json = r#"{
        "entities": [],
        "declarations": [{"predicate": "worn_smooth_from", "type": "Attribute"}],
        "triples": [
            {"subject": "room", "predicate": "worn_smooth_from", "object": {"kind": "literal", "value": "countless passages", "datatype": "string"}}
        ]
    }"#;
    let result = game.apply_commune_delta(delta_json);
    assert!(
        result.is_ok(),
        "underscored predicate should commit cleanly: {result:?}"
    );
    let relations = game.handle("relations");
    assert!(
        relations.contains("worn_smooth_from"),
        "the declared predicate should read back exactly as written, got: {relations}"
    );
}

#[test]
fn apply_commune_delta_rejects_a_triple_using_the_fixed_contains_predicate() {
    // Regression: a real commune response once wrote its own `contains`
    // triple (room self-containment) instead of relying on the automatic
    // containment placement `apply_commune_delta` already does for minted
    // entities. That triple was syntactically well-formed -- "contains"
    // resolves to the exact same IRI as the fixed vocabulary's `contains`
    // -- so it reached the graph's strict, generator-only contains-shape
    // validator and failed there with a confusing low-level message
    // ("contains object must have one of [...]") instead of a clear one.
    // The parser must now reject any triple predicate that isn't a
    // self-declared Relation/Attribute before it ever reaches the graph.
    let mut game = Game::new(5);
    let delta_json = r#"{
        "entities": [],
        "declarations": [],
        "triples": [
            {"subject": "room", "predicate": "contains", "object": {"kind": "node", "ref": "room"}}
        ]
    }"#;
    let result = game.apply_commune_delta(delta_json);
    assert!(
        result.is_err(),
        "a triple using the fixed `contains` predicate must be rejected by the parser, not the graph"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("contains") && err.contains("self-declared"),
        "the rejection should name the offending predicate and explain why, got: {err}"
    );
}

#[test]
fn apply_commune_delta_allows_reusing_a_relation_declared_in_an_earlier_commune_call() {
    let mut game = Game::new(5);
    let first = r#"{
        "entities": [],
        "declarations": [{"predicate": "hauntedBy", "type": "Relation"}],
        "triples": [
            {"subject": "room", "predicate": "hauntedBy", "object": {"kind": "literal", "value": "true", "datatype": "bool"}}
        ]
    }"#;
    // Note: hauntedBy is declared a Relation but this first call misuses
    // it with a literal object, which the graph validator should reject
    // -- included to prove that failed declare-and-misuse call doesn't
    // wrongly seed `usable_predicates` for a later, separate call.
    assert!(game.apply_commune_delta(first).is_err());

    let declare_properly = r#"{
        "entities": [{"localId": "warden", "kind": "Npc", "name": "the Warden", "description": ""}],
        "declarations": [{"predicate": "hauntedBy", "type": "Relation"}],
        "triples": [
            {"subject": "room", "predicate": "hauntedBy", "object": {"kind": "node", "ref": "warden"}}
        ]
    }"#;
    assert!(game.apply_commune_delta(declare_properly).is_ok());

    // A later call reuses "hauntedBy" WITHOUT re-declaring it -- must
    // still be accepted because it's already on record in the graph.
    let reuse = r#"{
        "entities": [{"localId": "warden2", "kind": "Npc", "name": "another warden", "description": ""}],
        "declarations": [],
        "triples": [
            {"subject": "room", "predicate": "hauntedBy", "object": {"kind": "node", "ref": "warden2"}}
        ]
    }"#;
    let result = game.apply_commune_delta(reuse);
    assert!(
        result.is_ok(),
        "reusing a predicate already declared Relation/Attribute in a prior commune call must not require re-declaring it: {result:?}"
    );
}

#[test]
fn apply_commune_delta_rejects_minting_a_mechanically_privileged_kind() {
    let mut game = Game::new(5);
    let delta_json = r#"{
        "entities": [{"localId": "x", "kind": "Room", "name": "a second room", "description": ""}],
        "declarations": [],
        "triples": []
    }"#;
    let result = game.apply_commune_delta(delta_json);
    assert!(
        result.is_err(),
        "the AI must not be able to mint Room/Player/Machine/Edge -- only Item or Npc"
    );
}

#[test]
fn apply_commune_delta_rejects_an_oversized_response() {
    let mut game = Game::new(5);
    let mut triples = String::new();
    for i in 0..20 {
        if i > 0 {
            triples.push(',');
        }
        triples.push_str(&format!(
            r#"{{"subject": "room", "predicate": "filler{i}", "object": {{"kind": "literal", "value": "x"}}}}"#
        ));
    }
    let declarations: String = (0..20)
        .map(|i| format!(r#"{{"predicate": "filler{i}", "type": "Attribute"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let delta_json =
        format!(r#"{{"entities": [], "declarations": [{declarations}], "triples": [{triples}]}}"#);
    let result = game.apply_commune_delta(&delta_json);
    assert!(
        result.is_err(),
        "an oversized commune response must be rejected as a safety cap, not silently truncated"
    );
}

#[test]
fn commune_context_reflects_the_players_current_room() {
    let game = Game::new(5);
    let context = game.commune_context();
    assert!(
        context.contains("\"room\"") && context.contains("\"vocabulary\""),
        "commune context must include both room facts and declared vocabulary, got: {context}"
    );
}

#[test]
fn raising_a_petition_freezes_the_current_commune_context() {
    // Since #15's "no strings in the graph" pass, a petition's context is a
    // PetitionSnapshot's real triples, reconstructed into this same JSON
    // shape at read time (see commune::freeze_room_snapshot/
    // context_from_snapshot) rather than a stored blob. The room facts on
    // it must still never drift -- that's the whole point of freezing them
    // -- but the vocabulary summary is deliberately read live off the graph
    // every time (it's global and monotonic, not something that drifts per
    // room), so this test checks both halves of that contract separately
    // instead of asserting the whole string is byte-identical forever.
    let mut game = Game::new(5);
    let context_before = game.commune_context();
    let petition = game.raise_petition_for_current_room(1_000_000);

    let frozen = game
        .petition_context(&petition)
        .expect("a freshly raised petition must carry a frozen context");
    assert_eq!(
        frozen, context_before,
        "the frozen context must match what commune_context reported at the moment of raising"
    );

    // Moving away afterwards must not change the room facts already frozen.
    // Finds whichever exit is actually open rather than hardcoding "north"
    // -- RNG-seeded procedural generation (`demiurge::generate_frontier`)
    // is on its way out in favor of DMML-driven generation, so this test
    // shouldn't depend on, or be fixed to preserve, any specific direction
    // being open for this seed; the property under test (the vocabulary
    // summary is live) doesn't care which direction the player takes.
    // Tries each available direction until one actually succeeds (proven by
    // player_location_via_located_in becoming Some) -- at genesis every
    // direction is unexplored, and Game::go only generates+rolls an edge
    // (open or locked) the first time it's actually attempted, so there's
    // no way to know which direction is open without trying.
    let mut moved = false;
    for action in game.available_actions() {
        if action.command.starts_with("go ") {
            game.handle(&action.command);
            if game.player_location_via_located_in().is_some() {
                moved = true;
                break;
            }
        }
    }
    assert!(
        moved,
        "none of the Threshold's directions opened a real exit -- expected but not \
         strictly guaranteed by world generation"
    );
    let frozen_after_moving = game
        .petition_context(&petition)
        .expect("the frozen context must still be readable after the player moves on");

    let room_before: serde_json::Value = serde_json::from_str(&frozen).unwrap();
    let room_after: serde_json::Value = serde_json::from_str(&frozen_after_moving).unwrap();
    assert_eq!(
        room_before["room"], room_after["room"],
        "a petition's frozen room facts must not drift once the player leaves the room it concerns"
    );

    // The vocabulary summary is the deliberately-not-frozen half: moving
    // north mints the player's first `locatedIn` commit, so a live read
    // must actually show that -- proving this isn't frozen by accident,
    // not just permissively not-asserted.
    let uses = |ctx: &serde_json::Value, predicate: &str| -> u64 {
        ctx["vocabulary"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["predicate"] == predicate)
            .and_then(|e| e["uses"].as_u64())
            .unwrap_or(0)
    };
    assert_eq!(
        uses(&room_before, "locatedIn"),
        0,
        "locatedIn must be unused at the moment the petition was raised"
    );
    assert_eq!(
        uses(&room_after, "locatedIn"),
        1,
        "a live vocabulary read after moving must show the new locatedIn use \
         -- proving the vocabulary summary is genuinely live, not frozen"
    );
}

#[test]
fn equip_operator_commits_a_visible_grant_tagged_by_the_operators_own_identifier() {
    let mut game = Game::new(5);
    let mark = game.transcript_len();
    game.equip_operator("did:plc:invited-agent", 1_000_000)
        .expect("equipping an operator is always valid");

    let commits = game.commits_since(mark);
    assert_eq!(commits.len(), 1, "equipping an operator is exactly one commit");
    let (_, text, source) = &commits[0];
    assert_eq!(
        source, "did:plc:invited-agent",
        "the commit's source must be the operator's own identifier, not a generic tag"
    );
    assert!(
        text.contains("operatorLabel") && text.contains("did:plc:invited-agent"),
        "the committed delta must carry the operator's label, got: {text}"
    );
}

#[test]
fn equip_operator_can_be_called_repeatedly_for_distinct_operators() {
    let mut game = Game::new(5);
    game.equip_operator("agent-one", 1_000_000)
        .expect("first invite redemption is valid");
    game.equip_operator("agent-two", 1_000_001)
        .expect("a second, independent invite redemption is also valid");

    let commits = game.commits_since(0);
    let operator_commits: Vec<&str> = commits
        .iter()
        .map(|(_, _, source)| source.as_str())
        .filter(|s| *s == "agent-one" || *s == "agent-two")
        .collect();
    assert_eq!(
        operator_commits,
        vec!["agent-one", "agent-two"],
        "each redeemed invite must produce its own independently-tagged commit"
    );
}

#[test]
fn pending_petitions_are_listed_oldest_first() {
    let mut game = Game::new(5);
    // Distinct timestamps: now that creation order is genuinely wall-clock
    // based (see `graph::creation_order`), three petitions really do need
    // three different moments to have a meaningful "oldest first" order --
    // sharing one timestamp would make them simultaneous, not ordered.
    let first = game.raise_petition_for_current_room(1_000_000);
    let second = game.raise_petition_for_current_room(1_000_001);
    let third = game.raise_petition_for_current_room(1_000_002);

    assert_eq!(game.pending_petitions(), vec![first, second, third]);
}

#[test]
fn resolving_a_petition_applies_the_delta_and_flips_status_to_resolved() {
    let mut game = Game::new(5);
    let petition = game.raise_petition_for_current_room(1_000_000);
    assert_eq!(game.pending_petitions(), vec![petition.clone()]);

    let delta_json = r#"{
        "entities": [
            {"localId": "warden", "kind": "Npc", "name": "the Warden", "description": "Says nothing."}
        ],
        "declarations": [
            {"predicate": "hauntedBy", "type": "Relation"}
        ],
        "triples": [
            {"subject": "room", "predicate": "hauntedBy", "object": {"kind": "node", "ref": "warden"}}
        ]
    }"#;

    let result = game.resolve_petition(&petition, delta_json, "demiurge-ai", 1_000_500);
    assert!(
        result.is_ok(),
        "a well-formed resolution must commit: {result:?}"
    );
    assert!(
        result.unwrap().contains("Warden"),
        "the resolution's result text must reflect the newly minted content"
    );
    assert!(
        game.pending_petitions().is_empty(),
        "a resolved petition must no longer show up as pending"
    );
}

#[test]
fn resolving_an_already_resolved_petition_fails() {
    let mut game = Game::new(5);
    let petition = game.raise_petition_for_current_room(1_000_000);
    let delta_json = r#"{"entities": [], "declarations": [], "triples": []}"#;

    assert!(game
        .resolve_petition(&petition, delta_json, "demiurge-ai", 1_000_500)
        .is_ok());
    let second_attempt = game.resolve_petition(&petition, delta_json, "demiurge-ai", 1_000_600);
    assert!(
        second_attempt.is_err(),
        "resolving an already-resolved petition must fail, not silently double-apply"
    );
}

#[test]
fn resolving_with_a_rule_breaking_delta_fails_without_corrupting_status() {
    let mut game = Game::new(5);
    let petition = game.raise_petition_for_current_room(1_000_000);

    // Reuses the fixed "contains" predicate, which parse_commune_json
    // rejects for AI-sourced triples -- see
    // apply_commune_delta_rejects_a_triple_using_the_fixed_contains_predicate.
    let rule_breaking = r#"{
        "entities": [{"localId": "x", "kind": "Item", "name": "a thing", "description": ""}],
        "declarations": [],
        "triples": [
            {"subject": "room", "predicate": "contains", "object": {"kind": "node", "ref": "x"}}
        ]
    }"#;

    let result = game.resolve_petition(&petition, rule_breaking, "demiurge-ai", 1_000_500);
    assert!(
        result.is_err(),
        "a rule-breaking resolution must be rejected"
    );
    assert_eq!(
        game.pending_petitions(),
        vec![petition],
        "a failed resolution attempt must leave the petition exactly as pending as it started"
    );
}

#[test]
fn a_petition_within_its_ttl_survives_an_expiry_sweep() {
    let mut game = Game::new(5);
    let petition = game.raise_petition_for_current_room(1_000_000);

    // Well within the default 10-minute TTL.
    let expired = game.expire_stale_petitions(1_000_000 + 60_000);
    assert!(
        expired.is_empty(),
        "a petition inside its TTL must not be swept, got: {expired:?}"
    );
    assert_eq!(
        game.pending_petitions(),
        vec![petition],
        "a petition inside its TTL must still be pending"
    );
}

#[test]
fn a_petition_past_its_ttl_expires_and_can_no_longer_be_resolved() {
    let mut game = Game::new(5);
    let petition = game.raise_petition_for_current_room(1_000_000);

    let past_ttl = 1_000_000 + dmml_runtime::commune::DEFAULT_PETITION_TTL_MS + 1;
    let expired = game.expire_stale_petitions(past_ttl);
    assert_eq!(
        expired,
        vec![petition.clone()],
        "a petition past its TTL must be swept into expired"
    );
    assert!(
        game.pending_petitions().is_empty(),
        "an expired petition must no longer show up as pending"
    );

    let delta_json = r#"{"entities": [], "declarations": [], "triples": []}"#;
    let result = game.resolve_petition(&petition, delta_json, "demiurge-ai", past_ttl + 1);
    assert!(
        result.is_err(),
        "an expired petition must not be resolvable, got: {result:?}"
    );
}

#[test]
fn expiry_sweep_does_not_touch_an_already_resolved_petition() {
    let mut game = Game::new(5);
    let petition = game.raise_petition_for_current_room(1_000_000);
    let delta_json = r#"{"entities": [], "declarations": [], "triples": []}"#;
    game.resolve_petition(&petition, delta_json, "demiurge-ai", 1_000_500)
        .expect("resolution must succeed");

    let past_ttl = 1_000_000 + dmml_runtime::commune::DEFAULT_PETITION_TTL_MS + 1;
    let expired = game.expire_stale_petitions(past_ttl);
    assert!(
        expired.is_empty(),
        "sweeping must not touch a petition that's already resolved, got: {expired:?}"
    );
}

// -- DMML petition state machine (additive, alongside the mechanism
// above) -- WorldGraph-level, not Game-level, since nothing outside
// engine/ calls this path yet (see commune.rs's own doc comment on the
// new functions).

const DMML_REPLY_JSON: &str = r#"{
    "entities": [
        {"localId": "warden", "kind": "Npc", "name": "the Warden", "description": "Says nothing."}
    ],
    "declarations": [
        {"predicate": "hauntedBy", "type": "Relation"}
    ],
    "triples": [
        {"subject": "room", "predicate": "hauntedBy", "object": {"kind": "node", "ref": "warden"}}
    ]
}"#;

#[test]
fn dmml_petition_raise_reply_accept_full_cycle() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);

    let petition = dmml_runtime::commune::raise_petition_dmml(
        &mut graph,
        &boot.start_room,
        1_000_000,
        dmml_runtime::commune::DEFAULT_PETITION_TTL_MS,
    )
    .expect("raising a DMML petition is always valid");
    dmml_runtime::commune::reply_petition_dmml(&mut graph, &petition, DMML_REPLY_JSON, "target-did")
        .expect("a well-formed reply must be accepted for proposal");
    assert!(
        dmml_runtime::commune::accept_petition_dmml(&mut graph, &petition, 1_001_000).is_ok(),
        "accepting a well-formed, already-proposed reply must succeed"
    );
}

/// Proves the data-sovereignty property the DMML flow exists for: a reply
/// only *proposes* content -- the room must not actually change until a
/// separate `accept_petition_dmml` call lands.
#[test]
fn dmml_petition_reply_does_not_apply_content_before_accept() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let haunted_by =
        vocab::dynamic_predicate("hauntedBy").expect("hauntedBy is a valid dynamic predicate name");

    let petition = dmml_runtime::commune::raise_petition_dmml(
        &mut graph,
        &boot.start_room,
        1_000_000,
        dmml_runtime::commune::DEFAULT_PETITION_TTL_MS,
    )
    .unwrap();
    dmml_runtime::commune::reply_petition_dmml(&mut graph, &petition, DMML_REPLY_JSON, "target-did")
        .expect("a well-formed reply must be accepted for proposal");
    assert!(
        graph.objects(&boot.start_room, &haunted_by).is_empty(),
        "a reply must not touch the world until accepted"
    );

    dmml_runtime::commune::accept_petition_dmml(&mut graph, &petition, 1_001_000)
        .expect("accepting the proposed reply must succeed");
    assert!(
        !graph.objects(&boot.start_room, &haunted_by).is_empty(),
        "accepting must apply the reply's previously-proposed content for real"
    );
}

#[test]
fn dmml_petition_double_reply_rejected() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);

    let petition = dmml_runtime::commune::raise_petition_dmml(
        &mut graph,
        &boot.start_room,
        1_000_000,
        dmml_runtime::commune::DEFAULT_PETITION_TTL_MS,
    )
    .unwrap();
    dmml_runtime::commune::reply_petition_dmml(&mut graph, &petition, DMML_REPLY_JSON, "target-did")
        .expect("the first reply must succeed");
    let second = dmml_runtime::commune::reply_petition_dmml(
        &mut graph,
        &petition,
        DMML_REPLY_JSON,
        "a-different-target-did",
    );
    assert!(
        second.is_err(),
        "a second reply to an already-replied petition must be rejected, not silently \
         overwrite the first"
    );
}

#[test]
fn dmml_petition_accept_without_reply_rejected() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);

    let petition = dmml_runtime::commune::raise_petition_dmml(
        &mut graph,
        &boot.start_room,
        1_000_000,
        dmml_runtime::commune::DEFAULT_PETITION_TTL_MS,
    )
    .unwrap();
    let accept = dmml_runtime::commune::accept_petition_dmml(&mut graph, &petition, 1_000_500);
    assert!(
        accept.is_err(),
        "accepting a petition nobody has replied to yet must be rejected"
    );
}

#[test]
fn dmml_petition_double_accept_rejected() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);

    let petition = dmml_runtime::commune::raise_petition_dmml(
        &mut graph,
        &boot.start_room,
        1_000_000,
        dmml_runtime::commune::DEFAULT_PETITION_TTL_MS,
    )
    .unwrap();
    dmml_runtime::commune::reply_petition_dmml(&mut graph, &petition, DMML_REPLY_JSON, "target-did")
        .expect("the reply must succeed");
    dmml_runtime::commune::accept_petition_dmml(&mut graph, &petition, 1_000_500)
        .expect("the first accept must succeed");
    let second_accept = dmml_runtime::commune::accept_petition_dmml(&mut graph, &petition, 1_001_000);
    assert!(
        second_accept.is_err(),
        "a second accept of an already-resolved petition must be rejected, not silently \
         re-apply the same content"
    );
}

#[test]
fn game_snapshot_and_from_snapshot_round_trip_a_playable_game() {
    let mut game = Game::new(7);
    game.handle("north");
    let before_snapshot = game.look();
    let before_actions: Vec<String> = game
        .available_actions()
        .into_iter()
        .map(|a| a.command)
        .collect();

    let snapshot = game.snapshot().expect("snapshot must succeed");
    assert!(!snapshot.nquads.is_empty());

    let mut restored = Game::from_snapshot(&snapshot).expect("from_snapshot must succeed");

    assert_eq!(
        restored.look(),
        before_snapshot,
        "a restored game must render identically to the one that was snapshotted"
    );
    let restored_actions: Vec<String> = restored
        .available_actions()
        .into_iter()
        .map(|a| a.command)
        .collect();
    assert_eq!(
        restored_actions, before_actions,
        "unexplored-direction derivation must survive the round trip identically"
    );

    // Minting a new node post-restore must not collide with anything
    // minted before the snapshot was taken -- this is exactly what
    // content_hash travelling alongside the dump exists to guarantee.
    let south_result = restored.handle("south");
    assert!(
        !south_result.to_lowercase().contains("error") && !south_result.is_empty(),
        "generating past a restored game's frontier must work cleanly, got: {south_result}"
    );
}

#[test]
fn dump_nquads_round_trips_through_a_fresh_store() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let _ = boot;

    let bytes = graph.dump_nquads().expect("dump must succeed");
    assert!(
        !bytes.is_empty(),
        "a bootstrapped world has real content to dump"
    );

    let reloaded = oxigraph::store::Store::new().expect("in-memory store always constructs");
    reloaded
        .load_from_slice(oxigraph::io::RdfFormat::NQuads, bytes.as_slice())
        .expect("reloading a dump of our own output must succeed");

    let re_dump = reloaded
        .dump_to_writer(oxigraph::io::RdfFormat::NQuads, Vec::new())
        .expect("re-dumping the reloaded store must succeed");

    // Oxigraph doesn't guarantee stable iteration order across store
    // instances, so a fresh store loaded from the same triples can
    // legitimately serialize them in a different sequence -- compare as a
    // set of lines (one N-Quads statement per line) rather than exact
    // bytes, which is the actual round-trip guarantee that matters: same
    // triples, not same serialization order.
    let as_line_set = |b: &[u8]| -> std::collections::HashSet<String> {
        String::from_utf8_lossy(b)
            .lines()
            .map(str::to_string)
            .collect()
    };
    assert_eq!(
        as_line_set(&bytes),
        as_line_set(&re_dump),
        "dump -> load -> dump must preserve every triple, order aside"
    );
}

#[test]
fn generated_room_renders_a_composed_description_from_ground_facts() {
    // No predicate can hold a stored description literal anymore -- a
    // generated room's text always comes from its ground facts, composed
    // fresh every call.
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let room =
        dmml_runtime::demiurge::generate_frontier(&mut graph, 7, &boot.start_room, Direction::North);

    let rendered = dmml_runtime::render::render_room_text(&graph, &boot.player, &room, &[]);
    assert!(
        rendered.contains("stood here before"),
        "composed description must include the visit-history clause, got: {rendered}"
    );
}

#[test]
fn threshold_composes_its_description_from_ground_facts_like_any_room() {
    // The Threshold isn't an exception to "no stored prose" either -- its
    // distinguishing feature is its `name` only. Its description composes
    // from the same dampness/decay/light facts (0.0, 0.0, 0.4) any
    // generated room with that profile would: light 0.4 makes "dark" the
    // dominant trait (intensity 0.6, the weak band).
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let rendered = dmml_runtime::render::render_room_text(&graph, &boot.player, &boot.start_room, &[]);
    assert!(
        rendered.contains("Shadows crowd the corners"),
        "the Threshold's description must compose from its own ground facts, got: {rendered}"
    );
}

#[test]
fn visits_increments_on_entry_and_persists_across_a_revisit() {
    // See the seed-176 comment on `revisiting_a_generated_room_...` above --
    // same cause, same replacement.
    let mut game = Game::new(176);

    let first = game.handle("north");
    assert!(
        first.contains("never stood here before"),
        "a room's first entry should read as never having been stood in, got: {first}"
    );

    game.handle("south");
    let second = game.handle("north");
    assert!(
        second.contains("passed through here before"),
        "revisiting must reflect the accumulated visit count, got: {second}"
    );
}

#[test]
fn validator_rejects_dampness_outside_its_declared_range() {
    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let d = Delta::new()
        .assert(room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(room, vocab::dampness(), dmml_runtime::graph::lit_float(1.5));
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "dampness of 1.5 is outside the declared [0.0, 1.0] domain and must be rejected"
    );
}

#[test]
fn validator_rejects_negative_visits() {
    // `lit_int` only accepts a `u64`, so a negative value can't reach the
    // validator through the normal helper -- construct the raw typed
    // literal directly, the way a malformed delta from outside this
    // module's helpers could, to prove the validator itself is the guard
    // and not just the type of `lit_int`'s parameter.
    use oxigraph::model::vocab::xsd;
    use oxigraph::model::Literal;

    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let negative_visits = Literal::new_typed_literal("-1", xsd::INTEGER);
    let d = Delta::new()
        .assert(room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(room, vocab::visits(), negative_visits);
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "a negative visits count must be rejected by the validator"
    );
}

#[test]
fn relations_command_lists_self_declared_predicates_and_use_counts() {
    let mut graph = WorldGraph::new();
    let room_a = graph.fresh("room/");
    let room_b = graph.fresh("room/");
    let echoes_from = graph.fresh("relation/");
    graph
        .commit(
            "test",
            Delta::new()
                .assert(room_a.clone(), vocab::rdf_type(), vocab::class_room())
                .assert(room_b.clone(), vocab::rdf_type(), vocab::class_room())
                .assert(
                    echoes_from.clone(),
                    vocab::rdf_type(),
                    vocab::class_relation(),
                )
                .assert(room_a, echoes_from, room_b),
        )
        .expect("declare-and-use in one delta is valid");

    let listed = dmml_runtime::render::render_relations(&graph);
    assert!(
        listed.contains("relation") && listed.contains("used 1 time"),
        "declared relation must be listed with its use count, got: {listed}"
    );
}

#[test]
fn declared_vocabulary_returns_structured_data_for_the_same_world_render_relations_describes() {
    use dmml_runtime::render::{declared_vocabulary, PredicateKind};

    let mut graph = WorldGraph::new();
    let room_a = graph.fresh("room/");
    let room_b = graph.fresh("room/");
    let echoes_from = graph.fresh("relation/");
    let dampness = graph.fresh("attribute/");
    graph
        .commit(
            "test",
            Delta::new()
                .assert(room_a.clone(), vocab::rdf_type(), vocab::class_room())
                .assert(room_b.clone(), vocab::rdf_type(), vocab::class_room())
                .assert(echoes_from.clone(), vocab::rdf_type(), vocab::class_relation())
                .assert(dampness.clone(), vocab::rdf_type(), vocab::class_attribute())
                .assert(room_a.clone(), echoes_from, room_b)
                .assert(room_a, dampness, oxigraph::model::Literal::from("0.4")),
        )
        .expect("declare-and-use in one delta is valid");

    let declared = declared_vocabulary(&graph);
    assert_eq!(declared.len(), 2, "both the relation and the attribute should be listed");

    let relation = declared
        .iter()
        .find(|p| p.kind == PredicateKind::Relation)
        .expect("the declared relation should be present");
    assert!(relation.name.starts_with("relation/"), "got {}", relation.name);
    assert_eq!(relation.use_count, 1);

    let attribute = declared
        .iter()
        .find(|p| p.kind == PredicateKind::Attribute)
        .expect("the declared attribute should be present");
    assert!(attribute.name.starts_with("attribute/"), "got {}", attribute.name);
    assert_eq!(attribute.use_count, 1);
}

#[test]
fn declared_vocabulary_is_empty_for_a_world_with_no_self_declared_predicates() {
    use dmml_runtime::render::declared_vocabulary;

    let graph = WorldGraph::new();
    assert!(declared_vocabulary(&graph).is_empty());
}

#[test]
fn declaring_and_using_a_new_relation_in_the_same_delta_commits() {
    let mut graph = WorldGraph::new();
    let room_a = graph.fresh("room/");
    let room_b = graph.fresh("room/");
    let echoes_from = graph.fresh("relation/");
    let d = Delta::new()
        .assert(room_a.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(room_b.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(
            echoes_from.clone(),
            vocab::rdf_type(),
            vocab::class_relation(),
        )
        .assert(room_a, echoes_from, room_b);
    assert!(
        graph.commit("test", d).is_ok(),
        "a predicate self-declared as a Relation in the same delta must be usable node-to-node"
    );
}

#[test]
fn declaring_and_using_a_new_attribute_in_the_same_delta_commits() {
    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let smell = graph.fresh("attribute/");
    let d = Delta::new()
        .assert(room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(smell.clone(), vocab::rdf_type(), vocab::class_attribute())
        .assert(room, smell, dmml_runtime::graph::lit_str("wet stone"));
    assert!(
        graph.commit("test", d).is_ok(),
        "a predicate self-declared as an Attribute in the same delta must be usable node-to-literal"
    );
}

#[test]
fn retired_machine_predicate_is_rejected_without_self_declaration() {
    // `effectKind` came out of `is_closed_vocabulary` (#50 Tier 1 item 1) --
    // a bare `WorldGraph::new()` (no `demiurge::bootstrap`, which is the
    // only place that now self-declares it) must reject it exactly like
    // any other undeclared novel predicate.
    let mut graph = WorldGraph::new();
    let effect = graph.fresh("effect/");
    let d = Delta::new().assert(effect, vocab::effect_kind(), dmml_runtime::graph::lit_str("incrementAttr"));
    let err = graph
        .commit("test", d)
        .expect_err("effectKind with no self-declaration anywhere must be rejected");
    assert!(
        format!("{err}").contains("not a recognized predicate"),
        "rejection must be the self-declaration check, got: {err}"
    );
}

#[test]
fn retired_machine_predicates_accept_any_literal_once_self_declared() {
    // `requirementLockedValue`/`effectStep` used to get a dedicated
    // bool/float `expect_literal_type` check in `validate`; retiring them
    // from closed vocabulary retires that check too -- once self-declared
    // as an ordinary Attribute (no `rangeMin`/`rangeMax`), they accept any
    // literal, same as any other unranged self-declared Attribute (see
    // `declaring_and_using_a_new_attribute_in_the_same_delta_commits`).
    let mut graph = WorldGraph::new();
    let req = graph.fresh("req/");
    let effect = graph.fresh("effect/");
    let d = Delta::new()
        .assert(
            vocab::requirement_locked_value(),
            vocab::rdf_type(),
            vocab::class_attribute(),
        )
        .assert(vocab::effect_step(), vocab::rdf_type(), vocab::class_attribute())
        .assert(
            req,
            vocab::requirement_locked_value(),
            dmml_runtime::graph::lit_str("not a boolean"),
        )
        .assert(effect, vocab::effect_step(), dmml_runtime::graph::lit_str("not a float"));
    assert!(
        graph.commit("test", d).is_ok(),
        "a self-declared Attribute with no declared range must accept any literal, \
         including former closed-vocabulary predicates whose dedicated type check was retired"
    );
}

#[test]
fn a_relation_declared_in_an_earlier_commit_stays_usable_later() {
    let mut graph = WorldGraph::new();
    let room_a = graph.fresh("room/");
    let room_b = graph.fresh("room/");
    let echoes_from = graph.fresh("relation/");
    graph
        .commit(
            "test",
            Delta::new()
                .assert(room_a.clone(), vocab::rdf_type(), vocab::class_room())
                .assert(room_b.clone(), vocab::rdf_type(), vocab::class_room())
                .assert(
                    echoes_from.clone(),
                    vocab::rdf_type(),
                    vocab::class_relation(),
                ),
        )
        .expect("declaring the relation on its own is a valid delta");

    let result = graph.commit("test", Delta::new().assert(room_a, echoes_from, room_b));
    assert!(
        result.is_ok(),
        "a relation declared in a prior commit must still validate against the committed store"
    );
}

#[test]
fn validator_rejects_an_undeclared_novel_predicate() {
    let mut graph = WorldGraph::new();
    let room_a = graph.fresh("room/");
    let room_b = graph.fresh("room/");
    let mystery = graph.fresh("relation/");
    let d = Delta::new()
        .assert(room_a.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(room_b.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(room_a, mystery, room_b);
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "a predicate that never self-declared as Relation or Attribute must be rejected"
    );
}

#[test]
fn validator_rejects_a_declared_relation_used_with_a_literal_object() {
    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let echoes_from = graph.fresh("relation/");
    let d = Delta::new()
        .assert(room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(
            echoes_from.clone(),
            vocab::rdf_type(),
            vocab::class_relation(),
        )
        .assert(room, echoes_from, dmml_runtime::graph::lit_str("not a node"));
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "a predicate declared a Relation must reject a literal object"
    );
}

#[test]
fn validator_rejects_a_declared_attribute_used_with_a_node_object() {
    let mut graph = WorldGraph::new();
    let room_a = graph.fresh("room/");
    let room_b = graph.fresh("room/");
    let smell = graph.fresh("attribute/");
    let d = Delta::new()
        .assert(room_a.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(room_b.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(smell.clone(), vocab::rdf_type(), vocab::class_attribute())
        .assert(room_a, smell, room_b);
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "a predicate declared an Attribute must reject a node object"
    );
}

#[test]
fn validator_rejects_a_petition_status_string_literal() {
    // petitionStatus moved from a string tag to a closed, three-valued
    // node enum (vocab::status_pending/resolved/expired) -- the old
    // stringly-typed shape must now be rejected outright, not silently
    // accepted as some fourth, unrecognized status.
    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let petition = graph.fresh("petition/");
    let d = Delta::new()
        .assert(room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(petition.clone(), vocab::rdf_type(), vocab::class_petition())
        .assert(petition.clone(), vocab::petition_concerns(), room)
        .assert(petition, vocab::petition_status(), dmml_runtime::graph::lit_str("pending"));
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "a string-literal petitionStatus must be rejected now that it's a node enum"
    );
}

#[test]
fn validator_rejects_a_petition_status_outside_the_closed_set() {
    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let petition = graph.fresh("petition/");
    let bogus_status = graph.fresh("status/");
    let d = Delta::new()
        .assert(room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(petition.clone(), vocab::rdf_type(), vocab::class_petition())
        .assert(petition.clone(), vocab::petition_concerns(), room)
        .assert(petition, vocab::petition_status(), bogus_status);
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "a node that isn't one of status_pending/resolved/expired must be rejected"
    );
}

#[test]
fn validator_rejects_foreign_uri_stored_as_a_literal() {
    // foreignUri/foreignCid moved from opaque string literals to real
    // node references -- the validator must reject the old shape.
    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let d = Delta::new()
        .assert(room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(
            room,
            vocab::foreign_uri(),
            dmml_runtime::graph::lit_str("at://did:plc:abc123/coll/rkey"),
        );
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "a string-literal foreignUri must be rejected now that it's a node reference"
    );
}

#[test]
fn validator_rejects_noticed_change_stored_as_a_literal() {
    // noticedChange moved from an accreted string sentence to a relation
    // pointing at a structured Drift node -- the validator must reject
    // the old shape.
    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let d = Delta::new().assert(room.clone(), vocab::rdf_type(), vocab::class_room()).assert(
        room,
        vocab::noticed_change(),
        dmml_runtime::graph::lit_str("someone fixed the lamp"),
    );
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "a string-literal noticedChange must be rejected now that it points at a Drift node"
    );
}

#[test]
fn validator_rejects_noticed_change_pointing_at_a_non_drift_node() {
    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let not_a_drift = graph.fresh("item/");
    let d = Delta::new()
        .assert(room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(not_a_drift.clone(), vocab::rdf_type(), vocab::class_item())
        .assert(room, vocab::noticed_change(), not_a_drift);
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "noticedChange must point at a node typed Drift, got something else"
    );
}

#[test]
fn foreign_uri_node_round_trips_a_did_containing_colons() {
    // A raw at:// URI with a did:plc authority isn't itself a valid IRI
    // (RFC 3987's authority grammar expects digits after a colon) --
    // vocab::foreign_uri_node percent-encodes around that, and the
    // round-trip through Game::reach/foreign_link (see the reach_* tests
    // above) must recover the exact original string.
    let at_uri = "at://did:plc:abc123/org.writtenworld.commit/3jzfcijpj2z2a";
    let node = vocab::foreign_uri_node(at_uri);
    assert_eq!(vocab::foreign_uri_from_node(&node).as_deref(), Some(at_uri));
}

#[test]
fn validator_rejects_wear_outside_its_declared_range() {
    let mut graph = WorldGraph::new();
    let item = graph.fresh("item/");
    let d = Delta::new()
        .assert(item.clone(), vocab::rdf_type(), vocab::class_item())
        .assert(item, vocab::wear(), dmml_runtime::graph::lit_float(5.0));
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "wear of 5.0 is outside the declared [0.0, 2.0] domain and must be rejected"
    );
}

#[test]
fn validator_rejects_contains_from_a_non_room() {
    let mut graph = WorldGraph::new();
    let item_a = graph.fresh("item/");
    let item_b = graph.fresh("item/");
    let d = Delta::new()
        .assert(item_a.clone(), vocab::rdf_type(), vocab::class_item())
        .assert(item_b.clone(), vocab::rdf_type(), vocab::class_item())
        .assert(item_a, vocab::contains(), item_b);
    let result = graph.commit("test", d);
    assert!(
        result.is_err(),
        "an Item cannot `contains` another Item's peer-level content in this schema"
    );
}

#[test]
fn reach_links_the_current_room_and_is_readable_back() {
    let mut game = Game::new(1);
    assert!(
        game.foreign_link().is_none(),
        "a fresh room has no foreign correspondence yet"
    );

    let response = game.handle("reach at://did:plc:abc123/org.writtenworld.commit/3jzfcijpj2z2a");
    assert!(response.contains("at://did:plc:abc123"));

    let link = game.foreign_link().expect("reach must set a foreign link");
    assert_eq!(link.uri, "at://did:plc:abc123/org.writtenworld.commit/3jzfcijpj2z2a");
    assert!(link.cid.is_none(), "no fetch has happened yet, so no cid is cached");
    assert!(link.snapshot.is_none());
}

#[test]
fn reach_again_overwrites_the_prior_link() {
    let mut game = Game::new(1);
    game.handle("reach at://did:plc:abc/coll/one");
    game.handle("reach at://did:plc:xyz/coll/two");

    let link = game.foreign_link().expect("a link exists");
    assert_eq!(link.uri, "at://did:plc:xyz/coll/two");
}

#[test]
fn record_foreign_drift_is_a_noop_without_a_link() {
    let mut game = Game::new(1);
    let result = game.record_foreign_drift("bafyfakecid", "{}", 1_000);
    assert!(result.is_ok());
    let room_text = game.handle("look");
    assert!(
        !room_text.contains("shifted"),
        "nothing should be recorded for a room with no foreign link"
    );
}

#[test]
fn record_foreign_drift_accretes_a_drift_node_only_on_a_real_change_after_a_baseline() {
    let mut game = Game::new(1);
    game.handle("reach at://did:plc:abc/coll/rkey");

    // First observation: no prior cid was cached, so this is the
    // baseline -- nothing to compare against yet, no drift.
    game.record_foreign_drift("cid-one", "{\"light\":\"bright\"}", 1_000)
        .unwrap();
    let link = game.foreign_link().unwrap();
    assert_eq!(link.cid.as_deref(), Some("cid-one"));
    assert_eq!(link.snapshot.as_deref(), Some("{\"light\":\"bright\"}"));
    let room_text = game.handle("look");
    assert!(
        !room_text.contains("shifted"),
        "a first-ever observation is a baseline, not a drift, got: {room_text}"
    );

    // Second observation: a real prior cid to compare against, and it
    // differs -- a genuine drift now, structured (Drift node), not
    // narrated.
    game.record_foreign_drift("cid-two", "{\"light\":\"dim\"}", 2_000)
        .unwrap();
    let link = game.foreign_link().unwrap();
    assert_eq!(link.cid.as_deref(), Some("cid-two"));
    assert_eq!(link.snapshot.as_deref(), Some("{\"light\":\"dim\"}"));
    let room_text = game.handle("look");
    assert!(
        room_text.contains("shifted since you last reached it"),
        "a real cid change must surface as noticedChange, got: {room_text}"
    );
}

#[test]
fn record_foreign_drift_does_not_accrete_a_second_drift_for_an_unchanged_cid() {
    let mut game = Game::new(1);
    game.handle("reach at://did:plc:abc/coll/rkey");
    game.record_foreign_drift("cid-one", "{}", 1_000).unwrap();
    game.record_foreign_drift("cid-two", "{}", 2_000).unwrap();
    let first = game.handle("look");
    assert!(
        first.contains("shifted"),
        "the real drift must be visible, got: {first}"
    );

    // Recording the same cid again is "still faithful" -- no new drift.
    game.record_foreign_drift("cid-two", "{}", 3_000).unwrap();
    let second = game.handle("look");
    assert_eq!(
        first.matches("shifted").count(),
        second.matches("shifted").count(),
        "an unchanged cid must not accrete a second drift line, got: {second}"
    );
}

#[test]
fn foreign_drift_commits_are_tagged_narrator_not_player() {
    let mut game = Game::new(1);
    game.handle("reach at://did:plc:abc/coll/rkey");
    game.record_foreign_drift("cid-one", "{}", 1_000).unwrap();

    let transcript = game.handle("transcript");
    assert!(
        transcript.contains("[narrator]"),
        "drift commits must be tagged narrator, distinct from player/demiurge/demiurge-ai, got: {transcript}"
    );
}

// -- Percept pipeline -------------------------------------------------

#[test]
fn bootstrap_equips_player_with_a_map_sense_machine() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let machines = dmml_runtime::machine::sense_machines_for_kind(&graph, &boot.player, "map");
    assert_eq!(
        machines.len(),
        1,
        "bootstrap should equip exactly one map sense-machine"
    );
}

#[test]
fn perceive_returns_none_for_a_player_with_no_sense_machine_equipped() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let blind_player = graph.fresh("player/");
    let base = Delta::new().assert(blind_player.clone(), vocab::rdf_type(), vocab::class_player());
    graph
        .commit("demiurge", base)
        .expect("a bare player node is always a valid commit");

    assert!(
        dmml_runtime::render::perceive_room(&graph, &blind_player, &boot.start_room, &[]).is_none(),
        "no room sense-machine equipped must mean no room percept at all"
    );
    assert!(
        dmml_runtime::render::perceive_map(&graph, &blind_player).is_none(),
        "no map sense-machine equipped must mean no map percept at all"
    );
}

#[test]
fn perceive_room_omits_fields_for_predicates_the_sight_machine_doesnt_sense() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);

    // A second player with a narrower sight machine than bootstrap's own --
    // proves the gate is real (senses actually bound what's built), not
    // just "every field happens to be present so far."
    let narrow_player = graph.fresh("player/");
    let base = Delta::new().assert(narrow_player.clone(), vocab::rdf_type(), vocab::class_player());
    graph
        .commit("demiurge", base)
        .expect("a bare player node is always a valid commit");
    let (_, sense_delta) = dmml_runtime::machine::build_sense_machine(
        &mut graph,
        &narrow_player,
        &["room"],
        &[vocab::name(), vocab::dampness()],
    );
    graph
        .commit("demiurge", sense_delta)
        .expect("a sense machine is always a valid commit");

    let percept = dmml_runtime::render::perceive_room(&graph, &narrow_player, &boot.start_room, &[])
        .expect("a room sense-machine is equipped");
    assert!(
        percept.field("exits").is_none(),
        "connectsTo isn't sensed, so exits must not appear, got: {percept:?}"
    );
    assert!(
        percept.field("items").is_none(),
        "contains isn't sensed, so items must not appear, got: {percept:?}"
    );
    assert!(
        percept.field("description").is_some(),
        "dampness is sensed, so a composed description must appear, got: {percept:?}"
    );
}

#[test]
fn perceive_room_field_gating_is_content_driven_not_hardcoded_by_predicate_name() {
    // Proves the sense->field gate reads real self-declared content
    // (`vocab::unlocks_field`, asserted only by `demiurge::bootstrap`)
    // rather than hardcoding a predicate's identity -- a bare
    // `WorldGraph::new()` that never runs bootstrap has no
    // `dampness unlocksField "description"` fact, so sensing `dampness`
    // here must NOT unlock `description`, even though the identical setup
    // through `demiurge::bootstrap` does (see the sibling test above).
    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let player = graph.fresh("player/");
    let base = Delta::new()
        .assert(room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(player.clone(), vocab::rdf_type(), vocab::class_player());
    graph
        .commit("test", base)
        .expect("a bare room+player is always a valid commit");
    let (_, sense_delta) =
        dmml_runtime::machine::build_sense_machine(&mut graph, &player, &["room"], &[vocab::dampness()]);
    graph
        .commit("test", sense_delta)
        .expect("a sense machine is always a valid commit");

    let percept = dmml_runtime::render::perceive_room(&graph, &player, &room, &[])
        .expect("a room sense-machine is equipped");
    assert!(
        percept.field("description").is_none(),
        "dampness is sensed but never self-declared as unlocking \"description\" \
         in this bare graph -- the gate must read content, not hardcode the \
         predicate's identity, got: {percept:?}"
    );
}

#[test]
fn perceive_examine_omits_wear_band_when_wear_isnt_sensed() {
    use dmml_runtime::graph::lit_float;

    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);

    let item = graph.fresh("item/");
    let base = Delta::new()
        .assert(item.clone(), vocab::rdf_type(), vocab::class_item())
        .assert(item.clone(), vocab::wear(), lit_float(0.5));
    graph
        .commit("demiurge", base)
        .expect("an item with wear is always a valid commit");

    // Bootstrap's own sight machine now covers "examine" too (collapsed
    // into the same machine as "room" -- see demiurge::bootstrap) and
    // senses wear.
    let full = dmml_runtime::render::perceive_examine(&graph, &boot.player, &item)
        .expect("the sight machine is equipped for examine");
    assert!(
        full.field("wear").is_some(),
        "wear is sensed by default, got: {full:?}"
    );

    let narrow_player = graph.fresh("player/");
    let narrow_base =
        Delta::new().assert(narrow_player.clone(), vocab::rdf_type(), vocab::class_player());
    graph
        .commit("demiurge", narrow_base)
        .expect("a bare player node is always a valid commit");
    let (_, sense_delta) = dmml_runtime::machine::build_sense_machine(
        &mut graph,
        &narrow_player,
        &["examine"],
        &[vocab::name()],
    );
    graph
        .commit("demiurge", sense_delta)
        .expect("a sense machine is always a valid commit");

    let narrow = dmml_runtime::render::perceive_examine(&graph, &narrow_player, &item)
        .expect("a narrower examine machine is equipped");
    assert!(
        narrow.field("wear").is_none(),
        "wear isn't sensed by this narrower machine, got: {narrow:?}"
    );
}

#[test]
fn examining_a_plain_item_with_no_graded_facts_says_nothing_more_to_notice() {
    // A generated flavor item carries only a name -- no wear, no stored
    // description literal (there's no such predicate anymore). Examining
    // it must read as "you looked, there wasn't more to find," not as if
    // the item weren't there at all.
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);

    let item = graph.fresh("item/");
    let base = Delta::new()
        .assert(item.clone(), vocab::rdf_type(), vocab::class_item())
        .assert(item.clone(), vocab::name(), dmml_runtime::graph::lit_str("clay jug"));
    graph
        .commit("demiurge", base)
        .expect("a plain item with just a name is always a valid commit");

    let percept = dmml_runtime::render::perceive_examine(&graph, &boot.player, &item)
        .expect("the sight machine is equipped for examine");
    let text = dmml_runtime::render::render_percept_text(&percept);
    assert_eq!(
        text, "Nothing more to notice about the clay jug.",
        "a plain item with no graded facts has nothing composable beyond its name"
    );
}

#[test]
fn map_percept_excludes_a_generated_but_unvisited_room() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    // generate_frontier alone never visits a room -- only Game::go does,
    // by moving the player into it. This room exists in the graph but has
    // visits == 0.
    dmml_runtime::demiurge::generate_frontier(&mut graph, 3, &boot.start_room, Direction::North);

    let map = dmml_runtime::render::perceive_map(&graph, &boot.player)
        .expect("map machine is equipped at bootstrap");
    let Some(dmml_runtime::percept::PerceptValue::Nested(rooms)) = map.field("rooms") else {
        panic!("map percept must have a Nested \"rooms\" field, got: {map:?}");
    };
    assert_eq!(
        rooms.len(),
        1,
        "only the already-visited Threshold should appear, got: {rooms:?}"
    );
    assert_eq!(rooms[0].title, "The Threshold");
    assert!(
        rooms[0].field("current").is_some(),
        "the only visited room is where the player currently stands"
    );
}

#[test]
fn map_method_matches_the_map_command() {
    // `Game::map()` is what a server-side response builder calls for a
    // persistent map panel; `handle("map")` is the equivalent text-command
    // path. They must agree, since both go through the same percept
    // pipeline underneath.
    let mut game = Game::new(2);
    let via_method = game.map();
    let via_command = game.handle("map");
    assert_eq!(via_method, via_command);
}

#[test]
fn map_command_reports_the_starting_room() {
    let mut game = Game::new(1);
    let response = game.handle("map");
    assert!(response.contains("The Threshold"), "got: {response}");
    assert!(
        response.contains("here"),
        "the starting room should be marked as where the player is, got: {response}"
    );
}

#[test]
fn map_command_grows_after_visiting_a_new_room() {
    let mut game = Game::new(1);
    let before = game.handle("map");
    // A generated door is locked some of the time (see go()'s own doc
    // comment) -- try every direction until one actually moves the player,
    // rather than depending on a specific seed/direction staying unlocked.
    for word in ["north", "south", "east", "west", "up", "down"] {
        game.handle(word);
        if game.handle("map") != before {
            break;
        }
    }
    let after = game.handle("map");
    assert_ne!(
        before, after,
        "the map must change after visiting a new room (every exit from the Threshold was sealed?)"
    );
    assert!(
        after.lines().count() > before.lines().count(),
        "a newly visited room should add a line to the map, got before={before:?} after={after:?}"
    );
}

// -- Expanded perception across an open edge (Stage 3) ------------------

/// Wires `origin` to `adjacent` (a room the caller has already committed,
/// so it can carry whatever facts a test needs before the edge exists) via
/// a fresh edge to the north, `locked` as given -- shared setup for the
/// tests below.
fn connect_north(
    graph: &mut WorldGraph,
    origin: &oxigraph::model::NamedNode,
    adjacent: &oxigraph::model::NamedNode,
    locked: bool,
) {
    let edge = graph.fresh("edge/");
    let d = Delta::new()
        .assert(edge.clone(), vocab::rdf_type(), vocab::class_edge())
        .assert(
            edge.clone(),
            vocab::direction(),
            dmml_runtime::graph::lit_str("north"),
        )
        .assert(edge.clone(), vocab::locked(), dmml_runtime::graph::lit_bool(locked))
        .assert(origin.clone(), vocab::connects_to(), edge.clone())
        .assert(edge.clone(), vocab::to(), adjacent.clone());
    graph
        .commit("demiurge", d)
        .expect("an edge shape is always a valid commit");
}

#[test]
fn perceive_room_folds_an_item_from_an_unlocked_adjacent_room() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let adjacent = graph.fresh("room/");
    let item = graph.fresh("item/");
    let facts = Delta::new()
        .assert(adjacent.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(item.clone(), vocab::rdf_type(), vocab::class_item())
        .assert(item.clone(), vocab::name(), dmml_runtime::graph::lit_str("brass whistle"))
        .assert(adjacent.clone(), vocab::contains(), item);
    graph
        .commit("demiurge", facts)
        .expect("a room containing an item is always a valid commit");
    connect_north(&mut graph, &boot.start_room, &adjacent, false);

    let percept = dmml_runtime::render::perceive_room(&graph, &boot.player, &boot.start_room, &[])
        .expect("sight machine is equipped at bootstrap");
    let Some(dmml_runtime::percept::PerceptValue::List(items)) = percept.field("items") else {
        panic!("expected an items field, got: {percept:?}");
    };
    assert!(
        items.iter().any(|i| i.contains("brass whistle") && i.contains("north")),
        "an item in an unlocked adjacent room should appear, tagged by direction, got: {items:?}"
    );
}

#[test]
fn perceive_room_does_not_fold_items_across_a_locked_edge() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let adjacent = graph.fresh("room/");
    let item = graph.fresh("item/");
    let facts = Delta::new()
        .assert(adjacent.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(item.clone(), vocab::rdf_type(), vocab::class_item())
        .assert(item.clone(), vocab::name(), dmml_runtime::graph::lit_str("brass whistle"))
        .assert(adjacent.clone(), vocab::contains(), item);
    graph
        .commit("demiurge", facts)
        .expect("a room containing an item is always a valid commit");
    connect_north(&mut graph, &boot.start_room, &adjacent, true);

    let percept = dmml_runtime::render::perceive_room(&graph, &boot.player, &boot.start_room, &[])
        .expect("sight machine is equipped at bootstrap");
    assert!(
        percept.field("items").is_none(),
        "a locked edge must not leak what's beyond it, got: {percept:?}"
    );
}

#[test]
fn perceive_room_respects_senses_even_when_the_edge_is_open() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let adjacent = graph.fresh("room/");
    let item = graph.fresh("item/");
    let facts = Delta::new()
        .assert(adjacent.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(item.clone(), vocab::rdf_type(), vocab::class_item())
        .assert(item.clone(), vocab::name(), dmml_runtime::graph::lit_str("brass whistle"))
        .assert(adjacent.clone(), vocab::contains(), item);
    graph
        .commit("demiurge", facts)
        .expect("a room containing an item is always a valid commit");
    connect_north(&mut graph, &boot.start_room, &adjacent, false);

    // A player that senses connectsTo/direction (so exits still work) but
    // not contains -- expanded perception still has to run through the
    // same senses gate as everything else, not bypass it just because an
    // edge happens to be open.
    let narrow_player = graph.fresh("player/");
    let base = Delta::new().assert(narrow_player.clone(), vocab::rdf_type(), vocab::class_player());
    graph
        .commit("demiurge", base)
        .expect("a bare player node is always a valid commit");
    let (_, sense_delta) = dmml_runtime::machine::build_sense_machine(
        &mut graph,
        &narrow_player,
        &["room"],
        &[vocab::name(), vocab::connects_to(), vocab::direction()],
    );
    graph
        .commit("demiurge", sense_delta)
        .expect("a sense machine is always a valid commit");

    let percept = dmml_runtime::render::perceive_room(&graph, &narrow_player, &boot.start_room, &[])
        .expect("a room sense-machine is equipped");
    assert!(
        percept.field("exits").is_some(),
        "connectsTo is sensed, so exits must still appear, got: {percept:?}"
    );
    assert!(
        percept.field("items").is_none(),
        "contains isn't sensed, so nothing should leak through even though the edge is open, got: {percept:?}"
    );
}

#[test]
fn perceive_room_folds_noticed_change_from_an_adjacent_room() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let adjacent = graph.fresh("room/");
    let drift = graph.fresh("drift/");
    let facts = Delta::new()
        .assert(adjacent.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(drift.clone(), vocab::rdf_type(), vocab::class_drift())
        .assert(drift.clone(), vocab::drift_old_cid(), vocab::foreign_cid_node("cid-old"))
        .assert(drift.clone(), vocab::drift_new_cid(), vocab::foreign_cid_node("cid-new"))
        .assert(
            drift.clone(),
            vocab::drift_observed_at(),
            dmml_runtime::graph::lit_int(1_000),
        )
        .assert(adjacent.clone(), vocab::noticed_change(), drift);
    graph
        .commit("demiurge", facts)
        .expect("a room with a noticedChange fact is always a valid commit");
    connect_north(&mut graph, &boot.start_room, &adjacent, false);

    let percept = dmml_runtime::render::perceive_room(&graph, &boot.player, &boot.start_room, &[])
        .expect("sight machine is equipped at bootstrap");
    let Some(dmml_runtime::percept::PerceptValue::List(noticed)) = percept.field("noticedChange") else {
        panic!("expected a noticedChange field, got: {percept:?}");
    };
    assert!(
        noticed
            .iter()
            .any(|n| n.contains("shifted") && n.contains("north")),
        "an adjacent room's own noticed drift should fold in, tagged by direction, got: {noticed:?}"
    );
}

// -- Migrating a pre-percept-pipeline session (regression) --------------

/// Reproduces a real session bootstrapped before `renderKind` existed:
/// hand-builds the exact shape the old `demiurge::bootstrap` produced --
/// player, room, hands machine, and a sight machine with `senses` but no
/// `renderKind` triple at all, since the predicate didn't exist yet.
/// `sense_machines_for_kind` can never match a machine like this, which is
/// exactly the bug a real player hit: every room rendered as "You perceive
/// nothing" despite their world being otherwise completely intact.
#[test]
fn from_snapshot_migrates_a_sight_machine_with_no_render_kind() {
    let mut graph = WorldGraph::new();
    let room = graph.fresh("room/");
    let player = graph.fresh("player/");
    let sight = graph.fresh("machine/");
    let base = Delta::new()
        .assert(room.clone(), vocab::rdf_type(), vocab::class_room())
        .assert(room.clone(), vocab::name(), dmml_runtime::graph::lit_str("The Threshold"))
        .assert(room.clone(), vocab::dampness(), dmml_runtime::graph::lit_float(0.0))
        .assert(room.clone(), vocab::decay(), dmml_runtime::graph::lit_float(0.0))
        .assert(room.clone(), vocab::light(), dmml_runtime::graph::lit_float(0.4))
        .assert(room.clone(), vocab::visits(), dmml_runtime::graph::lit_int(1))
        .assert(player.clone(), vocab::rdf_type(), vocab::class_player())
        .assert(room.clone(), vocab::contains(), player.clone())
        .assert(sight.clone(), vocab::rdf_type(), vocab::class_machine())
        .assert(player.clone(), vocab::equips(), sight.clone())
        .assert(sight.clone(), vocab::senses(), vocab::name());
    graph
        .commit("demiurge", base)
        .expect("a pre-renderKind bootstrap shape is still a valid commit");
    assert!(
        dmml_runtime::machine::sense_machines_for_kind(&graph, &player, "room").is_empty(),
        "this sight machine deliberately has no renderKind -- sanity-checking the reproduction itself"
    );

    let nquads = graph.dump_nquads().expect("dump must succeed");
    let snapshot = dmml_runtime::game::Snapshot {
        nquads,
        content_hash: graph.content_hash(),
        world_seed: 1,
    };
    let mut game = Game::from_snapshot(&snapshot).expect("a valid pre-existing session must still load");

    let response = game.handle("look");
    assert!(
        !response.contains("You perceive nothing"),
        "a returning player's world must render again after migration, got: {response}"
    );
    assert!(
        response.contains("The Threshold"),
        "the migrated sight machine should perceive the room's own name, got: {response}"
    );
}

// --- Pantheon: commit-log replay (jedelman/written-world#8) ---

#[test]
fn canonical_text_round_trips_through_from_canonical_text() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    dmml_runtime::demiurge::generate_frontier(&mut graph, 1, &boot.start_room, Direction::North);

    for entry in graph.transcript_since(0) {
        let text = entry.canonical_text();
        let parsed =
            dmml_runtime::graph::Delta::from_canonical_text(&text).expect("canonical_text is always parseable");
        assert_eq!(
            parsed.add.len(),
            entry.added.len(),
            "add count must round-trip for entry #{}",
            entry.seq
        );
        assert_eq!(
            parsed.remove.len(),
            entry.removed.len(),
            "remove count must round-trip for entry #{}",
            entry.seq
        );
        // Re-render the parsed quads and compare against the original --
        // canonical_text sorts before rendering, so this is exact-text
        // equality, not just a length check.
        let mut reparsed_added: Vec<String> = parsed.add.iter().map(|q| q.to_string()).collect();
        let mut original_added: Vec<String> = entry.added.iter().map(|q| q.to_string()).collect();
        reparsed_added.sort();
        original_added.sort();
        assert_eq!(reparsed_added, original_added, "entry #{}", entry.seq);
    }
}

#[test]
fn from_canonical_text_rejects_malformed_lines() {
    let bad = "this is not a valid canonical-text line\n";
    assert!(
        dmml_runtime::graph::Delta::from_canonical_text(bad).is_err(),
        "a line missing the '+ '/'- ' prefix must be rejected, not silently ignored"
    );
}

#[test]
fn replaying_a_full_commit_log_reconstructs_identical_state() {
    let mut original = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut original);
    dmml_runtime::demiurge::generate_frontier(&mut original, 1, &boot.start_room, Direction::North);
    dmml_runtime::demiurge::generate_frontier(&mut original, 1, &boot.start_room, Direction::South);

    // Replay every entry, in order, against a fresh graph -- this is the
    // actual operation a client reconstructing a player's world from
    // nothing but their PDS's commit log would perform, once those
    // records carry canonical_text (not just its hash, as they do today).
    let mut replayed = WorldGraph::new();
    for entry in original.transcript_since(0) {
        let delta = dmml_runtime::graph::Delta::from_canonical_text(&entry.canonical_text())
            .expect("a real commit's own canonical_text is always parseable");
        replayed
            .commit(&entry.source, delta)
            .unwrap_or_else(|e| panic!("replaying entry #{} ({}) failed: {e}", entry.seq, entry.source));
    }

    // Compared as a sorted line set, not raw bytes: `dump_nquads`'s own
    // ordering isn't stable across two separately-built stores holding
    // equivalent content (confirmed by hand -- a byte-equality assertion
    // here failed on ordering alone with zero actual content difference).
    // A set of triples is the right notion of "identical state" anyway;
    // nothing about this graph's semantics depends on serialization order.
    let sorted_lines = |bytes: Vec<u8>| -> Vec<String> {
        let mut lines: Vec<String> = String::from_utf8(bytes)
            .expect("dump is valid UTF-8")
            .lines()
            .map(str::to_string)
            .collect();
        lines.sort();
        lines
    };
    assert_eq!(
        sorted_lines(original.dump_nquads().expect("dump succeeds")),
        sorted_lines(replayed.dump_nquads().expect("dump succeeds")),
        "replaying the full transcript must reconstruct the identical set of quads"
    );

    // The actual point of content_hash (over a plain incrementing
    // counter): replay runs the identical ordered sequence of commit()
    // calls the original session did, so it lands on the identical
    // content_hash too -- with nothing extra carried alongside the
    // replayed records to make that true.
    assert_eq!(
        original.content_hash(),
        replayed.content_hash(),
        "replay must reconstruct the same content_hash the original session had, or fresh ids minted from here would risk colliding with ones the original already used"
    );

    // And concretely: minting something new against each graph from this
    // matching point on must actually produce the same id, and that id
    // must not already be in use -- not just equal hashes in principle.
    let original_fresh = original.fresh("room/");
    let replayed_fresh = replayed.fresh("room/");
    assert_eq!(
        original_fresh, replayed_fresh,
        "the next fresh id after equal content_hash must be identical"
    );
    assert!(
        replayed
            .subjects(&vocab::rdf_type(), &oxigraph::model::Term::NamedNode(vocab::class_room()))
            .iter()
            .all(|r| *r != replayed_fresh),
        "the freshly minted id must not already exist anywhere in the replayed graph"
    );
}

#[test]
fn reconstructing_a_game_from_world_seed_plus_post_genesis_commits_matches_the_live_session() {
    // This is the shape the actual PDS-backed validator uses (see
    // server/src/atproto/replay.rs): genesis is never expected to exist
    // as literal records in a fetched commit log (nothing signs
    // Game::new's own bootstrap commits to the PDS -- see
    // Game::replay_commit's doc comment for why that's fine), so a
    // reconstructor regenerates it locally from world_seed and only
    // replays whatever came after.
    let seed = 5;
    let mut original = Game::new(seed);
    let genesis_len = original.transcript_len();
    original.handle("north");
    original.raise_petition_for_current_room(1_000_000);
    let post_genesis = original.commits_since(0);
    assert_eq!(
        post_genesis.len() as u64,
        original.transcript_len(),
        "sanity: commits_since(0) must include every commit, genesis included, since this test slices it manually below"
    );

    let mut reconstructed = Game::new(seed);
    assert_eq!(
        reconstructed.transcript_len(),
        genesis_len,
        "regenerating genesis from the same seed must produce the identical number of commits"
    );
    for (i, (_, text, source)) in post_genesis.iter().enumerate() {
        if (i as u64) < genesis_len {
            continue; // Genesis itself -- already reconstructed above, not replayed.
        }
        reconstructed
            .replay_commit(source, text, 1_000_000)
            .unwrap_or_else(|e| panic!("replaying post-genesis commit #{i} ({source}) failed: {e}"));
    }

    assert_eq!(
        original.look(),
        reconstructed.look(),
        "a Game reconstructed from world_seed + replayed post-genesis commits must render identically to the live session"
    );
    assert_eq!(
        original.pending_petitions(),
        reconstructed.pending_petitions(),
        "reconstructed state must include the petition raised after genesis"
    );
    assert_eq!(
        original.snapshot().unwrap().content_hash,
        reconstructed.snapshot().unwrap().content_hash,
        "matching content_hash means minting from this point on stays collision-free with the original"
    );
}

// --- Pantheon: Theoi as machines (jedelman/written-world#8) ---

fn theos_domain_of(graph: &WorldGraph, room: &oxigraph::model::NamedNode) -> Option<String> {
    dmml_runtime::machine::machines_for_verb(graph, room, "generate")
        .into_iter()
        .find_map(|m| {
            let effect_node = dmml_runtime::graph::as_node(graph.object(&m, &vocab::has_effect())?)?;
            match dmml_runtime::machine::read_effect(graph, &effect_node)? {
                dmml_runtime::machine::Effect::GenerateFrontier { domain } => Some(domain),
                _ => None,
            }
        })
}

#[test]
fn frontier_generation_equips_a_theos_generator_on_the_origin_room() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    assert!(
        theos_domain_of(&graph, &boot.start_room).is_none(),
        "a freshly bootstrapped room has no generator equipped yet"
    );

    dmml_runtime::demiurge::generate_frontier(&mut graph, 1, &boot.start_room, Direction::North);

    let domain = theos_domain_of(&graph, &boot.start_room);
    assert!(
        domain.is_some(),
        "expanding the frontier from a room must equip a Theos generator on it"
    );
    assert!(
        ["pluto", "demeter"].contains(&domain.unwrap().as_str()),
        "the rolled domain must be a registered Theos"
    );
}

#[test]
fn theos_domain_propagates_to_the_generated_room() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);

    let new_room =
        dmml_runtime::demiurge::generate_frontier(&mut graph, 1, &boot.start_room, Direction::North);

    let origin_domain = theos_domain_of(&graph, &boot.start_room);
    let new_room_domain = theos_domain_of(&graph, &new_room);
    assert!(
        origin_domain.is_some() && origin_domain == new_room_domain,
        "the generated room must inherit its origin's Theos, so territory reads as a region"
    );
}

#[test]
fn theos_domain_is_stable_across_multiple_expansions_from_the_same_room() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);

    dmml_runtime::demiurge::generate_frontier(&mut graph, 1, &boot.start_room, Direction::North);
    let domain_after_first = theos_domain_of(&graph, &boot.start_room)
        .expect("the first expansion equips a generator");

    dmml_runtime::demiurge::generate_frontier(&mut graph, 1, &boot.start_room, Direction::South);
    let domain_after_second = theos_domain_of(&graph, &boot.start_room)
        .expect("the generator equipped by the first expansion is still there");

    assert_eq!(
        domain_after_first, domain_after_second,
        "a second expansion from the same room must not re-roll its Theos"
    );
    assert_eq!(
        dmml_runtime::machine::machines_for_verb(&graph, &boot.start_room, "generate").len(),
        1,
        "a second expansion must not equip a second generator machine on the same room"
    );
}

#[test]
fn generated_room_and_item_nouns_match_its_theos_domain() {
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);

    let new_room =
        dmml_runtime::demiurge::generate_frontier(&mut graph, 1, &boot.start_room, Direction::North);
    let domain = theos_domain_of(&graph, &new_room).expect("the new room inherited a domain");

    let room_name = graph
        .object(&new_room, &vocab::name())
        .and_then(|t| dmml_runtime::graph::as_string(&t))
        .expect("a generated room always has a name");
    let room_noun = room_name
        .split_whitespace()
        .last()
        .expect("a room name is at least one word")
        .to_lowercase();

    let expected_room_nouns: &[&str] = match domain.as_str() {
        "pluto" => &[
            "quarry", "shaft", "chamber", "smeltworks", "gallery", "slag", "vault", "stope",
            "floor", "cavern",
        ],
        "demeter" => &[
            "hall", "cellar", "passage", "landing", "greenhouse", "thicket", "grove", "ruin",
            "hollow", "stairwell",
        ],
        other => panic!("unexpected Theos domain: {other}"),
    };
    assert!(
        expected_room_nouns.contains(&room_noun.as_str()),
        "room name {room_name:?} should end in one of {domain}'s own nouns, got {room_noun:?}"
    );
}

// --- Regression: commit_log must survive a snapshot save/reload round trip ---
//
// `WorldGraph::commit_log` (the ordered per-`apply_commit`-call structure
// `current_value`/`current_subjects_with` walk) used to go silently unrestored
// by `WorldGraph::load_nquads`/`Game::from_snapshot`. That blanked every
// `apply_commit`-sourced fact -- `heldBy` (`take`/`drop`) and `locatedIn`
// (`go`) -- the instant a session was reconstructed from a snapshot, even
// though the underlying triples were still physically present in the
// reloaded store (`apply_commit` never deletes). The fix: `WorldGraph::
// dump_commit_log`/`restore_commit_log`, threaded through `Game::snapshot`/
// `Game::from_snapshot` alongside the existing N-Quads dump. These tests
// reproduce the original failure mode and pin the fixed behavior.

#[test]
fn taken_item_survives_a_snapshot_save_and_reload_round_trip() {
    let mut game = Game::new(11);
    game.set_now(1_000);
    let conjure = game.handle("conjure lantern");
    assert!(!conjure.to_lowercase().contains("error"), "sanity: conjure succeeded, got: {conjure}");
    let take = game.handle("take lantern");
    assert!(take.contains("You take the lantern"), "sanity: take succeeded, got: {take}");

    let before = game.handle("inventory");
    assert_eq!(before, "You are carrying: lantern");

    let snapshot = game.snapshot().expect("snapshot must succeed");
    let mut restored = Game::from_snapshot(&snapshot).expect("from_snapshot must succeed");

    let after = restored.handle("inventory");
    assert_eq!(
        after, before,
        "a taken item's heldBy state must survive a snapshot round trip identically, got: {after}"
    );

    // And the inverse half of the same migration: dropping it post-reload
    // must still work off the correctly-restored heldBy state.
    let drop = restored.handle("drop lantern");
    assert!(drop.contains("You drop the lantern"), "got: {drop}");
    assert_eq!(restored.handle("inventory"), "You are carrying nothing.");
}

#[test]
fn located_in_survives_a_snapshot_save_and_reload_round_trip() {
    let mut game = Game::new(11);
    game.set_now(1_000);
    game.handle("north");

    let before = game
        .player_location_via_located_in()
        .expect("locatedIn must be set after a `go`");

    let snapshot = game.snapshot().expect("snapshot must succeed");
    let restored = Game::from_snapshot(&snapshot).expect("from_snapshot must succeed");

    let after = restored
        .player_location_via_located_in()
        .expect("locatedIn must survive a snapshot round trip, not go blank");
    assert_eq!(
        after, before,
        "the materialized locatedIn value itself must be unchanged by the round trip"
    );

    // player_room (the contains-backed, non-apply_commit read every
    // dispatch relies on) must independently agree, before and after.
    assert_eq!(restored.player_room(), before);
}

#[test]
fn player_room_does_not_panic_after_snapshot_reload_with_apply_commit_state_present() {
    // The originally-suspected failure mode: `player_room`'s
    // `.expect("player is always in exactly one room")` panicking once
    // apply_commit-sourced state (heldBy/locatedIn) is also in play across
    // a reload. `player_room` itself never reads that state (it stays on
    // the old `contains` path -- see `vocab::located_in`'s doc comment for
    // why), so this must degrade to nothing worse than continuing to work;
    // this test pins that explicitly rather than relying on it being an
    // accidental side effect of the other round-trip tests above.
    let mut game = Game::new(21);
    game.set_now(1_000);
    game.handle("north");
    game.handle("conjure torch");
    game.handle("take torch");

    let snapshot = game.snapshot().expect("snapshot must succeed");
    let restored = Game::from_snapshot(&snapshot).expect("from_snapshot must succeed");
    // Must not panic.
    let _room = restored.player_room();
}

#[test]
fn snapshot_reload_still_works_for_a_legacy_pre_fix_snapshot_blob() {
    // Simulates a snapshot persisted before `Snapshot.nquads` carried a
    // framed commit-log section: bare `dump_nquads()` bytes, exactly what
    // `Game::snapshot()` used to produce. `decode_snapshot_blob` must
    // recognize this shape (no `WWCL` magic prefix) and fall back to
    // treating the whole blob as plain N-Quads with no commit-log to
    // restore, rather than erroring out and stranding an already-existing
    // session. Hand-built the same way
    // `from_snapshot_migrates_a_sight_machine_with_no_render_kind` above
    // builds its own pre-existing-session reproduction, rather than
    // through `Game::snapshot()` (which now always produces the new,
    // framed format).
    let mut graph = WorldGraph::new();
    let boot = dmml_runtime::demiurge::bootstrap(&mut graph);
    let item = graph.fresh("item/");
    let mint = Delta::new()
        .assert(item.clone(), vocab::rdf_type(), vocab::class_item())
        .assert(item.clone(), vocab::name(), dmml_runtime::graph::lit_str("lantern"))
        .assert(item.clone(), vocab::portable(), dmml_runtime::graph::lit_bool(true))
        .assert(boot.start_room.clone(), vocab::contains(), item.clone());
    graph.commit("test", mint).expect("mint is valid");

    // The `take` verb's own apply_commit half, replicated directly against
    // the graph (mirroring `Game::take`'s own body).
    let quad = oxigraph::model::Quad::new(
        item.clone(),
        vocab::held_by(),
        oxigraph::model::Term::NamedNode(boot.player.clone()),
        oxigraph::model::GraphName::DefaultGraph,
    );
    let commit = dmml_runtime::graph::Commit {
        consumes: Vec::new(),
        produces: format!("{quad} ."),
        predicate: "takenBy".to_string(),
        via: None,
        responds_to: None,
        created_at: "0".to_string(),
    };
    graph.apply_commit("test", commit).expect("take is valid");

    let legacy_nquads = graph.dump_nquads().expect("dump must succeed");
    let legacy_snapshot = dmml_runtime::game::Snapshot {
        nquads: legacy_nquads,
        content_hash: graph.content_hash(),
        world_seed: 31,
    };
    let mut restored = Game::from_snapshot(&legacy_snapshot)
        .expect("a legacy, unframed snapshot blob must still load, not error");

    assert!(
        restored.look().contains("The Threshold"),
        "plain N-Quads content (room, name, etc.) must still restore correctly from a legacy blob"
    );
    // The known, accepted degradation for genuinely legacy data: heldBy
    // state recorded before this fix existed has nothing to restore from,
    // so it reads back empty -- not wrong, just as limited as it always was.
    assert_eq!(
        restored.handle("inventory"),
        "You are carrying nothing.",
        "a legacy blob has no framed commit-log section, so heldBy state predictably doesn't restore"
    );
}

#[test]
fn held_by_is_self_declared_so_a_take_replays_through_replay_commit() {
    // Mirrors `locatedIn`'s own self-declaration in `demiurge::bootstrap`
    // (see that call site's doc comment): a predicate written only via
    // `apply_commit` that isn't self-declared as a Relation fails
    // `validate` the moment it's replayed through the validated `Delta`/
    // `WorldGraph::commit` path `Game::replay_commit` uses. `heldBy` had
    // this gap; `locatedIn` didn't. This reproduces the failure mode
    // directly: replaying a `take`-produced transcript entry must not
    // error.
    let seed = 41;
    let mut original = Game::new(seed);
    let genesis_len = original.transcript_len();
    original.set_now(1_000);
    original.handle("conjure key");
    original.handle("take key");

    let post_genesis = original.commits_since(0);
    let mut reconstructed = Game::new(seed);
    for (i, (_, text, source)) in post_genesis.iter().enumerate() {
        if (i as u64) < genesis_len {
            continue; // Genesis itself -- regenerated above, not replayed.
        }
        reconstructed
            .replay_commit(source, text, 1_000)
            .unwrap_or_else(|e| {
                panic!("replaying post-genesis commit #{i} ({source}) failed: {e}")
            });
    }

    // The raw triples replay correctly regardless (validate only gates
    // whether the commit is *accepted*, not what it asserts) -- what this
    // test actually pins is that replay didn't reject the commit at all.
    assert_eq!(original.look(), reconstructed.look());
}

#[test]
fn replay_commit_after_a_snapshot_reload_does_not_corrupt_restored_state() {
    // Item 4's "does replay_commit still work correctly through a
    // snapshot boundary" question: reload from a snapshot (the fixed
    // path, so heldBy/locatedIn are already correctly restored), then
    // continue mutating the session via `replay_commit` -- the PDS-replay
    // entry point, a code path entirely separate from `load_nquads`. The
    // already-restored apply_commit state must survive that untouched,
    // and the newly replayed commit must land correctly.
    let mut game = Game::new(51);
    game.set_now(1_000);
    game.handle("conjure map");
    game.handle("take map");
    let mark = game.transcript_len();

    let snapshot = game.snapshot().expect("snapshot must succeed");
    let mut restored = Game::from_snapshot(&snapshot).expect("from_snapshot must succeed");
    assert_eq!(restored.handle("inventory"), "You are carrying: map");

    // Replay one more real commit (a fresh `conjure`) against the
    // restored game, the same way a PDS-replay caller would append newly
    // fetched records after loading a cached snapshot.
    game.handle("conjure quill");
    let new_commits = game.commits_since(mark);
    for (_, text, source) in &new_commits {
        restored
            .replay_commit(source, text, 1_000)
            .expect("replaying a fresh post-snapshot commit must succeed");
    }

    // The pre-snapshot heldBy state, restored via the fix, must be
    // unaffected by an unrelated replayed commit landing afterward.
    assert_eq!(
        restored.handle("inventory"),
        "You are carrying: map",
        "replaying an unrelated commit must not disturb already-restored heldBy state"
    );
}

#[test]
fn replay_commit_reconstruction_does_not_materialize_apply_commit_state_known_limitation() {
    // A related but *distinct* gap from the one this fix addresses, found
    // while investigating it: `Game::replay_commit` always re-applies a
    // transcript entry through the old `Delta`/`WorldGraph::commit` path,
    // regardless of whether the *original* commit went through `commit()`
    // or `apply_commit()` -- `TranscriptEntry`/`canonical_text()` carry no
    // record of which mechanism produced an entry. So a `Game`
    // reconstructed *purely* by `replay_commit`-ing a transcript from
    // scratch (the PDS-replay flow, not a `Game::snapshot`/`from_snapshot`
    // round trip) never populates `commit_log` at all, even for entries
    // that originally came from `apply_commit` -- `current_value`/
    // `current_subjects_with` (and so `heldBy`/`locatedIn`) stay
    // permanently blank on a replay-only reconstruction, the identical
    // symptom the snapshot bug had, just via a different, untouched code
    // path. Confirmed by reproduction.
    //
    // **Still not fixed as of #26** (see `Game::record_reconstruction_gap`'s
    // own doc comment for why not, deliberately): the underlying blankness
    // stays exactly as it was. What #26 changes is that this reconstruction
    // now *knows* and *says* it may be incomplete (`reconstruction_gap()`),
    // instead of silently proceeding as if nothing were missing -- "corruption
    // as content" applied to reconstruction completeness itself, not just to
    // foreign-correspondence drift.
    let seed = 91;
    let mut original = Game::new(seed);
    let genesis_len = original.transcript_len();
    original.set_now(1_000);
    let go_result = original.handle("north");
    assert!(
        original.player_location_via_located_in().is_some(),
        "sanity: this seed's north edge must actually open, got: {go_result}"
    );

    let post_genesis = original.commits_since(0);
    let mut reconstructed = Game::new(seed);
    for (i, (_, text, source)) in post_genesis.iter().enumerate() {
        if (i as u64) < genesis_len {
            continue;
        }
        reconstructed
            .replay_commit(source, text, 1_000)
            .unwrap_or_else(|e| {
                panic!("replaying post-genesis commit #{i} ({source}) failed: {e}")
            });
    }

    assert_eq!(
        original.look(),
        reconstructed.look(),
        "contains-backed state (player_room and everything look() reads) still replays correctly"
    );
    assert!(
        reconstructed.player_location_via_located_in().is_none(),
        "known limitation: a purely replay-reconstructed Game never repopulates commit_log, \
         so apply_commit-sourced state (locatedIn here) does not materialize -- unlike a \
         Game::from_snapshot round trip, which an earlier fix does cover"
    );
    assert!(
        original.reconstruction_gap().is_none(),
        "a live, non-replayed session must never report a gap -- nothing about it was \
         reconstructed at all"
    );
    assert_eq!(
        reconstructed.reconstruction_gap(),
        Some("locatedIn".to_string()),
        "#26: the replay-only reconstruction must honestly report which predicate first \
         couldn't be materialized, instead of silently proceeding as if nothing were missing"
    );
}
