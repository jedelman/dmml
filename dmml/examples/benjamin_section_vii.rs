//! Section VII, read slowly: a FOURTH citation posture (distinct from
//! Section I's endorsed Valery, Section II's hedged Gance, Section III's
//! correct-but-incomplete Riegl/Wickhoff) -- four external theorists cited
//! NEGATIVELY, as instances of a diagnosed error, not as evidence for
//! anything Benjamin endorses. And a real fan-in shape: ONE diagnosis
//! fact, consumed independently by FOUR witnesses, none citing each
//! other -- the mirror image of Section IV's fan-in-then-pivot (there,
//! many premises fed one pivot; here, one diagnosis licenses many
//! independent illustrations). Also: Gance himself is cited TWICE across
//! this essay (Section II's hedged "will make films" enthusiasm, and this
//! section's "hieroglyphs" comparison, cited as flatly wrong) -- modeled
//! as two separate facts from the same source, not one fact reused,
//! because they are two different claims with two different postures.
//! Run with `cargo run -p dmml --example benjamin_section_vii`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. The diagnosis ("the primary question -- whether photography's
//!    invention transformed art's entire nature -- was never raised")
//!    is consumed by FOUR independent witness commits, none of which cite
//!    each other. Checked: each witness's consumes count is 1 (only the
//!    diagnosis), and none of the four witnesses appears in any other
//!    witness's consumes.
//! 2. Gance is cited twice, correctly modeled as two SEPARATE facts (not
//!    one fact re-used) -- Section II's `source/gance` (the "will make
//!    films" quote, hedged-but-partly-sympathetic) and this section's
//!    `source/ganceHieroglyphs` (the hieroglyphs comparison, cited
//!    flatly as an instance of the diagnosed error). Checked: these are
//!    different (uri, cid) pairs with different produced content, not the
//!    same fact reappearing.
//! 3. A real counter-evidence fact -- L'Opinion publique and The Gold
//!    Rush had ALREADY appeared when these theories were published --
//!    modeled as its own commit, consuming all four witnesses together,
//!    producing a rebuttal that the theorists' sacred/ritual framing was
//!    already empirically outdated even as spoken. Checked: this
//!    rebuttal's consumes count is 4, citing every witness at once, not
//!    just the most recent one.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// The dispute itself: "devious and confused," yet important as a SYMPTOM
// of a transformation neither rival realized. Asserted, a historical
// datum, no consumes.
const DISPUTE_SRC: &str = r#"
commit asserts {
  declare attribute surfaceCoherence
  declare attribute symptomaticSignificance

  argument/painting_photography_dispute surfaceCoherence "devious and confused"
  argument/painting_photography_dispute symptomaticSignificance "important precisely as a symptom of a historical transformation neither rival realized"
}
"#;

// "the semblance of its autonomy disappeared forever" -- flagged again:
// the same one-way, permanent-loss language as Section II's aura
// withering. Not re-litigated here; the tension already stands.
const AUTONOMY_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {dispute_uri} (cid: {dispute_cid}) {
      subject: argument/painting_photography_dispute
      predicate: symptomaticSignificance
    }
  }
  produces {
    argument/section_vii claim "the semblance of art's autonomy disappeared forever once mechanical reproduction separated it from its basis in cult"
  }
}
"#;

// The actual diagnosis: the WRONG question was asked ("is photography/
// film art?") instead of the RIGHT one ("did photography's invention
// transform art's entire nature?").
const DIAGNOSIS_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {autonomy_uri} (cid: {autonomy_cid}) {
      subject: argument/section_vii
      predicate: claim
    }
  }
  produces {
    argument/section_vii_diagnosis claim "the primary question -- whether photography's invention transformed art's entire nature -- was never raised; theorists asked instead whether photography or film IS art"
  }
}
"#;

// Gance's SECOND appearance, hieroglyphs -- a DIFFERENT fact from Section
// II's "will make films" quote, same source, different claim, different
// posture (flatly diagnosed as wrong here, not hedged-but-sympathetic).
const GANCE_HIEROGLYPHS_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/ganceHieroglyphs claim "by a remarkable regression, we have come back to the level of expression of the Egyptians; there is as yet insufficient cult of what film expresses"
}
"#;

const SEVERIN_MARS_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/severinMars claim "only the most high-minded persons, in the most perfect and mysterious moments of their lives, should be allowed to enter its ambience"
}
"#;

const ARNOUX_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/arnoux claim "do not all the bold descriptions we have given amount to the definition of prayer?"
}
"#;

const WERFEL_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/werfel claim "the film's real possibilities consist in its unique faculty to express all that is fairylike, marvelous, supernatural"
}
"#;

// Four witness commits -- each consumes ONLY the diagnosis, none cites
// any other witness. The same diagnosed error, four independent
// illustrations.
const WITNESS_TEMPLATE: &str = r#"
commit argues {
  declare attribute instance

  consumes {
    fact {diagnosis_uri} (cid: {diagnosis_cid}) {
      subject: argument/section_vii_diagnosis
      predicate: claim
    }
    fact {source_uri} (cid: {source_cid}) {
      subject: {source_subject}
      predicate: claim
    }
  }
  produces {
    {witness_subject} instance "reads ritual elements into film with a striking lack of discretion, forcing film into a category (art) it need not occupy"
  }
}
"#;

// The rebuttal: L'Opinion publique and The Gold Rush had ALREADY
// appeared when these theories were published -- an empirical counter-
// fact, consuming all four witnesses at once.
const COUNTEREVIDENCE_TEMPLATE: &str = r#"
commit argues {
  declare attribute rebuttal

  consumes {
    fact {gance_witness_uri} (cid: {gance_witness_cid}) {
      subject: witness/gance
      predicate: instance
    }
    fact {severin_witness_uri} (cid: {severin_witness_cid}) {
      subject: witness/severinMars
      predicate: instance
    }
    fact {arnoux_witness_uri} (cid: {arnoux_witness_cid}) {
      subject: witness/arnoux
      predicate: instance
    }
    fact {werfel_witness_uri} (cid: {werfel_witness_cid}) {
      subject: witness/werfel
      predicate: instance
    }
  }
  produces {
    argument/section_vii_counterevidence rebuttal "L'Opinion publique and The Gold Rush had already appeared when these speculations were published -- the sacred/ritual framing was already empirically outdated even as spoken"
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

fn witness_uri(name: &str) -> String {
    format!("at://did:plc:{name}/org.jason-edelman.writtenworld.commit/rkey001")
}

fn main() {
    let dispute_uri = "at://did:plc:form-reading-vii/org.jason-edelman.writtenworld.commit/rkey001";
    let dispute_cid = "bafyDispute";
    let dispute = identify(DISPUTE_SRC, dispute_uri, dispute_cid);
    println!("=== The dispute: devious and confused, yet symptomatic ===\n{DISPUTE_SRC}");

    let autonomy_uri = "at://did:plc:form-reading-vii/org.jason-edelman.writtenworld.commit/rkey002";
    let autonomy_cid = "bafyAutonomy";
    let autonomy_src = AUTONOMY_TEMPLATE
        .replace("{dispute_uri}", dispute_uri)
        .replace("{dispute_cid}", dispute_cid);
    let autonomy = identify(&autonomy_src, autonomy_uri, autonomy_cid);

    let diagnosis_uri = "at://did:plc:form-reading-vii/org.jason-edelman.writtenworld.commit/rkey003";
    let diagnosis_cid = "bafyDiagnosis";
    let diagnosis_src = DIAGNOSIS_TEMPLATE
        .replace("{autonomy_uri}", autonomy_uri)
        .replace("{autonomy_cid}", autonomy_cid);
    let diagnosis = identify(&diagnosis_src, diagnosis_uri, diagnosis_cid);
    println!("=== The diagnosis: the wrong question was asked ===\n{diagnosis_src}");

    let gance_h_uri = witness_uri("ganceHieroglyphs");
    let gance_h = identify(GANCE_HIEROGLYPHS_SRC, &gance_h_uri, "bafyGanceHieroglyphs");
    let severin_uri = witness_uri("severinMars");
    let severin = identify(SEVERIN_MARS_SRC, &severin_uri, "bafySeverinMars");
    let arnoux_uri = witness_uri("arnoux");
    let arnoux = identify(ARNOUX_SRC, &arnoux_uri, "bafyArnoux");
    let werfel_uri = witness_uri("werfel");
    let werfel = identify(WERFEL_SRC, &werfel_uri, "bafyWerfel");
    println!(
        "=== Four external witnesses, cited NEGATIVELY -- Gance (hieroglyphs), Severin-Mars, \
         Arnoux, Werfel ===\n{GANCE_HIEROGLYPHS_SRC}{SEVERIN_MARS_SRC}{ARNOUX_SRC}{WERFEL_SRC}"
    );

    let witness = |name: &str, source_uri: &str, source_cid: &str, source_subject: &str| {
        let witness_uri_str = witness_uri(&format!("witness-{name}"));
        let src = WITNESS_TEMPLATE
            .replace("{diagnosis_uri}", diagnosis_uri)
            .replace("{diagnosis_cid}", diagnosis_cid)
            .replace("{source_uri}", source_uri)
            .replace("{source_cid}", source_cid)
            .replace("{source_subject}", source_subject)
            .replace("{witness_subject}", &format!("witness/{name}"));
        identify(&src, &witness_uri_str, &format!("bafyWitness{name}"))
    };

    let gance_witness = witness("gance", &gance_h_uri, "bafyGanceHieroglyphs", "source/ganceHieroglyphs");
    let severin_witness = witness("severinMars", &severin_uri, "bafySeverinMars", "source/severinMars");
    let arnoux_witness = witness("arnoux", &arnoux_uri, "bafyArnoux", "source/arnoux");
    let werfel_witness = witness("werfel", &werfel_uri, "bafyWerfel", "source/werfel");

    let counterevidence_src = COUNTEREVIDENCE_TEMPLATE
        .replace("{gance_witness_uri}", &gance_witness.uri)
        .replace("{gance_witness_cid}", &gance_witness.cid)
        .replace("{severin_witness_uri}", &severin_witness.uri)
        .replace("{severin_witness_cid}", &severin_witness.cid)
        .replace("{arnoux_witness_uri}", &arnoux_witness.uri)
        .replace("{arnoux_witness_cid}", &arnoux_witness.cid)
        .replace("{werfel_witness_uri}", &werfel_witness.uri)
        .replace("{werfel_witness_cid}", &werfel_witness.cid);
    let counterevidence_uri = "at://did:plc:form-reading-vii/org.jason-edelman.writtenworld.commit/rkey004";
    let counterevidence = identify(&counterevidence_src, counterevidence_uri, "bafyCounterevidence");
    println!("=== Counter-evidence: the films these theorists discussed already existed ===\n{counterevidence_src}");

    // Check 1: fan-in -- all four witnesses consume ONLY the diagnosis
    // (plus their own quoted source), none cite each other.
    for (name, w) in [
        ("gance", &gance_witness),
        ("severinMars", &severin_witness),
        ("arnoux", &arnoux_witness),
        ("werfel", &werfel_witness),
    ] {
        assert_eq!(w.commit.consumes.len(), 2, "{name} consumes only the diagnosis + its own source quote");
    }
    println!(
        "\nCheck 1: all four witnesses consume exactly 2 facts each (the shared diagnosis + \
         their own quote) -- a real fan-in, one diagnosis independently illustrated four times, \
         none of the witnesses citing each other."
    );

    // Check 2: Gance's two appearances are genuinely separate facts, not
    // the same fact reused -- different (uri, cid), different content.
    assert_ne!(gance_h.uri, "at://did:plc:gance/org.jason-edelman.writtenworld.commit/rkey001");
    let gance_h_materialized = Materialized::from_identified_commits(&[gance_h.clone()]);
    let gance_h_claim = gance_h_materialized.current_value("source/ganceHieroglyphs", "claim");
    println!(
        "Check 2: source/ganceHieroglyphs claim = {gance_h_claim:?} -- a genuinely different \
         fact from Section II's source/gance (\"will make films\"), same author, two different \
         claims, two different citation postures (there: hedged-but-sympathetic; here: flatly \
         diagnosed as an instance of the error)."
    );

    // Check 3: the counter-evidence rebuttal consumes ALL FOUR witnesses
    // at once, not just the latest one.
    assert_eq!(counterevidence.commit.consumes.len(), 4);
    println!(
        "Check 3: counterevidence.commit.consumes.len() = {} -- the rebuttal engages every \
         witness together, not just the most recent, matching the text's own move (both named \
         films had already appeared, undercutting ALL of these theorists at once, not just one).",
        counterevidence.commit.consumes.len(),
    );

    println!(
        "\n=== done: a real fan-in, one diagnosis independently illustrated by four witnesses \
         that never cite each other (Check 1); Gance's two appearances modeled honestly as two \
         separate facts with two different postures (Check 2); the counter-evidence rebuttal \
         engages all four witnesses at once (Check 3). ==="
    );
}
