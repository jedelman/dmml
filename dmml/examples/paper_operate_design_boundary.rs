//! Outline-first, in DMML itself, same pattern as the retired-but-real
//! precedent `paper_predicate_convergence.rs` (2026-08-26, git history
//! only now -- deleted in 45e2c44 along with the whole hand-written text
//! DSL it depended on, `dmml::parse`; this file rebuilds the same
//! pattern on the current, sole JSON authoring surface,
//! `dmml::from_json::update_from_json`). No prior prose exists for this
//! argument -- it is built as a commit graph first, and only the LAST
//! commit produces a prose paragraph, as its own fact, consuming the
//! graph that licenses it.
//!
//! The argument: today's Round 4 (`VALAR-EVAL-2026-08-30.md`,
//! 2026-08-30) and Round 5 (`available_actions.rs`, 2026-08-31) findings
//! -- structure closing two real gaps a schema built from prose alone
//! left open -- are not just software-engineering results. They are a
//! structure-preserving correspondence between process ontology
//! (Deleuze & Guattari's desiring-machines: connective synthesis,
//! desire-as-production, the body without organs) and computational
//! linguistics (DMML's grammar: guards, `may_fire`,
//! `commit_fires_transition`, the materialized world-graph). Jason's own
//! correction, stated directly and recorded verbatim rather than
//! paraphrased away: "it's not an argument about can and can't - it's
//! the construction of a homeomorphism between process ontology and
//! computational linguistics." That correction IS this file's actual
//! thesis, not a gloss on it.
//!
//! Honesty checks this file is built to fail loudly if they don't hold,
//! same discipline as the retired precedent:
//! 1. The Round 5 numbers (how many of the seed world's declared
//!    transitions are legal right now) are NOT copied from
//!    `available_actions.rs`'s prior run -- they're recomputed live,
//!    right here, against the same seed world and the same
//!    `dmml::machine::may_fire` primitive.
//! 2. The Round 4 numbers are NOT retyped from memory -- this file reads
//!    `VALAR-EVAL-2026-08-30.md` off disk and asserts the exact
//!    convergence figures ("0/5", "1/1") are actually present in the
//!    committed record before citing them.
//! 3. The correspondence claim is stated, then immediately confounded --
//!    "homeomorphism" is this paper's chosen term for a
//!    structure-preserving mapping, not a completed topological proof;
//!    the confound commit says exactly what is and isn't established,
//!    rather than letting the strong word imply more rigor than exists.
//! 4. The final prose is read back via `Materialized::current_value` on
//!    the full folded history, not composed separately in Rust and
//!    printed alongside an unconnected graph.
//!
//! Run with `cargo run -p dmml --example paper_operate_design_boundary`.

use dmml::from_json::update_from_json;
use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::lower::{lower_commit, TripleValue};
use dmml::machine::{self, EvalContext, MachineBody};
use dmml::validate::validate_declarations;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn identify(commit_json: &str, uri: &str, cid: &str) -> IdentifiedCommit {
    let update = update_from_json(commit_json).unwrap_or_else(|e| panic!("invalid commit JSON for {uri}: {e}\n{commit_json}"));
    let commit = &update.batches[0].commits[0];
    validate_declarations(commit).unwrap_or_else(|e| panic!("undeclared predicate(s) in {uri}: {e:?}\n{commit_json}"));
    IdentifiedCommit {
        uri: uri.to_string(),
        cid: cid.to_string(),
        commit: lower_commit(commit),
    }
}

fn str_value(v: &TripleValue) -> &str {
    match v {
        TripleValue::Str(s) => s.as_str(),
        other => panic!("expected a string value, found {other:?}"),
    }
}

// ===== Real, computed Round 5 evidence: the exact same seed world and
// enumeration `available_actions.rs` uses, run fresh right here so the
// numbers this file cites are never stale copies. =====

fn house_world_seed_json() -> &'static str {
    r#"{
  "update": [
    {
      "machines": [
        {
          "node": "Valinor",
          "states": [{"ident": "unformed"}, {"ident": "hills"}, {"ident": "mountains"}],
          "transitions": [
            {"ident": "raise", "from": "unformed", "to": "hills"},
            {"ident": "uplift", "from": "hills", "to": "mountains"}
          ]
        },
        {
          "node": "Valinor/quarry",
          "states": [{"ident": "untouched"}, {"ident": "stone"}, {"ident": "sand"}, {"ident": "clay"}, {"ident": "brick"}],
          "transitions": [
            {"ident": "quarry", "from": "untouched", "to": "stone",
              "guards": [{"exists": {"anchor": {"kind": "node", "value": "Valinor"},
                "hops": [{"predicate": "state", "term": {"kind": "node", "value": "mountains"}}]}}]},
            {"ident": "grind", "from": "stone", "to": "sand"},
            {"ident": "wet", "from": "sand", "to": "clay"},
            {"ident": "fire", "from": "clay", "to": "brick"}
          ]
        },
        {
          "node": "Valinor/streambed",
          "states": [{"ident": "bare"}, {"ident": "sand"}],
          "transitions": [{"ident": "wash", "from": "bare", "to": "sand"}]
        },
        {
          "node": "Valinor/spring",
          "states": [{"ident": "dry"}, {"ident": "flowing"}],
          "transitions": [{"ident": "well_up", "from": "dry", "to": "flowing"}]
        },
        {
          "node": "Valinor/mortar",
          "states": [{"ident": "unmixed"}, {"ident": "mixed"}],
          "transitions": [
            {"ident": "mix", "params": ["sand_source", "water_source"], "from": "unmixed", "to": "mixed",
              "guards": [
                {"exists": {"anchor": {"kind": "param", "value": "sand_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "sand"}}]}},
                {"exists": {"anchor": {"kind": "param", "value": "water_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "flowing"}}]}}
              ]}
          ]
        },
        {
          "node": "Valinor/wall",
          "states": [{"ident": "unbuilt"}, {"ident": "built"}],
          "transitions": [
            {"ident": "build", "params": ["brick_source", "mortar_source"], "from": "unbuilt", "to": "built",
              "guards": [
                {"exists": {"anchor": {"kind": "param", "value": "brick_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "brick"}}]}},
                {"exists": {"anchor": {"kind": "param", "value": "mortar_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "mixed"}}]}}
              ]}
          ]
        },
        {
          "node": "Valinor/forest",
          "states": [{"ident": "full"}, {"ident": "thinned"}, {"ident": "depleted"}],
          "transitions": [
            {"ident": "gather", "from": "full", "to": "thinned"},
            {"ident": "overgather", "from": "thinned", "to": "depleted"}
          ]
        },
        {
          "node": "Valinor/carpentry",
          "states": [{"ident": "no_frame"}, {"ident": "framed"}],
          "transitions": [
            {"ident": "make_frame", "from": "no_frame", "to": "framed",
              "guards": [{"negated": true, "exists": {"anchor": {"kind": "node", "value": "Valinor/forest"},
                "hops": [{"predicate": "state", "term": {"kind": "node", "value": "depleted"}}]}}]}
          ]
        },
        {
          "node": "Valinor/roof",
          "states": [{"ident": "unroofed"}, {"ident": "roofed"}],
          "transitions": [
            {"ident": "add_roof", "params": ["wall_source", "frame_source"], "from": "unroofed", "to": "roofed",
              "guards": [
                {"exists": {"anchor": {"kind": "param", "value": "wall_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "built"}}]}},
                {"exists": {"anchor": {"kind": "param", "value": "frame_source"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "framed"}}]}}
              ]}
          ]
        },
        {
          "node": "Valinor/house",
          "states": [{"ident": "unbuilt"}, {"ident": "built"}],
          "transitions": [
            {"ident": "construct_house", "from": "unbuilt", "to": "built",
              "guards": [
                {"exists": {"anchor": {"kind": "node", "value": "Valinor/wall"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "built"}}]}},
                {"exists": {"anchor": {"kind": "node", "value": "Valinor/roof"},
                  "hops": [{"predicate": "state", "term": {"kind": "node", "value": "roofed"}}]}}
              ]}
          ]
        }
      ],
      "commits": [
        {
          "verb": "mints",
          "declares": [{"kind": "attribute", "name": "state"}],
          "facts": [
            {"subject": "Valinor", "predicate": "state", "object": {"kind": "node", "value": "unformed"}},
            {"subject": "Valinor/quarry", "predicate": "state", "object": {"kind": "node", "value": "untouched"}},
            {"subject": "Valinor/streambed", "predicate": "state", "object": {"kind": "node", "value": "bare"}},
            {"subject": "Valinor/spring", "predicate": "state", "object": {"kind": "node", "value": "dry"}},
            {"subject": "Valinor/mortar", "predicate": "state", "object": {"kind": "node", "value": "unmixed"}},
            {"subject": "Valinor/wall", "predicate": "state", "object": {"kind": "node", "value": "unbuilt"}},
            {"subject": "Valinor/forest", "predicate": "state", "object": {"kind": "node", "value": "full"}},
            {"subject": "Valinor/carpentry", "predicate": "state", "object": {"kind": "node", "value": "no_frame"}},
            {"subject": "Valinor/roof", "predicate": "state", "object": {"kind": "node", "value": "unroofed"}},
            {"subject": "Valinor/house", "predicate": "state", "object": {"kind": "node", "value": "unbuilt"}}
          ]
        }
      ]
    }
  ]
}"#
}

fn known_nodes(world: &Materialized) -> Vec<String> {
    let mut nodes: Vec<String> = world
        .iter()
        .filter(|(_, pred, _)| *pred == "state")
        .map(|(subj, _, _)| subj.to_string())
        .collect();
    nodes.sort();
    nodes.dedup();
    nodes
}

fn param_bindings(param_names: &[String], candidates: &[String]) -> Vec<HashMap<String, String>> {
    if param_names.is_empty() {
        return vec![HashMap::new()];
    }
    let mut results = vec![HashMap::new()];
    for name in param_names {
        let mut next = Vec::new();
        for existing in &results {
            for candidate in candidates {
                let mut binding = existing.clone();
                binding.insert(name.clone(), candidate.clone());
                next.push(binding);
            }
        }
        results = next;
    }
    results
}

/// Recomputes, live, exactly what `available_actions.rs` computes:
/// (legal-right-now count, total declared transitions).
fn recompute_round5_numbers() -> (usize, usize) {
    let update = update_from_json(house_world_seed_json()).expect("seed JSON is valid DMML");
    let mut machines: HashMap<String, MachineBody> = HashMap::new();
    for batch in &update.batches {
        for m in &batch.machines {
            machines.insert(m.node.segments.join("/"), MachineBody { states: m.states.clone(), transitions: m.transitions.clone() });
        }
    }
    let history: Vec<_> = update.batches.iter().flat_map(|b| b.commits.iter().map(lower_commit)).collect();
    let world = Materialized::from_commits(&history);
    let nodes = known_nodes(&world);

    let mut legal = 0usize;
    let mut total = 0usize;
    for (node, body) in &machines {
        for decl in &body.transitions {
            total += 1;
            if decl.params.is_empty() {
                let ctx = EvalContext { self_node: node.clone(), params: HashMap::new() };
                if machine::may_fire(body, &decl.ident, &ctx, &world) == Some(true) {
                    legal += 1;
                }
            } else {
                for binding in param_bindings(&decl.params, &nodes) {
                    let ctx = EvalContext { self_node: node.clone(), params: binding };
                    if machine::may_fire(body, &decl.ident, &ctx, &world) == Some(true) {
                        legal += 1;
                    }
                }
            }
        }
    }
    (legal, total)
}

/// Real file I/O, not a remembered figure: confirms the Round 4
/// convergence numbers this argument cites actually appear in the
/// committed eval writeup before building a fact out of them.
fn confirm_round4_numbers_on_disk() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../papers/desiring-production-ontology/VALAR-EVAL-2026-08-30.md");
    let contents = fs::read_to_string(&path).unwrap_or_else(|e| panic!("could not read {path:?}: {e}"));
    assert!(contents.contains("0/5 convergence"), "VALAR-EVAL-2026-08-30.md no longer states the 0/5 figure this argument cites");
    assert!(contents.contains("1/1 one-shot"), "VALAR-EVAL-2026-08-30.md no longer states the 1/1 figure this argument cites");
}

fn main() {
    confirm_round4_numbers_on_disk();
    let (legal_now, total_declared) = recompute_round5_numbers();
    assert_eq!((legal_now, total_declared), (5, 15), "Round 5's seed-world numbers changed -- update the cited figures, don't silently drift");

    // ===== Step 1: the process-ontology claim, asserted as it stands. =====
    let process_ontology_json = r#"{"update": [{"commits": [{
        "verb": "asserts",
        "declares": [{"kind": "attribute", "name": "claim"}],
        "facts": [{"subject": "paper/desiring_machines", "predicate": "claim", "object": {"kind": "str",
          "value": "a desiring-machine is defined by connective synthesis: what it can couple to, and whether that coupling produces a flow or interrupts it -- desire as production, not as lack seeking an object, and never reducible to the symbolic order it runs through"}}]
    }]}]}"#;
    let process_ontology = identify(process_ontology_json, "at://did:example:dmml-paper/world.paper/step-1", "step-1-cid");

    // ===== Step 2: the computational-linguistics claim, asserted independently. =====
    let computational_linguistics_json = r#"{"update": [{"commits": [{
        "verb": "asserts",
        "declares": [{"kind": "attribute", "name": "claim"}],
        "facts": [{"subject": "paper/dmml_grammar", "predicate": "claim", "object": {"kind": "str",
          "value": "a DMML transition is defined by its guard: whether the referenced nodes' current state satisfies it, checked by dmml::machine::may_fire against the materialized world-graph -- a bounded, decidable membership question, not an open-ended judgment"}}]
    }]}]}"#;
    let computational_linguistics = identify(computational_linguistics_json, "at://did:example:dmml-paper/world.paper/step-2", "step-2-cid");

    // ===== Step 3: Round 5's live-recomputed evidence, citing step 2. =====
    let round5_json = format!(
        r#"{{"update": [{{"commits": [{{
        "verb": "reproduces",
        "declares": [{{"kind": "attribute", "name": "claim"}}],
        "consumes": [{{"kind": "fact", "commit": {{"uri": "{}", "cid": "{}"}}, "subject": "paper/dmml_grammar", "predicate": "claim"}}],
        "facts": [{{"subject": "paper/round5_evidence", "predicate": "claim", "object": {{"kind": "str",
          "value": "recomputed live, right here, against the real house-world seed: {legal_now} of {total_declared} declared transitions are legal right now, per dmml::machine::may_fire -- when the operate-tier schema was built from this computed set instead of a hand-typed catalog, a real prior failure (Valinor/quarry :: quarry, structurally valid but GuardNotSatisfied) became unrepresentable, and the model's next pick fired for real"}}}}]
    }}]}}]}}"#,
        computational_linguistics.uri, computational_linguistics.cid
    );
    let round5_evidence = identify(&round5_json, "at://did:example:dmml-paper/world.paper/step-3", "step-3-cid");

    // ===== Step 4: Round 4's on-disk-confirmed evidence, citing step 1. =====
    let round4_json = format!(
        r#"{{"update": [{{"commits": [{{
        "verb": "reproduces",
        "declares": [{{"kind": "attribute", "name": "claim"}}],
        "consumes": [{{"kind": "fact", "commit": {{"uri": "{}", "cid": "{}"}}, "subject": "paper/desiring_machines", "predicate": "claim"}}],
        "facts": [{{"subject": "paper/round4_evidence", "predicate": "claim", "object": {{"kind": "str",
          "value": "confirmed on disk in VALAR-EVAL-2026-08-30.md: the same model at the same reasoning effort moved from 0/5 convergence (prose-only has_content constraint, 5 rounds of explicit corrective feedback, never landing) to 1/1 one-shot convergence, purely by relocating has_content out of prose and into the schema's own anyOf-of-branches structure"}}}}]
    }}]}}]}}"#,
        process_ontology.uri, process_ontology.cid
    );
    let round4_evidence = identify(&round4_json, "at://did:example:dmml-paper/world.paper/step-4", "step-4-cid");

    // ===== Step 5: the correspondence itself -- three mapped pairs, then
    // the claim that the mapping is structure-preserving, not decorative. =====
    let correspondence_json = format!(
        r#"{{"update": [{{"commits": [{{
        "verb": "constructs",
        "declares": [{{"kind": "attribute", "name": "claim"}}, {{"kind": "attribute", "name": "maps_to"}}],
        "consumes": [
          {{"kind": "fact", "commit": {{"uri": "{r5_uri}", "cid": "{r5_cid}"}}, "subject": "paper/round5_evidence", "predicate": "claim"}},
          {{"kind": "fact", "commit": {{"uri": "{r4_uri}", "cid": "{r4_cid}"}}, "subject": "paper/round4_evidence", "predicate": "claim"}}
        ],
        "facts": [
          {{"subject": "paper/correspondence/coupling", "predicate": "maps_to", "object": {{"kind": "str",
            "value": "a desiring-machine's coupling (does this connect, does a flow pass or get interrupted) corresponds to a DMML transition's guard, evaluated by may_fire against live world state"}}}},
          {{"subject": "paper/correspondence/production", "predicate": "maps_to", "object": {{"kind": "str",
            "value": "desire as production (a new synthesis nothing in the prior structure implied) corresponds to the design tier: proposing a machine or transition no existing schema enumerates, fenceable by structure but not generated by it"}}}},
          {{"subject": "paper/correspondence/body_without_organs", "predicate": "maps_to", "object": {{"kind": "str",
            "value": "the body without organs, the real surface a proposed coupling is checked against rather than merely asserted onto, corresponds to the materialized world-graph and commit_fires_transition -- the ground truth every schema is only ever a projection of"}}}},
          {{"subject": "paper/correspondence", "predicate": "claim", "object": {{"kind": "str",
            "value": "these three pairs are not analogies but a structure-preserving correspondence: composing operations on the process-ontology side (couple-or-not, then produce, checked against the body without organs) matches composing their counterparts on the computational-linguistics side (may_fire, then design-tier proposal, checked by commit_fires_transition) -- the same composition order holds on both sides, term for term"}}}}
        ]
    }}]}}]}}"#,
        r5_uri = round5_evidence.uri, r5_cid = round5_evidence.cid,
        r4_uri = round4_evidence.uri, r4_cid = round4_evidence.cid,
    );
    let correspondence = identify(&correspondence_json, "at://did:example:dmml-paper/world.paper/step-5", "step-5-cid");

    // ===== Step 6: the confound -- named honestly, not smoothed over. =====
    let confound_json = format!(
        r#"{{"update": [{{"commits": [{{
        "verb": "qualifies",
        "declares": [{{"kind": "attribute", "name": "claim"}}],
        "consumes": [{{"kind": "fact", "commit": {{"uri": "{}", "cid": "{}"}}, "subject": "paper/correspondence", "predicate": "claim"}}],
        "facts": [{{"subject": "paper/correspondence_confound", "predicate": "claim", "object": {{"kind": "str",
          "value": "homeomorphism is this paper's chosen term for the mapping, not a completed proof: no topology has been placed on either domain, and no continuous bijection with continuous inverse has been formally constructed. what is actually established is narrower and real -- two independent rounds (well-formedness, then guard-legality) where the identical fix (relocate a constraint from prose into structure) closed the identical class of gap on the process side and the linguistic side alike, plus a term-by-term correspondence that respects composition under informal inspection. the formal category-theoretic construction remains open work, named here rather than assumed"}}}}]
    }}]}}]}}"#,
        correspondence.uri, correspondence.cid
    );
    let confound = identify(&confound_json, "at://did:example:dmml-paper/world.paper/step-6", "step-6-cid");

    // ===== Step 7: the thesis itself, produced as prose, consuming the
    // graph that licenses it -- not composed separately and pasted in. =====
    let history_so_far = [
        process_ontology.clone(), computational_linguistics.clone(),
        round5_evidence.clone(), round4_evidence.clone(),
        correspondence.clone(), confound.clone(),
    ];

    // The "cite-and-spend" gotcha, same one already logged in
    // dev-journal/2026-08-26-outline-first-prose-as-commit.md: each
    // downstream commit CONSUMES the fact it cites, which retracts it
    // from any materialization folded past that point. So every fact the
    // final prose needs has to be read from an isolated re-materialization
    // taken BEFORE the commit that consumes it, not from one shared view.
    let materialized_before_correspondence = Materialized::from_identified_commits(&history_so_far[..4]);
    let round5_claim = str_value(materialized_before_correspondence.current_value("paper/round5_evidence", "claim").expect("round5 evidence missing before correspondence even runs"));
    let round4_claim = str_value(materialized_before_correspondence.current_value("paper/round4_evidence", "claim").expect("round4 evidence missing before correspondence even runs"));

    let materialized_before_confound = Materialized::from_identified_commits(&history_so_far[..5]);
    let correspondence_claim = str_value(materialized_before_confound.current_value("paper/correspondence", "claim").expect("correspondence claim missing before confound even runs -- that's a real bug, not the cite-and-spend gotcha"));

    let materialized_so_far = Materialized::from_identified_commits(&history_so_far);
    let confound_claim = str_value(materialized_so_far.current_value("paper/correspondence_confound", "claim").expect("confound retracted before it could be cited"));

    let thesis_prose = format!(
        "The operate/design boundary this session's Round 4 and Round 5 closed \
is not a software-engineering nicety layered on top of DMML's ontology paper -- \
it is that paper's central claim, demonstrated mechanically. {correspondence_claim}. \
{confound_claim}. The evidence behind both sides of that mapping is real, not asserted: \
on the linguistic side, {round5_claim}. On the process-ontology side, {round4_claim}."
    );

    let thesis_json = format!(
        r#"{{"update": [{{"commits": [{{
        "verb": "states",
        "declares": [{{"kind": "attribute", "name": "prose"}}],
        "consumes": [
          {{"kind": "fact", "commit": {{"uri": "{corr_uri}", "cid": "{corr_cid}"}}, "subject": "paper/correspondence", "predicate": "claim"}},
          {{"kind": "fact", "commit": {{"uri": "{cf_uri}", "cid": "{cf_cid}"}}, "subject": "paper/correspondence_confound", "predicate": "claim"}},
          {{"kind": "fact", "commit": {{"uri": "{r5_uri}", "cid": "{r5_cid}"}}, "subject": "paper/round5_evidence", "predicate": "claim"}},
          {{"kind": "fact", "commit": {{"uri": "{r4_uri}", "cid": "{r4_cid}"}}, "subject": "paper/round4_evidence", "predicate": "claim"}}
        ],
        "facts": [{{"subject": "paper/thesis", "predicate": "prose", "object": {{"kind": "str", "value": {thesis_json_str}}}}}]
    }}]}}]}}"#,
        corr_uri = correspondence.uri, corr_cid = correspondence.cid,
        cf_uri = confound.uri, cf_cid = confound.cid,
        r5_uri = round5_evidence.uri, r5_cid = round5_evidence.cid,
        r4_uri = round4_evidence.uri, r4_cid = round4_evidence.cid,
        thesis_json_str = serde_json::to_string(&thesis_prose).unwrap(),
    );
    let thesis = identify(&thesis_json, "at://did:example:dmml-paper/world.paper/step-7", "step-7-cid");

    let full_history: Vec<IdentifiedCommit> = history_so_far.into_iter().chain([thesis]).collect();
    let full_world = Materialized::from_identified_commits(&full_history);

    let produced_prose = str_value(full_world.current_value("paper/thesis", "prose").expect("thesis prose missing from the final materialized view"));
    assert_eq!(produced_prose, thesis_prose, "the fact read back out of the graph must be byte-identical to what was actually consumed to build it");
    assert!(produced_prose.contains(&format!("{legal_now} of {total_declared}")), "thesis prose lost its own live-computed Round 5 figures somewhere in the graph");
    assert!(produced_prose.contains("0/5") && produced_prose.contains("1/1"), "thesis prose lost its own on-disk-confirmed Round 4 figures somewhere in the graph");

    println!("=== The argument's dependency graph, {} commits, none hand-pasted into the final prose ===\n", full_history.len());
    for c in &full_history {
        println!("  {} ({})", c.uri, c.commit.predicate_verb);
    }

    println!("\n=== The thesis, materialized from the graph via Materialized::current_value, not composed separately ===\n");
    println!("{produced_prose}");

    println!(
        "\n(Round 5 figures recomputed live this run: {legal_now}/{total_declared}. \
Round 4 figures confirmed present on disk this run, not retyped from memory.)"
    );
}
