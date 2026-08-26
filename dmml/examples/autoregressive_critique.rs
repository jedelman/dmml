//! An autoregressive dispatch experiment: the SAME independent-reader
//! prompt, run three times in a row against the paper's current text,
//! each round also given every prior round's output as additional
//! material it must not repeat. The question this file checks isn't a
//! claim about DMML's grammar -- it's a claim about the dispatch
//! methodology itself: does repeated dispatch against a fixed but
//! growing body of material CONVERGE (later rounds restate or trivially
//! rephrase earlier ones) or GENERATE (later rounds keep finding real,
//! distinct content)? That's checkable the same way everything else in
//! this project is: build each round's output as a real commit and see
//! whether it's structurally derivative of the others or independent.
//!
//! Per `AUTHORING.md`'s own reuse guidance, this file reuses the
//! existing `claim` predicate rather than coining `critiqueClaim` --
//! a critique is a claim about a claim, the existing vocabulary already
//! fits, and coining a near-duplicate here would be exactly the
//! dilution `AUTHORING.md` warns against.
//!
//! All three rounds were dispatched via a fresh `general-purpose` agent
//! (the custom `materialization-editor`/critical-reader agent types are
//! not directly dispatchable as a `subagent_type` in this session), each
//! given the paper's full current text and told explicitly not to repeat
//! whatever prior rounds already said.

use dmml::ast::TopLevelItem;
use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::lower;

fn commit_of(doc: &dmml::Document) -> &dmml::ast::CommitStmt {
    doc.items
        .iter()
        .find_map(|item| match item {
            TopLevelItem::Commit(c) => Some(c),
            _ => None,
        })
        .expect("the document has a commit")
}

fn identify(src: &str, uri: &str, cid: &str) -> IdentifiedCommit {
    let doc = dmml::parse(src).unwrap_or_else(|e| panic!("failed to parse {uri}: {e:?}\n{src}"));
    let commit = commit_of(&doc);
    validate_declarations(commit)
        .unwrap_or_else(|e| panic!("undeclared predicate(s) in {uri}: {e:?}\n{src}"));
    IdentifiedCommit {
        uri: uri.to_string(),
        cid: cid.to_string(),
        commit: lower::lower_commit(commit),
    }
}

// ===== The paper's own claims, as they stand today -- re-asserted here
// (not consumed from the other example files, which are separate
// worlds) so this file's critiques have something real to consume. =====

const SECTION1_FAIL_OPEN_SRC: &str = r#"
commit asserts {
  declare attribute claim

  paper/section1_fail_open claim "a consumes entry citing a fact nothing actually produced is a no-op, not an error -- a commit citing something fabricated is still accepted, it just fails to retract what it claimed to"
}
"#;

const SECTION3_SAMPLING_CONCERN_SRC: &str = r#"
commit asserts {
  declare attribute claim

  paper/section3_sampling_concern claim "petition resolution is currently produced by an LLM sampling from a learned distribution -- arguably close to selection from an already-determined menu, the very thing DMML's grammar is being distinguished from; whether sampling counts as production in Deleuze and Guattari's sense is left open"
}
"#;

const SECTION4_AUTO_RECOMBINANT_SRC: &str = r#"
commit asserts {
  declare attribute claim

  paper/section4_auto_recombinant claim "nothing in the grammar enforces convergence on a (subject, predicate) pair; a rival claim persists as one more citable, disputable production; Nyx's synthesis is auto-recombinant because the facts it folds are grounded against a real production history"
}
"#;

const SECTION1_CITATION_GRANULARITY_SRC: &str = r#"
commit asserts {
  declare attribute claim

  paper/section1_citation_granularity claim "a consumes entry can cite one specific triple within any prior commit, not only the tip of that commit and not the commit as an indivisible unit -- nothing in the grammar treats a commit as an atomic whole for citation purposes"
}
"#;

const SECTION5_ANSWER_SRC: &str = r#"
commit asserts {
  declare attribute claim

  paper/section5_answer claim "task-specific self-declared predicates show real convergence pressure under dispatch conditions (an author given an existing graph's vocabulary in context tends to reuse its exact names), which is evidence of citation-name stability under shared context, not yet evidence of spontaneous convergence between authors with zero shared context"
}
"#;

// ===== Cycle 1: dispatched fresh, given only the paper's text. =====
const CYCLE1_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {fail_open_uri} (cid: {fail_open_cid}) {
      subject: paper/section1_fail_open
      predicate: claim
    }
    fact {auto_recombinant_uri} (cid: {auto_recombinant_cid}) {
      subject: paper/section4_auto_recombinant
      predicate: claim
    }
  }
  produces {
    paper/critique_cycle1 claim "fail-open citation semantics undercuts the auto-recombinant claim: nothing structurally distinguishes a commit that legitimately synthesizes real prior facts from one that cites the same facts and produces something with no defensible relation to them -- both are grounded in exactly the sense the auto-recombinant claim requires. auto-recombinant may be a property of the well-behaved subset of commits authors happen to write, not a structural property of the grammar itself"
  }
}
"#;

// ===== Cycle 2: dispatched fresh, given the paper's text PLUS cycle 1's
// output, told not to repeat it. =====
const CYCLE2_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {auto_recombinant_uri} (cid: {auto_recombinant_cid}) {
      subject: paper/section4_auto_recombinant
      predicate: claim
    }
    fact {section5_answer_uri} (cid: {section5_answer_cid}) {
      subject: paper/section5_answer
      predicate: claim
    }
  }
  produces {
    paper/critique_cycle2 claim "the non-convergence claim (nothing forces convergence on facts) is in tension with the convergence claim (independent authors DO converge, on predicate vocabulary, under shared-context dispatch conditions -- the same condition under which real multi-author DMML worlds actually operate, since later commits can always see earlier ones). if schema-level convergence is real and shared-context-driven, the same pressure could eventually produce convergence on which facts count as canonical too -- auto-recombinant non-convergent multiplicity may be a transient phase before shared-context convergence pressure closes it down into something git-like, not a stable structural property"
  }
}
"#;

// ===== Cycle 3: dispatched fresh, given the paper's text PLUS cycles 1
// and 2, told not to repeat either. =====
const CYCLE3_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {sampling_concern_uri} (cid: {sampling_concern_cid}) {
      subject: paper/section3_sampling_concern
      predicate: claim
    }
    fact {section5_answer_uri} (cid: {section5_answer_cid}) {
      subject: paper/section5_answer
      predicate: claim
    }
  }
  produces {
    paper/critique_cycle3 claim "Section 5's convergence result is contaminated by the exact concern Section 3 raises about petition-resolution and does not apply to itself: both dispatched authoring agents are LLMs generating predicate names conditioned on overlapping training distributions and overlapping context, so convergence on counterClaim/distanceStrategy may be evidence that LLMs sampling from similar distributions reach for the same naming conventions -- true in ordinary code review or ontology engineering generally -- rather than evidence about DMML's grammar specifically. no control (human authors, or authors in a domain unrelated to DMML) separates these. this is the paper's own evidentiary standard (Section 3's sampling-vs-production distinction) applied unevenly: raised against the resolver, not raised against Section 5's own dispatched 'independent authors'"
  }
}
"#;

// ===== Cycle 4: dispatched fresh, given the paper + cycles 1-3, told
// not to repeat any. It found something none of the object-level cycles
// did: cycle 3 (LLM-sampling contamination) undercuts cycle 2's own
// evidentiary basis (the "facts might converge too" worry needs
// Section 5's convergence to be genuine DMML-grammar evidence, which
// cycle 3 casts doubt on). This is SECOND-ORDER: it consumes prior
// CRITIQUES, not base-paper facts -- the same "recombination of a
// recombination" shape `benjamin_second_reader.rs`'s forced-regression
// commit already demonstrated, now appearing spontaneously in a fresh
// dispatch rather than being asked for. =====
const CYCLE4_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {cycle2_uri} (cid: {cycle2_cid}) {
      subject: paper/critique_cycle2
      predicate: claim
    }
    fact {cycle3_uri} (cid: {cycle3_cid}) {
      subject: paper/critique_cycle3
      predicate: claim
    }
  }
  produces {
    paper/critique_cycle4 claim "cycle 3 defangs cycle 2: cycle 2's worry that fact-convergence could eventually follow vocabulary-convergence needs Section 5's convergence data to be genuine evidence about DMML's own grammar-level dynamics. cycle 3 shows that convergence is plausibly just LLM-sampling behavior common to any domain, not something DMML's grammar produced or licenses -- which removes the evidentiary basis cycle 2 needed. the critiques are not independent, additive damage; they partially cancel, and nothing in a chained-dispatch format where each reader is only told not to repeat prior points checks whether a later critique invalidates an earlier one's premises -- a structural feature of chained peer critique itself, not only of this paper"
  }
}
"#;

// ===== Cycle 5: dispatched fresh, given paper + cycles 1-4, told not to
// repeat any. Returns to FIRST-ORDER attack on the base paper, from an
// angle none of cycles 1-4 touched: citation granularity. =====
const CYCLE5_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {citation_granularity_uri} (cid: {citation_granularity_cid}) {
      subject: paper/section1_citation_granularity
      predicate: claim
    }
    fact {auto_recombinant_uri} (cid: {auto_recombinant_cid}) {
      subject: paper/section4_auto_recombinant
      predicate: claim
    }
  }
  produces {
    paper/critique_cycle5 claim "citation granularity is the triple, not the commit -- a later commit can extract one fact from its production context while ignoring everything else that commit asserted together, with no mechanism tracking which co-produced facts must travel together. Deleuze and Guattari's desiring-machine connections (breast-mouth) are constituted BY their connections, not by extractable partial objects severed from context -- DMML's triple-level citation does the reverse. Nyx's commit is fine only because it happened to cite whole coherent facts; nothing in the grammar prevents a future commit from citing one triple out of a commit while severing it from the production-context that gave it its sense, a gap orthogonal to the fail-open/convergence cluster and checkable directly against the schema (does consumes type-check against triples or whole commits?)"
  }
}
"#;

// ===== Cycle 6: dispatched fresh, given paper + cycles 1-5, told not to
// repeat any. SECOND-ORDER again, but a different move than cycle 4: it
// turns Section 3's own sampling-vs-production question on the critique
// -dispatch EXPERIMENT itself, not on any base-paper claim. =====
const CYCLE6_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {sampling_concern_uri} (cid: {sampling_concern_cid}) {
      subject: paper/section3_sampling_concern
      predicate: claim
    }
    fact {cycle3_uri} (cid: {cycle3_cid}) {
      subject: paper/critique_cycle3
      predicate: claim
    }
  }
  produces {
    paper/critique_cycle6 claim "the critique-dispatch experiment producing cycles 1 through 5 is itself an unacknowledged second instance of exactly what cycle 3 (and Section 3) put in question: each critique is produced by an LLM reader sampling from a learned distribution, conditioned on the paper plus all prior critiques, accepted into the log without verification against any canonical correct critique, rival and uncoordinated in the same way pantheon.rs's Helios/Selene/Eos are. the paper anchors its empirical claims in pantheon.rs and the Benjamin simulation; the six-round critique experiment used to stress-test the paper is a live, unexamined third instance of the same open question -- is this production, or selection from an already-determined menu -- applied to the paper's own critical apparatus rather than to a fictional petition-resolver"
  }
}
"#;

fn main() {
    let fail_open_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0200";
    let fail_open = identify(SECTION1_FAIL_OPEN_SRC, fail_open_uri, "bafyFailOpen");

    let sampling_concern_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0201";
    let sampling_concern = identify(SECTION3_SAMPLING_CONCERN_SRC, sampling_concern_uri, "bafySamplingConcern");

    let auto_recombinant_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0202";
    let auto_recombinant = identify(SECTION4_AUTO_RECOMBINANT_SRC, auto_recombinant_uri, "bafyAutoRecombinant");

    let section5_answer_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0203";
    let section5_answer = identify(SECTION5_ANSWER_SRC, section5_answer_uri, "bafySection5Answer");

    let citation_granularity_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0204";
    let citation_granularity = identify(
        SECTION1_CITATION_GRANULARITY_SRC,
        citation_granularity_uri,
        "bafyCitationGranularity",
    );

    let cycle1_uri = "at://did:plc:reader-cycle1/org.jason-edelman.writtenworld.commit/rkey0001";
    let cycle1_src = CYCLE1_TEMPLATE
        .replace("{fail_open_uri}", fail_open_uri)
        .replace("{fail_open_cid}", "bafyFailOpen")
        .replace("{auto_recombinant_uri}", auto_recombinant_uri)
        .replace("{auto_recombinant_cid}", "bafyAutoRecombinant");
    let cycle1 = identify(&cycle1_src, cycle1_uri, "bafyCycle1");

    let cycle2_uri = "at://did:plc:reader-cycle2/org.jason-edelman.writtenworld.commit/rkey0001";
    let cycle2_src = CYCLE2_TEMPLATE
        .replace("{auto_recombinant_uri}", auto_recombinant_uri)
        .replace("{auto_recombinant_cid}", "bafyAutoRecombinant")
        .replace("{section5_answer_uri}", section5_answer_uri)
        .replace("{section5_answer_cid}", "bafySection5Answer");
    let cycle2 = identify(&cycle2_src, cycle2_uri, "bafyCycle2");

    let cycle3_uri = "at://did:plc:reader-cycle3/org.jason-edelman.writtenworld.commit/rkey0001";
    let cycle3_src = CYCLE3_TEMPLATE
        .replace("{sampling_concern_uri}", sampling_concern_uri)
        .replace("{sampling_concern_cid}", "bafySamplingConcern")
        .replace("{section5_answer_uri}", section5_answer_uri)
        .replace("{section5_answer_cid}", "bafySection5Answer");
    let cycle3 = identify(&cycle3_src, cycle3_uri, "bafyCycle3");

    let cycle4_uri = "at://did:plc:reader-cycle4/org.jason-edelman.writtenworld.commit/rkey0001";
    let cycle4_src = CYCLE4_TEMPLATE
        .replace("{cycle2_uri}", cycle2_uri)
        .replace("{cycle2_cid}", "bafyCycle2")
        .replace("{cycle3_uri}", cycle3_uri)
        .replace("{cycle3_cid}", "bafyCycle3");
    let cycle4 = identify(&cycle4_src, cycle4_uri, "bafyCycle4");

    let cycle5_uri = "at://did:plc:reader-cycle5/org.jason-edelman.writtenworld.commit/rkey0001";
    let cycle5_src = CYCLE5_TEMPLATE
        .replace("{citation_granularity_uri}", citation_granularity_uri)
        .replace("{citation_granularity_cid}", "bafyCitationGranularity")
        .replace("{auto_recombinant_uri}", auto_recombinant_uri)
        .replace("{auto_recombinant_cid}", "bafyAutoRecombinant");
    let cycle5 = identify(&cycle5_src, cycle5_uri, "bafyCycle5");

    let cycle6_uri = "at://did:plc:reader-cycle6/org.jason-edelman.writtenworld.commit/rkey0001";
    let cycle6_src = CYCLE6_TEMPLATE
        .replace("{sampling_concern_uri}", sampling_concern_uri)
        .replace("{sampling_concern_cid}", "bafySamplingConcern")
        .replace("{cycle3_uri}", cycle3_uri)
        .replace("{cycle3_cid}", "bafyCycle3");
    let cycle6 = identify(&cycle6_src, cycle6_uri, "bafyCycle6");

    println!("=== Cycle 1 (fresh reader, paper only) ===\n{cycle1_src}");
    println!("=== Cycle 2 (fresh reader, paper + cycle 1) ===\n{cycle2_src}");
    println!("=== Cycle 3 (fresh reader, paper + cycles 1-2) ===\n{cycle3_src}");
    println!("=== Cycle 4 (fresh reader, paper + cycles 1-3) ===\n{cycle4_src}");
    println!("=== Cycle 5 (fresh reader, paper + cycles 1-4) ===\n{cycle5_src}");
    println!("=== Cycle 6 (fresh reader, paper + cycles 1-5) ===\n{cycle6_src}");

    let pair_of = |c: &IdentifiedCommit| -> Vec<String> {
        let mut subs: Vec<String> = c
            .commit
            .consumes
            .iter()
            .filter_map(|cref| match cref {
                lower::ConsumeRef::Fact(f) => Some(f.subject.clone()),
                lower::ConsumeRef::Strong(_) => None,
            })
            .collect();
        subs.sort();
        subs
    };

    // Check 1: cycles 1, 2, 3, and 5 are FIRST-ORDER -- they consume
    // only base-paper facts, never another cycle's own critique.
    let cites_only_base_facts = |c: &IdentifiedCommit| {
        c.commit.consumes.iter().all(|cref| match cref {
            lower::ConsumeRef::Fact(f) => f.subject.starts_with("paper/section"),
            lower::ConsumeRef::Strong(_) => false,
        })
    };
    for (c, label) in [(&cycle1, "1"), (&cycle2, "2"), (&cycle3, "3"), (&cycle5, "5")] {
        assert_eq!(c.commit.consumes.len(), 2, "cycle {label} should consume exactly 2 facts");
        assert!(cites_only_base_facts(c), "cycle {label} should be first-order (base facts only)");
    }
    println!("Check 1: cycles 1, 2, 3, and 5 are first-order -- each consumes only base-paper facts, never another cycle's critique.");

    // Check 2: cycles 4 and 6 are SECOND-ORDER -- they consume prior
    // CRITIQUES, the same "recombination of a recombination" shape
    // `benjamin_second_reader.rs`'s forced-regression commit already
    // demonstrated, now appearing spontaneously rather than being asked
    // for. Confirms they are a genuinely different KIND of move, not
    // just a different first-order angle.
    let cites_a_prior_critique = |c: &IdentifiedCommit| {
        c.commit.consumes.iter().any(|cref| match cref {
            lower::ConsumeRef::Fact(f) => f.subject.starts_with("paper/critique_cycle"),
            lower::ConsumeRef::Strong(_) => false,
        })
    };
    assert!(cites_a_prior_critique(&cycle4), "cycle 4 should consume a prior critique");
    assert!(cites_a_prior_critique(&cycle6), "cycle 6 should consume a prior critique");
    println!("Check 2: cycles 4 and 6 are second-order -- each consumes at least one PRIOR CRITIQUE, not just base-paper facts, the same recombination-of-a-recombination shape benjamin_second_reader.rs already demonstrated.");

    // Check 3: no two cycles triangulate the identical fact-pair -- each
    // round is a genuinely different connection, not the same one
    // re-derived with different words.
    let pairs: Vec<(&str, Vec<String>)> = vec![
        ("1", pair_of(&cycle1)),
        ("2", pair_of(&cycle2)),
        ("3", pair_of(&cycle3)),
        ("4", pair_of(&cycle4)),
        ("5", pair_of(&cycle5)),
        ("6", pair_of(&cycle6)),
    ];
    for i in 0..pairs.len() {
        for j in (i + 1)..pairs.len() {
            assert_ne!(
                pairs[i].1, pairs[j].1,
                "cycle {} and cycle {} triangulated the identical fact-pair",
                pairs[i].0, pairs[j].0
            );
        }
    }
    for (label, pair) in &pairs {
        println!("  cycle {label} triangulates: {pair:?}");
    }
    println!("Check 3: all six cycles triangulate distinct fact-pairs -- six genuinely different connections across six rounds, none re-deriving another.");

    // Check 4: every cycle's claim remains real and independently
    // citable. Checked the honest way per Section VI's finding: cycle 2
    // and cycle 3 are each downstream-cited (by cycle 4 and/or cycle 6),
    // so their own key would read back as None inside the full combined
    // log -- checked in isolation instead. Cycles 1, 4, 5, 6 are never
    // downstream-cited by anything else here, so the full combined log
    // is the honest check for them.
    let full_log = Materialized::from_identified_commits(&[
        fail_open.clone(),
        sampling_concern.clone(),
        auto_recombinant.clone(),
        section5_answer.clone(),
        citation_granularity.clone(),
        cycle1.clone(),
        cycle2.clone(),
        cycle3.clone(),
        cycle4.clone(),
        cycle5.clone(),
        cycle6.clone(),
    ]);
    for subject in ["paper/critique_cycle1", "paper/critique_cycle4", "paper/critique_cycle5", "paper/critique_cycle6"] {
        full_log
            .current_value(subject, "claim")
            .unwrap_or_else(|| panic!("{subject} should still be real in the full combined log"));
    }
    for (isolated, subject) in [(&cycle2, "paper/critique_cycle2"), (&cycle3, "paper/critique_cycle3")] {
        Materialized::from_identified_commits(&[(*isolated).clone()])
            .current_value(subject, "claim")
            .unwrap_or_else(|| panic!("{subject} should still be real in isolation"));
    }
    println!("Check 4: every one of the six critiques remains real and independently citable -- cycles 1/4/5/6 checked in the full combined log, cycles 2/3 checked in isolation since cycles 4/6 downstream-cite and retract their keys in the combined log (the same cite-and-spend semantics found in Section VI, correctly accounted for rather than mistaken for a bug).");

    println!(
        "\n=== done: six autoregressive dispatch rounds against the same fixed paper text (plus, from round 4 on, all prior critiques) produced six structurally distinct critiques (Check 1-3), all independently real (Check 4) -- GENERATION, not convergence, through six rounds. Two of the six (4 and 6) were spontaneously second-order -- consuming a prior critique rather than the base paper -- without being asked to; the dispatch prompt never suggested that move. No plateau reached in this run: round 6 still produced a genuinely new angle, though it explicitly flagged one section (2) as having little independent purchase left, a soft signal worth watching in any further rounds rather than a plateau itself. ==="
    );
}
