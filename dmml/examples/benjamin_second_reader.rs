//! A genuinely independent second reading, from an agent given the
//! PRIMARY TEXT itself (not a compressed summary of this project's own
//! facts) and no memory of this session's prior conversation. Unlike
//! `benjamin_rival_reading.rs` (dispatched to ox-alpha with only a fact
//! summary), this reading was built cold from the essay, then reviewed
//! here the same way: accepted where it holds up, extended where it
//! sharpens something already built, never applied wholesale without
//! checking. Run with `cargo run -p dmml --example
//! benjamin_second_reader`.
//!
//! Four of the second reader's six points are built here as real
//! commits from `did:plc:second-reader`:
//!
//! 1. The actor's alienation (Section IX-X) is sharper read through
//!    commodity fetishism than through "hollow aura substitute" alone --
//!    consumes the star-cult fact, produces a claim that doesn't replace
//!    it but adds a distinct mechanism (mystifying alienated labor, not
//!    just borrowing aura's shape).
//! 2. Section VII is the Preface's own method DEMONSTRATED, not just
//!    asserted -- a real cross-section link the original unified log
//!    never built. Consumes both the Preface's vocabulary-stance fact
//!    and Section VII's diagnosis fact together.
//! 3. Fascism doesn't merely fail to be the surgeon (my own dispute of
//!    ox-alpha in `benjamin_rival_reading.rs`) -- it actively
//!    reconstructs the magician's authority-distance around the Fuhrer,
//!    a stronger, more falsifiable claim. Consumes my own dispute
//!    commit AND the magician fact together, extending rather than
//!    repeating it.
//! 4. Dada (Section XIV) destroys its own aura using no reproduction
//!    technology at all, which is in real tension with the Preface's own
//!    claim that superstructure "transformation... takes more than half
//!    a century" to catch up with substructure -- art moving AHEAD of
//!    its supposed technological cause. Consumes a freshly-declared
//!    Preface lag-claim fact (present in the primary text, never built
//!    as its own fact in the unified log) and Dada's aura-destruction
//!    fact together.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// Facts re-declared cross-file from the unified essay and the rival
// reading, per this series' standing convention.
const STAR_CULT_SRC: &str = r#"
commit reproduces {
  declare attribute claim

  argument/section_x_star_cult claim "the cult of the movie star preserves not the unique aura of the person but the phony spell of a commodity"
}
"#;

const VOCAB_STANCE_SRC: &str = r#"
commit declares {
  declare attribute vocabularyStance

  argument/preface vocabularyStance "anti-fascist-terms"
}
"#;

const SECTION_VII_DIAGNOSIS_SRC: &str = r#"
commit argues {
  declare attribute claim

  argument/section_vii_diagnosis claim "the primary question -- did photography's invention transform art's entire nature -- was never raised; theorists asked instead whether photography or film IS art, reading ritual elements into film with a striking lack of discretion"
}
"#;

const MAGICIAN_SRC: &str = r#"
commit asserts {
  declare attribute distanceStrategy

  role/magician distanceStrategy "maintains natural distance, increases it through authority"
}
"#;

const DEV_LEAD_DISPUTE_SRC: &str = r#"
commit disputes {
  declare attribute counterClaim

  argument/section_xi_ambiguity counterClaim "the equivalence conflates two different mechanisms -- the Epilogue's apparatus is pressed into the production of RITUAL values, forcing ritual content INTO an apparatus, closer to the opposite of the surgeon's structure. Fascism's move is a forced reversal toward auratic/ritual structure, not an instance of surgical structure."
}
"#;

const DADA_AURA_DESTRUCTION_SRC: &str = r#"
commit reproduces {
  declare attribute claim
  declare attribute agency

  argument/section_xiv_dada claim "what the Dadaists intended and achieved was a relentless destruction of the aura of their creations, which they branded as reproductions with the very means of production"
  argument/section_xiv_dada agency "deliberate, using no reproduction technology at all -- a poem, a canvas with buttons mounted on it"
}
"#;

// A fact the original unified log never built explicitly: the Preface's
// own stated LAG between substructure and superstructure transformation.
const PREFACE_LAG_SRC: &str = r#"
commit asserts {
  declare attribute claim

  argument/preface_lag claim "the transformation of the superstructure, which takes place far more slowly than that of the substructure, has taken more than half a century to manifest the change in the conditions of production"
}
"#;

// ===== 1. Actor's alienation sharpened via commodity fetishism, not
// just "hollow aura substitute." Adds to, doesn't replace, the star-cult
// reading. =====
const COMMODITY_FETISHISM_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {star_cult_uri} (cid: {star_cult_cid}) {
      subject: argument/section_x_star_cult
      predicate: claim
    }
  }
  produces {
    argument/section_x_second_reading claim "the 'phony spell of a commodity' line should be read through the factory-article analogy Benjamin gives two sentences earlier ('as little contact with it as any article made in a factory') -- this is commodity fetishism attaching to a human personality, a distinct and sharper mechanism than 'aura substitute': the star cult mystifies alienated labor-power the same way commodity fetishism mystifies a produced object's social origin"
  }
}
"#;

// ===== 2. Section VII demonstrates the Preface's method rather than
// merely asserting it -- a real cross-section link the original log
// never built. =====
const PREFACE_METHOD_DEMONSTRATED_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {vocab_uri} (cid: {vocab_cid}) {
      subject: argument/preface
      predicate: vocabularyStance
    }
    fact {diagnosis_uri} (cid: {diagnosis_cid}) {
      subject: argument/section_vii_diagnosis
      predicate: claim
    }
  }
  produces {
    argument/section_vii_second_reading claim "critics sacralizing film (hieroglyphs, 'definition of prayer,' 'fairylike, marvelous, supernatural') are the Preface's rejected vocabulary caught in the act of reattaching itself to a new medium -- Section VII performs, in real time, the exact danger the Preface warned about ('uncontrolled application would lead to a processing of data in the Fascist sense'), which is why Benjamin quotes them at length and mocks their lack of discretion rather than simply asserting the point"
  }
}
"#;

// ===== 3. Fascism as forced REGRESSION toward the magician pole --
// extends my own dispute of ox-alpha, sharper and more falsifiable. =====
const FORCED_REGRESSION_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim

  consumes {
    fact {dispute_uri} (cid: {dispute_cid}) {
      subject: argument/section_xi_ambiguity
      predicate: counterClaim
    }
    fact {magician_uri} (cid: {magician_cid}) {
      subject: role/magician
      predicate: distanceStrategy
    }
  }
  produces {
    argument/epilogue_second_reading claim "fascism does not merely fail to be the surgeon, it actively reverses the mapping: the technological apparatus (radio, film, mass rally staging) is used to reconstruct the magician's natural-distance-increased-by-authority specifically around the Fuhrer, while the masses are held at the opposite pole from the absent-minded examiner Section XV describes -- rapt, ritual spectatorship. A stronger, more falsifiable claim than 'produces ritual values' alone."
  }
}
"#;

// ===== 4. Dada precedes its own technological cause -- real tension
// with the Preface's own stated lag claim. =====
const DADA_PRECEDES_CAUSE_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {preface_lag_uri} (cid: {preface_lag_cid}) {
      subject: argument/preface_lag
      predicate: claim
    }
    fact {dada_uri} (cid: {dada_cid}) {
      subject: argument/section_xiv_dada
      predicate: agency
    }
  }
  produces {
    argument/section_xiv_second_reading claim "Dada's aura-destruction anticipates and rehearses film's shock/tactile structure using no reproduction technology at all -- at least one thread of aura's destruction runs through deliberate artistic strategy AHEAD of its supposed technological cause, in real tension with the Preface's own claim that superstructure transformation lags substructure by more than half a century"
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
    let star_cult_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0023";
    let star_cult_cid = "bafyEssay0023";
    let star_cult = identify(STAR_CULT_SRC, star_cult_uri, star_cult_cid);

    let vocab_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0001";
    let vocab_cid = "bafyEssay0001";
    let vocab = identify(VOCAB_STANCE_SRC, vocab_uri, vocab_cid);

    let diagnosis_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0026";
    let diagnosis_cid = "bafyEssay0026";
    let diagnosis = identify(SECTION_VII_DIAGNOSIS_SRC, diagnosis_uri, diagnosis_cid);

    let magician_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0024";
    let magician_cid = "bafyEssay0024";
    let magician = identify(MAGICIAN_SRC, magician_uri, magician_cid);

    let dispute_uri = "at://did:plc:dev-lead/org.jason-edelman.writtenworld.commit/rkey001";
    let dispute_cid = "bafyDispute";
    let dispute = identify(DEV_LEAD_DISPUTE_SRC, dispute_uri, dispute_cid);

    let dada_uri = "at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey0037";
    let dada_cid = "bafyEssay0037";
    let dada = identify(DADA_AURA_DESTRUCTION_SRC, dada_uri, dada_cid);

    let preface_lag_uri = "at://did:plc:second-reader/org.jason-edelman.writtenworld.commit/rkey000";
    let preface_lag_cid = "bafyPrefaceLag";
    let preface_lag = identify(PREFACE_LAG_SRC, preface_lag_uri, preface_lag_cid);

    println!("=== Carried over: star cult, Preface stance, Section VII diagnosis, magician, the dispute, Dada, the Preface's lag claim ===\n");

    let commodity_uri = "at://did:plc:second-reader/org.jason-edelman.writtenworld.commit/rkey001";
    let commodity_src = COMMODITY_FETISHISM_TEMPLATE
        .replace("{star_cult_uri}", star_cult_uri)
        .replace("{star_cult_cid}", star_cult_cid);
    let commodity = identify(&commodity_src, commodity_uri, "bafyCommodity");
    println!("=== 1. Commodity fetishism sharpens, doesn't replace, the star-cult reading ===\n{commodity_src}");

    let method_uri = "at://did:plc:second-reader/org.jason-edelman.writtenworld.commit/rkey002";
    let method_src = PREFACE_METHOD_DEMONSTRATED_TEMPLATE
        .replace("{vocab_uri}", vocab_uri)
        .replace("{vocab_cid}", vocab_cid)
        .replace("{diagnosis_uri}", diagnosis_uri)
        .replace("{diagnosis_cid}", diagnosis_cid);
    let method = identify(&method_src, method_uri, "bafyMethod");
    println!("=== 2. Section VII demonstrates the Preface's own method -- a new cross-section link ===\n{method_src}");

    let regression_uri = "at://did:plc:second-reader/org.jason-edelman.writtenworld.commit/rkey003";
    let regression_src = FORCED_REGRESSION_TEMPLATE
        .replace("{dispute_uri}", dispute_uri)
        .replace("{dispute_cid}", dispute_cid)
        .replace("{magician_uri}", magician_uri)
        .replace("{magician_cid}", magician_cid);
    let regression = identify(&regression_src, regression_uri, "bafyRegression");
    println!("=== 3. Fascism as forced regression to the magician pole -- extends the dispute ===\n{regression_src}");

    let dada_second_uri = "at://did:plc:second-reader/org.jason-edelman.writtenworld.commit/rkey004";
    let dada_second_src = DADA_PRECEDES_CAUSE_TEMPLATE
        .replace("{preface_lag_uri}", preface_lag_uri)
        .replace("{preface_lag_cid}", preface_lag_cid)
        .replace("{dada_uri}", dada_uri)
        .replace("{dada_cid}", dada_cid);
    let dada_second = identify(&dada_second_src, dada_second_uri, "bafyDadaSecond");
    println!("=== 4. Dada precedes its own technological cause -- tension with the Preface's lag claim ===\n{dada_second_src}");

    // Check 1: commodity fetishism genuinely consumes the star-cult fact
    // and adds to it, not replacing it -- both remain real and citable.
    assert_eq!(commodity.commit.consumes.len(), 1);
    let star_cult_alone = Materialized::from_identified_commits(&[star_cult.clone()]);
    assert!(star_cult_alone.current_value("argument/section_x_star_cult", "claim").is_some());
    println!(
        "\nCheck 1: commodity.commit.consumes.len() = {} -- built ON the star-cult fact, which \
         remains fully real and citable, materialized alone.",
        commodity.commit.consumes.len(),
    );

    // Check 2: THE NEW LINK. Section VII genuinely connects to the
    // Preface -- a real cross-section citation the original unified log
    // never built.
    assert_eq!(method.commit.consumes.len(), 2);
    let cites_preface = method.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "vocabularyStance",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_preface, "Section VII's demonstration claim must genuinely cite the Preface's vocabulary stance");
    println!(
        "Check 2 (A GENUINELY NEW LINK): method.commit.consumes.len() = {}, citing the \
         Preface's vocabularyStance directly -- a real cross-section citation the original \
         44-commit log never built, surfaced only because this reader worked from the primary \
         text rather than my own compressed summary.",
        method.commit.consumes.len(),
    );

    // Check 3: the forced-regression claim genuinely consumes AND
    // extends my own dispute, not ox-alpha's original claim.
    assert_eq!(regression.commit.consumes.len(), 2);
    let cites_my_dispute = regression.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "counterClaim",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_my_dispute, "the regression claim must genuinely cite the Dev Lead's own dispute, not ox-alpha's original claim");
    println!(
        "Check 3: regression.commit.consumes.len() = {}, citing my OWN dispute commit's \
         counterClaim predicate -- this reading extends my rebuttal into a positive, sharper \
         claim rather than re-litigating ox-alpha's original point.",
        regression.commit.consumes.len(),
    );

    // Check 4: Dada's claim genuinely consumes the Preface's OWN lag
    // fact, exposing a real internal tension.
    assert_eq!(dada_second.commit.consumes.len(), 2);
    let cites_lag = dada_second.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.subject == "argument/preface_lag",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_lag, "the Dada claim must genuinely cite the Preface's own lag fact");
    println!(
        "Check 4: dada_second.commit.consumes.len() = {}, citing the Preface's own lag claim -- \
         a real internal tension: Dada's deliberate aura-destruction (Section XIV) uses no \
         reproduction technology at all, in real friction with the Preface's own half-century \
         lag claim.",
        dada_second.commit.consumes.len(),
    );

    println!(
        "\n=== done: an independent reader, given the primary text rather than a compressed \
         summary, sharpened one existing reading (Check 1), found a real cross-section link \
         nobody had built (Check 2), extended a live dispute into a stronger positive claim \
         (Check 3), and surfaced a genuine internal tension in the essay's own Marxist framing \
         (Check 4) -- four real, checkable contributions, reviewed and accepted here rather than \
         applied wholesale. ==="
    );
}
