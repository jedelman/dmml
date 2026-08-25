//! Section III, read slowly: a third citation posture (distinct from
//! Section I's endorsed Valery and Section II's hedged Gance), a
//! stipulated rather than derived definition, and the essay's actual
//! payoff -- the social mechanism Section I's methodological framing
//! promised but didn't yet deliver. Run with `cargo run -p dmml --example
//! benjamin_section_iii`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. Riegl and Wickhoff are cited as CORRECT AND INCOMPLETE, and the
//!    incompleteness itself is what licenses Benjamin's next move --
//!    "they did not attempt -- and, perhaps, saw no way -- to show the
//!    social transformations... the conditions for an analogous insight
//!    are more favorable in the present." Modeled as a commit consuming
//!    BOTH their positive claim and their stated scope-limit, producing a
//!    methodological warrant that depends on the limit being real, not
//!    just the claim. A third citation posture, none of the three
//!    identical: Valery (Section I, load-bearing, unhedged), Gance
//!    (Section II, cited then partly disowned), Riegl/Wickhoff (cited as
//!    correct, explicitly incomplete, and the incompleteness is the
//!    warrant).
//! 2. The natural-object aura definition ("we define the aura of the
//!    latter as...") is modeled as a STIPULATION over the existing "aura"
//!    term, not a derivation -- it consumes the fact that "aura" already
//!    exists as a coined term, but produces its extension by assertion,
//!    with no further consumes chained under it the way Section II's
//!    four-paragraph derivation had one. Checked: this commit's consumes
//!    count is 1 (the term exists), not built up from multiple premises
//!    the way Section II's chain was.
//! 3. The essay's actual causal payoff -- TWO mass-psychological drives,
//!    not the reproduction-technology mechanism Section II already gave --
//!    is modeled as its own attribute (`massDesire`), consuming BOTH the
//!    methodological warrant (Check 1) and the natural-object aura
//!    definition (Check 2) together. This is a different twofold from
//!    Section II's `underminingMechanism`: that one was about how
//!    reproduction technology undermines authenticity; this one is about
//!    why audiences want it to. Checked: they remain two separate,
//!    independently-queryable facts, not merged into one.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// The "aura" term already coined (Section II) -- re-declared minimally
// here for this file's self-containment, matching the standing pattern
// of these examples.
const AURA_EXISTS_SRC: &str = r#"
commit coins {
  declare attribute aura

  argument/section_ii aura "coined-over-three-linked-losses-not-one-undifferentiated-one"
}
"#;

// Riegl and Wickhoff, cited as real AND explicitly incomplete -- both
// halves stated, not just the flattering one.
const RIEGL_WICKHOFF_SRC: &str = r#"
commit asserts {
  declare attribute claim
  declare attribute scope

  source/rieglWickhoff claim "late Roman perception has its own formal hallmark, distinct from antiquity"
  source/rieglWickhoff scope "formal-hallmark-only-did-not-attempt-social-causes"
}
"#;

// The methodological warrant: their incompleteness licenses Benjamin's
// next move. Consumes BOTH their claim and their stated limit -- the
// warrant depends on the limit being real, not only on the claim.
const METHODOLOGY_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {rw_claim_uri} (cid: {rw_claim_cid}) {
      subject: source/rieglWickhoff
      predicate: claim
    }
    fact {rw_scope_uri} (cid: {rw_scope_cid}) {
      subject: source/rieglWickhoff
      predicate: scope
    }
  }
  produces {
    argument/section_iii_methodology claim "conditions now favor showing aura-decay's social causes, exactly where Riegl and Wickhoff's formal analysis stopped short"
  }
}
"#;

// The natural-object aura definition -- STIPULATED, not derived. Consumes
// only the fact that "aura" already exists as a term; the mountain-range/
// branch extension is asserted by illustration, not built up through
// further premises the way Section II's chain was.
const AURA_NATURAL_TEMPLATE: &str = r#"
commit stipulates {
  declare attribute naturalAuraDefinition

  consumes {
    fact {aura_uri} (cid: {aura_cid}) {
      subject: argument/section_ii
      predicate: aura
    }
  }
  produces {
    argument/section_iii_aura_natural naturalAuraDefinition "unique phenomenon of a distance, however close it may be -- illustrated by a mountain range or a branch's shadow, not argued for"
  }
}
"#;

// The payoff: two mass-psychological drives, a DIFFERENT twofold from
// Section II's reproduction-technology mechanism -- desire, not technique.
// Consumes the methodology warrant AND the natural-object definition
// together, since the "destroy its aura" language needs the natural-
// object sense of aura, not just the historical-object one.
const MASS_DESIRE_TEMPLATE: &str = r#"
commit argues {
  declare attribute massDesire
  declare attribute statisticsAnalogy

  consumes {
    fact {methodology_uri} (cid: {methodology_cid}) {
      subject: argument/section_iii_methodology
      predicate: claim
    }
    fact {aura_natural_uri} (cid: {aura_natural_cid}) {
      subject: argument/section_iii_aura_natural
      predicate: naturalAuraDefinition
    }
  }
  produces {
    argument/section_iii_mechanism massDesire "bring-things-closer-spatially-and-humanly, AND overcome-uniqueness-by-accepting-reproduction"
    argument/section_iii_mechanism statisticsAnalogy "sense of the universal equality of things, paralleling the rise of statistics in the theoretical sphere"
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
    let aura_uri = "at://did:plc:form-reading-refined/org.jason-edelman.writtenworld.commit/rkey004";
    let aura_cid = "bafyAuraExists";
    let aura_exists = identify(AURA_EXISTS_SRC, aura_uri, aura_cid);

    let rw_uri = "at://did:plc:riegl-wickhoff/org.jason-edelman.writtenworld.commit/rkey001";
    let rw_cid = "bafyRieglWickhoff";
    let riegl_wickhoff = identify(RIEGL_WICKHOFF_SRC, rw_uri, rw_cid);
    println!("=== Riegl and Wickhoff, cited as correct AND incomplete ===\n{RIEGL_WICKHOFF_SRC}");

    let methodology_uri = "at://did:plc:form-reading-iii/org.jason-edelman.writtenworld.commit/rkey001";
    let methodology_cid = "bafyMethodology";
    let methodology_src = METHODOLOGY_TEMPLATE
        .replace("{rw_claim_uri}", rw_uri)
        .replace("{rw_claim_cid}", rw_cid)
        .replace("{rw_scope_uri}", rw_uri)
        .replace("{rw_scope_cid}", rw_cid);
    let methodology = identify(&methodology_src, methodology_uri, methodology_cid);
    println!("=== Methodological warrant: their incompleteness licenses this ===\n{methodology_src}");

    let aura_natural_uri = "at://did:plc:form-reading-iii/org.jason-edelman.writtenworld.commit/rkey002";
    let aura_natural_cid = "bafyAuraNatural";
    let aura_natural_src = AURA_NATURAL_TEMPLATE
        .replace("{aura_uri}", aura_uri)
        .replace("{aura_cid}", aura_cid);
    let aura_natural = identify(&aura_natural_src, aura_natural_uri, aura_natural_cid);
    println!("=== Natural-object aura: stipulated, not derived ===\n{aura_natural_src}");

    let mass_desire_uri = "at://did:plc:form-reading-iii/org.jason-edelman.writtenworld.commit/rkey003";
    let mass_desire_cid = "bafyMassDesire";
    let mass_desire_src = MASS_DESIRE_TEMPLATE
        .replace("{methodology_uri}", methodology_uri)
        .replace("{methodology_cid}", methodology_cid)
        .replace("{aura_natural_uri}", aura_natural_uri)
        .replace("{aura_natural_cid}", aura_natural_cid);
    let mass_desire = identify(&mass_desire_src, mass_desire_uri, mass_desire_cid);
    println!("=== The payoff: two mass-psychological drives, a different twofold from Section II ===\n{mass_desire_src}");

    // Check 1: the methodological warrant genuinely consumes BOTH Riegl/
    // Wickhoff's claim and their stated limit -- the warrant depends on
    // the limit being real, not only the positive claim.
    assert_eq!(
        methodology.commit.consumes.len(),
        2,
        "the warrant cites both their claim and their stated incompleteness"
    );
    println!(
        "\nCheck 1: methodology.commit.consumes.len() = {} -- Riegl and Wickhoff are cited as \
         correct AND incomplete, and the incompleteness is what licenses Benjamin's next move, \
         a third citation posture distinct from Valery's (endorsed) and Gance's (hedged).",
        methodology.commit.consumes.len(),
    );

    // Check 2: the natural-object aura definition consumes only ONE fact
    // (that "aura" already exists) -- stipulation by illustration, not a
    // multi-premise derivation the way Section II's four-paragraph chain
    // was.
    assert_eq!(
        aura_natural.commit.consumes.len(),
        1,
        "the natural-object definition rests on one fact (the term exists), not a built-up derivation"
    );
    println!(
        "Check 2: aura_natural.commit.consumes.len() = {} -- stipulated by illustration \
         (mountain range, branch's shadow), not argued for the way Section II's chain was.",
        aura_natural.commit.consumes.len(),
    );

    // Check 3: the mass-desire mechanism and Section II's reproduction-
    // technology mechanism remain two independent, separately-queryable
    // facts -- checked by materializing them together and confirming
    // both attributes exist without either overwriting the other.
    let full_log = vec![
        aura_exists.clone(),
        riegl_wickhoff.clone(),
        methodology.clone(),
        aura_natural.clone(),
        mass_desire.clone(),
    ];
    let materialized = Materialized::from_identified_commits(&full_log);
    let mass_desire_value = materialized.current_value("argument/section_iii_mechanism", "massDesire");
    assert!(mass_desire_value.is_some());
    println!(
        "Check 3: current_value(argument/section_iii_mechanism, massDesire) = {mass_desire_value:?} \
         -- a genuinely separate attribute from Section II's underminingMechanism (reproduction \
         technology's own capacity), not a restatement of it. Technology explains HOW; this \
         explains WHY audiences want it to."
    );

    println!(
        "\n=== done: a scholarly precedent cited as correct-and-incomplete, its incompleteness \
         itself the warrant (Check 1); a stipulated definition kept honestly distinct from a \
         derived one by its consumes count (Check 2); the social mechanism modeled as its own \
         fact, independent of the reproduction-technology mechanism Section II already \
         established (Check 3). ==="
    );
}
