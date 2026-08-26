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

fn main() {
    let fail_open_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0200";
    let fail_open = identify(SECTION1_FAIL_OPEN_SRC, fail_open_uri, "bafyFailOpen");

    let sampling_concern_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0201";
    let sampling_concern = identify(SECTION3_SAMPLING_CONCERN_SRC, sampling_concern_uri, "bafySamplingConcern");

    let auto_recombinant_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0202";
    let auto_recombinant = identify(SECTION4_AUTO_RECOMBINANT_SRC, auto_recombinant_uri, "bafyAutoRecombinant");

    let section5_answer_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0203";
    let section5_answer = identify(SECTION5_ANSWER_SRC, section5_answer_uri, "bafySection5Answer");

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

    println!("=== Cycle 1 (fresh reader, paper only) ===\n{cycle1_src}");
    println!("=== Cycle 2 (fresh reader, paper + cycle 1) ===\n{cycle2_src}");
    println!("=== Cycle 3 (fresh reader, paper + cycles 1-2) ===\n{cycle3_src}");

    // Check 1: each cycle consumes exactly the base-paper facts it
    // actually engages with -- two each -- not a chain where later
    // cycles merely cite earlier cycles' critique (which would indicate
    // elaboration on a single thread rather than three independent
    // angles of attack).
    assert_eq!(cycle1.commit.consumes.len(), 2);
    assert_eq!(cycle2.commit.consumes.len(), 2);
    assert_eq!(cycle3.commit.consumes.len(), 2);
    let cites_only_base_facts = |c: &IdentifiedCommit| {
        c.commit.consumes.iter().all(|cref| match cref {
            lower::ConsumeRef::Fact(f) => {
                f.subject.starts_with("paper/section")
            }
            lower::ConsumeRef::Strong(_) => false,
        })
    };
    assert!(cites_only_base_facts(&cycle1));
    assert!(cites_only_base_facts(&cycle2));
    assert!(cites_only_base_facts(&cycle3));
    println!("Check 1: all three cycles consume ONLY base-paper facts (never each other) -- three independent angles of attack on the same fixed material, not one thread of elaboration mistaken for three.");

    // Check 2: no two cycles consume the identical fact-pair -- each
    // round is triangulating a genuinely different pair of claims, not
    // re-deriving the same connection with different words.
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
    let (p1, p2, p3) = (pair_of(&cycle1), pair_of(&cycle2), pair_of(&cycle3));
    assert_ne!(p1, p2);
    assert_ne!(p2, p3);
    assert_ne!(p1, p3);
    println!("Check 2: no two cycles triangulate the same pair of base facts -- {p1:?}, {p2:?}, {p3:?} are three distinct connections, not the same connection restated.");

    // Check 3: all three critiques remain real and independently
    // citable in the combined log -- checked the honest way, per
    // Section VI's finding: none of them consumes another cycle's own
    // output, so none is at risk of the cite-and-spend retraction that
    // would apply if one cycle's fact were downstream-cited by another.
    let materialized = Materialized::from_identified_commits(&[
        fail_open.clone(),
        sampling_concern.clone(),
        auto_recombinant.clone(),
        section5_answer.clone(),
        cycle1.clone(),
        cycle2.clone(),
        cycle3.clone(),
    ]);
    for (subject, cid_label) in [
        ("paper/critique_cycle1", "cycle1"),
        ("paper/critique_cycle2", "cycle2"),
        ("paper/critique_cycle3", "cycle3"),
    ] {
        let value = materialized
            .current_value(subject, "claim")
            .unwrap_or_else(|| panic!("{cid_label}'s claim should still be real in the combined log"));
        let _ = value;
    }
    println!("Check 3: all three critiques remain real and independently citable in the SAME combined log -- none retracted the others, since none consumes another cycle's output.");

    println!(
        "\n=== done: three autoregressive dispatch rounds against the same fixed paper text produced three structurally distinct critiques (Check 1-2), all independently real (Check 3) -- GENERATION, not convergence, for this run. A convergent run would have shown later cycles citing the same fact-pair as an earlier one, or citing an earlier cycle's own critique rather than the paper itself, i.e. narrowing rather than widening the angles of attack. Neither happened here. ==="
    );
}
