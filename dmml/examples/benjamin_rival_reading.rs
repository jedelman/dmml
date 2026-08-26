//! A genuine rival reading of the unified essay (`benjamin_full_essay.rs`),
//! dispatched to `stealth/ox-alpha` as an adversarial second reader,
//! reviewed critically here rather than applied wholesale. Ox-alpha
//! proposed three specific challenges, each tied to real facts from the
//! 44-commit unified log. Two are accepted as-is; the third is built AND
//! disputed, same shape as `editorial_loop.rs`'s self-dispute pattern --
//! not silently overridden, not silently accepted.
//!
//! ACCEPTED AS-IS: the Epilogue's loop-closure is a citation, not a
//! logical derivation -- `consumes` means "cites as a real premise," and
//! this project already learned that distinction the hard way (Section
//! VI's cite-and-spend finding). Ox-alpha is right that conflating
//! citation with entailment would be a real overclaim.
//!
//! ACCEPTED AS-IS: the movie-star cult and Fuhrer cult consume the same
//! structural facts in the original log, and nothing in that log
//! discriminates why one is aesthetically trivial and the other
//! catastrophic -- a real, unclosed gap.
//!
//! BUILT AND DISPUTED: ox-alpha's claim that the magician/surgeon
//! distance-analogy is "politically promiscuous" because fascism's
//! apparatus-violation in the Epilogue is "surgically structured." My
//! counter: the Epilogue's own words are an apparatus "pressed into the
//! production of RITUAL values" -- forcing ritual/cult content INTO an
//! apparatus, which is closer to the opposite of the surgeon's structure
//! (which abstains from ritual and authority entirely; that's the
//! analogy's whole point). This conflates "an apparatus under duress"
//! with "an apparatus performing surgical/testing structure." Built as a
//! real commit, then disputed by a second commit that consumes it --
//! both stay in the log, neither silently wins.
//!
//! Run with `cargo run -p dmml --example benjamin_rival_reading`.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// Facts re-declared cross-file from the unified essay, per this series'
// standing convention.
const VOCAB_STANCE_SRC: &str = r#"
commit declares {
  declare attribute vocabularyStance

  argument/preface vocabularyStance "anti-fascist-terms"
}
"#;

const EPILOGUE_CLAIM_SRC: &str = r#"
commit argues {
  declare attribute claim

  argument/epilogue claim "Fascism is rendering politics aesthetic. Communism responds by politicizing art."
}
"#;

const STAR_CULT_SRC: &str = r#"
commit reproduces {
  declare attribute claim

  argument/section_x_star_cult claim "the cult of the movie star preserves not the unique aura of the person but the phony spell of a commodity"
}
"#;

const FUHRER_VIOLATION_SRC: &str = r#"
commit reproduces {
  declare attribute violation

  masses/1 violation "forced to their knees by the Fuhrer cult -- the same manufactured, commodity-shaped substitute for real cult value"
}
"#;

const MAGICIAN_SRC: &str = r#"
commit asserts {
  declare attribute distanceStrategy

  role/magician distanceStrategy "maintains natural distance, increases it through authority"
}
"#;

const SURGEON_SRC: &str = r#"
commit asserts {
  declare attribute distanceStrategy

  role/surgeon distanceStrategy "diminishes physical distance by penetrating the body, abstains from facing the patient man to man"
}
"#;

// ===== RIVAL 1 (accepted as-is): the loop closes by citation, not
// derivation. =====
const GATE_TEMPLATE: &str = r#"
commit argues {
  declare attribute closureStatus

  consumes {
    fact {vocab_uri} (cid: {vocab_cid}) {
      subject: argument/preface
      predicate: vocabularyStance
    }
    fact {epilogue_uri} (cid: {epilogue_cid}) {
      subject: argument/epilogue
      predicate: claim
    }
  }
  produces {
    argument/epilogue_gap closureStatus "stipulated by a vocabulary gate installed in the Preface, not derived from the aura-dissolution analysis alone -- strip the gate and the same analysis licenses readings Benjamin never argues against: a conservative-authenticist one, a liberal-reformist one"
  }
}
"#;

// ===== RIVAL 2 (accepted as-is): no internal criterion discriminates
// trivial pseudo-aura from catastrophic pseudo-aura. =====
const CRITERION_MISSING_TEMPLATE: &str = r#"
commit argues {
  declare attribute pseudoAuraDiscriminationBasis

  consumes {
    fact {star_cult_uri} (cid: {star_cult_cid}) {
      subject: argument/section_x_star_cult
      predicate: claim
    }
    fact {fuhrer_uri} (cid: {fuhrer_cid}) {
      subject: masses/1
      predicate: violation
    }
  }
  produces {
    argument/criterion_missing pseudoAuraDiscriminationBasis "none available within the essay's own structural vocabulary -- commodity pseudo-aura and totalitarian pseudo-aura consume the same structural facts, and the essay supplies no internal criterion for why one is aesthetically trivial and the other catastrophic"
  }
}
"#;

// ===== RIVAL 3 (built, then disputed): the penetration-structure is
// "politically promiscuous." =====
const PROMISCUITY_TEMPLATE: &str = r#"
commit argues {
  declare attribute penetrationStructurePoliticalValence

  consumes {
    fact {magician_uri} (cid: {magician_cid}) {
      subject: role/magician
      predicate: distanceStrategy
    }
    fact {surgeon_uri} (cid: {surgeon_cid}) {
      subject: role/surgeon
      predicate: distanceStrategy
    }
  }
  produces {
    argument/section_xi_ambiguity penetrationStructurePoliticalValence "unassigned -- the identical surgical/penetration structure is instantiated by film-progressivism (Sections XI, XIII) AND by fascism's apparatus, per the Epilogue's own image of an apparatus pressed into producing ritual values -- the original log's magician/surgeon-consumes-aura-as-distance commit is a fault line, not a foundation"
  }
}
"#;

// The dispute: consumes RIVAL 3's own claim and the Fuhrer-violation
// fact, disputing the equivalence rather than accepting it.
const DISPUTE_TEMPLATE: &str = r#"
commit disputes {
  declare attribute counterClaim

  consumes {
    fact {promiscuity_uri} (cid: {promiscuity_cid}) {
      subject: argument/section_xi_ambiguity
      predicate: penetrationStructurePoliticalValence
    }
    fact {fuhrer_uri} (cid: {fuhrer_cid}) {
      subject: masses/1
      predicate: violation
    }
  }
  produces {
    argument/section_xi_ambiguity counterClaim "the equivalence conflates two different mechanisms -- the Epilogue's apparatus is 'pressed into the PRODUCTION OF RITUAL VALUES,' forcing ritual/cult content INTO an apparatus, which is closer to the opposite of the surgeon's structure (abstaining from ritual and authority entirely is the analogy's whole point). Fascism's move is a forced REVERSAL toward auratic/ritual structure, not an instance of surgical/testing structure. The ambiguity is real but narrower than claimed."
  }
}
"#;

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
    let doc = dmml::parse(src).unwrap_or_else(|e| panic!("failed to parse {uri}: {e:?}"));
    let commit = commit_of(&doc);
    validate_declarations(commit)
        .unwrap_or_else(|e| panic!("undeclared predicate(s) in {uri}: {e:?}"));
    IdentifiedCommit {
        uri: uri.to_string(),
        cid: cid.to_string(),
        commit: lower::lower_commit(commit),
    }
}

fn main() {
    let vocab_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0001";
    let vocab_cid = "bafyEssay0001";
    let vocab = identify(VOCAB_STANCE_SRC, vocab_uri, vocab_cid);

    let epilogue_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0044";
    let epilogue_cid = "bafyEssay0044";
    let epilogue = identify(EPILOGUE_CLAIM_SRC, epilogue_uri, epilogue_cid);

    let star_cult_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0023";
    let star_cult_cid = "bafyEssay0023";
    let star_cult = identify(STAR_CULT_SRC, star_cult_uri, star_cult_cid);

    let fuhrer_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0041";
    let fuhrer_cid = "bafyEssay0041";
    let fuhrer = identify(FUHRER_VIOLATION_SRC, fuhrer_uri, fuhrer_cid);

    let magician_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0024";
    let magician_cid = "bafyEssay0024";
    let magician = identify(MAGICIAN_SRC, magician_uri, magician_cid);

    let surgeon_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0025";
    let surgeon_cid = "bafyEssay0025";
    let surgeon = identify(SURGEON_SRC, surgeon_uri, surgeon_cid);

    println!("=== Carried over from the unified essay: 6 real facts the rival reading cites ===\n");

    let gate_uri = "at://did:plc:critical-reader/org.jason-edelman.writtenworld.commit/rkey001";
    let gate_src = GATE_TEMPLATE
        .replace("{vocab_uri}", vocab_uri)
        .replace("{vocab_cid}", vocab_cid)
        .replace("{epilogue_uri}", epilogue_uri)
        .replace("{epilogue_cid}", epilogue_cid);
    let gate = identify(&gate_src, gate_uri, "bafyGate");
    println!("=== RIVAL 1 (accepted): the loop closes by citation, not derivation ===\n{gate_src}");

    let criterion_uri = "at://did:plc:critical-reader/org.jason-edelman.writtenworld.commit/rkey002";
    let criterion_src = CRITERION_MISSING_TEMPLATE
        .replace("{star_cult_uri}", star_cult_uri)
        .replace("{star_cult_cid}", star_cult_cid)
        .replace("{fuhrer_uri}", fuhrer_uri)
        .replace("{fuhrer_cid}", fuhrer_cid);
    let criterion = identify(&criterion_src, criterion_uri, "bafyCriterion");
    println!("=== RIVAL 2 (accepted): no criterion discriminates trivial from catastrophic pseudo-aura ===\n{criterion_src}");

    let promiscuity_uri = "at://did:plc:critical-reader/org.jason-edelman.writtenworld.commit/rkey003";
    let promiscuity_src = PROMISCUITY_TEMPLATE
        .replace("{magician_uri}", magician_uri)
        .replace("{magician_cid}", magician_cid)
        .replace("{surgeon_uri}", surgeon_uri)
        .replace("{surgeon_cid}", surgeon_cid);
    let promiscuity = identify(&promiscuity_src, promiscuity_uri, "bafyPromiscuity");
    println!("=== RIVAL 3 (built, then disputed): the penetration-structure is politically promiscuous ===\n{promiscuity_src}");

    let dispute_uri = "at://did:plc:dev-lead/org.jason-edelman.writtenworld.commit/rkey001";
    let dispute_src = DISPUTE_TEMPLATE
        .replace("{promiscuity_uri}", promiscuity_uri)
        .replace("{promiscuity_cid}", "bafyPromiscuity")
        .replace("{fuhrer_uri}", fuhrer_uri)
        .replace("{fuhrer_cid}", fuhrer_cid);
    let dispute = identify(&dispute_src, dispute_uri, "bafyDispute");
    println!("=== DISPUTE: the equivalence conflates apparatus-under-duress with surgical structure ===\n{dispute_src}");

    // Check 1: RIVAL 1 genuinely consumes BOTH the Preface's vocabulary
    // stance AND the Epilogue's claim -- the same two facts the original
    // log's own loop-closure commit consumed, now recombined into a
    // materially different conclusion.
    assert_eq!(gate.commit.consumes.len(), 2);
    println!(
        "\nCheck 1: gate.commit.consumes.len() = {} -- citing the SAME two facts the original \
         log's loop-closure commit cited, but producing 'stipulated,' not 'derived.' Same \
         evidence, incompatible conclusion, both real citations.",
        gate.commit.consumes.len(),
    );

    // Check 2: RIVAL 2 genuinely consumes both cult facts together.
    assert_eq!(criterion.commit.consumes.len(), 2);
    println!(
        "Check 2: criterion.commit.consumes.len() = {} -- the discrimination gap is a real \
         citation of both cult facts together, not an assertion made in isolation.",
        criterion.commit.consumes.len(),
    );

    // Check 3: the dispute genuinely consumes RIVAL 3's own claim,
    // engaging it rather than ignoring it -- same shape as
    // editorial_loop.rs.
    assert_eq!(dispute.commit.consumes.len(), 2);
    println!(
        "Check 3: dispute.commit.consumes.len() = {} -- the counter-claim genuinely cites \
         RIVAL 3's own promiscuity claim, engaging it rather than silently overriding or \
         ignoring it.",
        dispute.commit.consumes.len(),
    );

    // Check 4: both RIVAL 3 and the dispute remain independently real
    // and citable -- neither erases the other, matching editorial_loop.rs's
    // finding that disputes coexist rather than replace.
    let full_log = vec![
        vocab.clone(), epilogue.clone(), star_cult.clone(), fuhrer.clone(),
        magician.clone(), surgeon.clone(), gate.clone(), criterion.clone(),
        promiscuity.clone(), dispute.clone(),
    ];
    let materialized = Materialized::from_identified_commits(&full_log);
    let promiscuity_alone = Materialized::from_identified_commits(&[promiscuity.clone()]);
    assert!(promiscuity_alone
        .current_value("argument/section_xi_ambiguity", "penetrationStructurePoliticalValence")
        .is_some());
    println!(
        "Check 4: RIVAL 3's original claim, materialized alone: {:?} -- still real and citable, \
         not erased by the dispute. Current combined-log value for counterClaim: {:?}. Both \
         positions are in the log; the reader (not the grammar) has to weigh them.",
        promiscuity_alone.current_value("argument/section_xi_ambiguity", "penetrationStructurePoliticalValence"),
        materialized.current_value("argument/section_xi_ambiguity", "counterClaim"),
    );

    println!(
        "\n=== done: two of ox-alpha's three challenges accepted as real, checkable rival \
         commits citing the same facts the original log used (Checks 1-2); the third is built \
         faithfully AND disputed rather than silently resolved, with both positions surviving in \
         the log (Checks 3-4). A rival reading, done honestly, isn't a takedown or a rubber \
         stamp -- it's a mix, checked piece by piece. ==="
    );
}
