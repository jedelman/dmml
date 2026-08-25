//! Not a correction -- an evolution of understanding, modeled the way
//! `editorial_loop.rs` models self-dispute: the new reading doesn't
//! overwrite the old one in place, it `consumes` it. Prompted directly:
//! "an evolution of understanding is different from a correction. consumes
//! might actually be a good primitive for that -- your new understanding
//! consumes the old." This file is that claim, checked, applied to a real
//! case: `benjamin_milieu.rs`'s FORM_SECTION_II_TEMPLATE compressed
//! Section II's actual four-paragraph argument (unique-existence's
//! history, authenticity's twofold undermining, the authenticity ->
//! testimony -> authority consequence chain, then the "aura" coinage over
//! all three) into a single coinage commit that skipped straight from
//! Section I to naming "aura." That original commit is left untouched in
//! the log below -- it is not deleted, edited, or reissued. A NEW chain of
//! finer-grained commits builds the four real paragraphs, and a final
//! REVISES commit consumes BOTH the original coarse reading AND the new
//! fine-grained one together, producing a claim that supersedes the
//! coarse reading in the current view while citing it as a real prior
//! understanding, not an error being erased. Run with `cargo run -p dmml
//! --example benjamin_understanding_evolves`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. The original coarse reading (argument/section_ii, from
//!    `benjamin_milieu.rs`'s own coinage template) is real and stays real
//!    -- materialized alone, it still says exactly what it said before,
//!    unaffected by anything built afterward.
//! 2. The REVISES commit's `consumes` count is 2, not 1 -- it genuinely
//!    cites the coarse reading alongside the new fine-grained derivation,
//!    the same shape `pantheon.rs`'s Nyx uses to weave three prior facts
//!    into one recombination, here applied reflexively to my own earlier
//!    pass over the same text rather than to three different deities.
//! 3. The current view over the full log (coarse reading, four
//!    fine-grained paragraph commits, then the revision) shows the revised
//!    claim -- last-write-wins, exactly as everywhere else in this
//!    project. What's different from a plain overwrite: the revision
//!    commit's own `consumes` makes the dependency on the coarse reading
//!    explicit and checkable, rather than silently discarding it the way
//!    replacing this file's own source text would have.
//! 4. Section II's fourth paragraph cites Gance's 1927 quote in a
//!    different posture than Section I cited Valery -- endorsed as
//!    evidence there, cited-then-partly-disowned here ("Presumably without
//!    intending it, he issued an invitation to a far-reaching
//!    liquidation"). Modeled as a single ordinary `consumes` -- the hedge
//!    lives in the produced claim's own content, not as a new grammar
//!    primitive invented to capture it. Checked: the commit still cites
//!    Gance for real (referential integrity holds) even though Benjamin's
//!    own text declines to fully endorse what's cited.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// ===== The ORIGINAL, coarse reading -- verbatim from benjamin_milieu.rs's
// FORM_SECTION_II_TEMPLATE. Left exactly as it was; not edited. =====

const SECTION_I_SRC: &str = r#"
commit argues {
  declare attribute claim

  argument/section_i claim "reproduction-technique-reached-full-standard"
}
"#;

const COARSE_SECTION_II_TEMPLATE: &str = r#"
commit coins {
  declare attribute aura
  declare attribute claim

  consumes {
    fact {section_i_uri} (cid: {section_i_cid}) {
      subject: argument/section_i
      predicate: claim
    }
  }
  produces {
    argument/section_ii claim "auratic-element-named"
    argument/section_ii aura "coined"
  }
}
"#;

// ===== The NEW, fine-grained reading -- Section II's actual four
// paragraphs, read slowly this time. =====

// Paragraph 1: unique existence entails a history -- physical-condition
// changes (chemical/physical analysis, impossible on a reproduction) and
// ownership changes (provenance) -- both requiring the original itself.
const PARA1_HISTORY_SRC: &str = r#"
commit argues {
  declare attribute physicalHistory
  declare attribute ownershipHistory

  artwork/mona_lisa physicalHistory "traceable-by-chemical-analysis-of-the-original-only"
  artwork/mona_lisa ownershipHistory "traceable-by-provenance-from-the-original-only"
}
"#;

// Paragraph 2: authenticity rests on presence of the original, undermined
// by technical reproduction for a stated TWOFOLD reason -- process
// reproduction is more independent of the original than manual
// reproduction (photography exceeds naked-eye vision); AND it can place
// the copy in situations out of the original's reach (the cathedral
// leaves its locale).
const PARA2_MECHANISM_TEMPLATE: &str = r#"
commit argues {
  declare attribute authenticity
  declare attribute underminingMechanism

  consumes {
    fact {para1_uri} (cid: {para1_cid}) {
      subject: artwork/mona_lisa
      predicate: physicalHistory
    }
  }
  produces {
    artwork/mona_lisa authenticity "grounded-in-original-presence"
    artwork/mona_lisa underminingMechanism "process-independence-and-situational-reach"
  }
}
"#;

// Paragraph 3: the consequence, stated as a CHAIN -- authenticity
// interfered with -> historical testimony jeopardized (because testimony
// "rests on" authenticity) -> "what is really jeopardized... is the
// authority of the object." Three links, not one claim.
const PARA3_CONSEQUENCE_TEMPLATE: &str = r#"
commit argues {
  declare attribute testimony
  declare attribute authority

  consumes {
    fact {para2_uri} (cid: {para2_cid}) {
      subject: artwork/mona_lisa
      predicate: authenticity
    }
  }
  produces {
    artwork/mona_lisa testimony "jeopardized"
    artwork/mona_lisa authority "jeopardized"
  }
}
"#;

// Gance's 1927 quote, cited then partly disowned -- a real external
// source, same shape as Section I's Valery citation, but consumed with a
// hedge stated in the produced claim rather than a special grammar
// primitive.
const GANCE_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/gance claim "Shakespeare, Rembrandt, Beethoven will make films"
}
"#;

// Paragraph 4: the naming move over ALL THREE prior links at once --
// "one might subsume the eliminated element in the term aura" -- plus the
// generalized twofold mechanism (plurality-of-copies, reactivation via the
// beholder's own situation), plus Gance cited as a symptom, not fully
// endorsed evidence.
const PARA4_NAMING_TEMPLATE: &str = r#"
commit coins {
  declare attribute aura
  declare attribute claim

  consumes {
    fact {para3_uri} (cid: {para3_cid}) {
      subject: artwork/mona_lisa
      predicate: authority
    }
    fact {gance_uri} (cid: {gance_cid}) {
      subject: source/gance
      predicate: claim
    }
  }
  produces {
    argument/section_ii_fine_grained claim "aura names the authenticity-testimony-authority chain, and Gance's enthusiasm is cited as a symptom of liquidation, not endorsed as its justification"
    argument/section_ii_fine_grained aura "coined-over-three-linked-losses-not-one-undifferentiated-one"
  }
}
"#;

// ===== The REVISION: consumes BOTH readings, produces the superseding
// claim -- Jason's actual proposed primitive, applied for real. =====

const REVISES_TEMPLATE: &str = r#"
commit revises {
  declare attribute claim
  declare relation refines

  consumes {
    fact {coarse_uri} (cid: {coarse_cid}) {
      subject: argument/section_ii
      predicate: claim
    }
    fact {fine_grained_uri} (cid: {fine_grained_cid}) {
      subject: argument/section_ii_fine_grained
      predicate: claim
    }
  }
  produces {
    argument/section_ii claim "the coinage names a three-link chain -- authenticity, testimony, authority -- each explicitly dependent on the one before, not a single undifferentiated loss"
    argument/section_ii refines argument/section_ii
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
    let section_i_uri = "at://did:plc:form-reading/org.jason-edelman.writtenworld.commit/rkey002";
    let section_i_cid = "bafySectionI";
    let section_i = identify(SECTION_I_SRC, section_i_uri, section_i_cid);

    let coarse_uri = "at://did:plc:form-reading/org.jason-edelman.writtenworld.commit/rkey003";
    let coarse_cid = "bafySectionIICoinage";
    let coarse_src = COARSE_SECTION_II_TEMPLATE
        .replace("{section_i_uri}", section_i_uri)
        .replace("{section_i_cid}", section_i_cid);
    let coarse = identify(&coarse_src, coarse_uri, coarse_cid);
    println!("=== ORIGINAL coarse reading (unchanged, from benjamin_milieu.rs) ===\n{coarse_src}");

    let para1_uri = "at://did:plc:form-reading-refined/org.jason-edelman.writtenworld.commit/rkey001";
    let para1_cid = "bafyPara1History";
    let para1 = identify(PARA1_HISTORY_SRC, para1_uri, para1_cid);

    let para2_uri = "at://did:plc:form-reading-refined/org.jason-edelman.writtenworld.commit/rkey002";
    let para2_cid = "bafyPara2Mechanism";
    let para2_src = PARA2_MECHANISM_TEMPLATE
        .replace("{para1_uri}", para1_uri)
        .replace("{para1_cid}", para1_cid);
    let para2 = identify(&para2_src, para2_uri, para2_cid);

    let para3_uri = "at://did:plc:form-reading-refined/org.jason-edelman.writtenworld.commit/rkey003";
    let para3_cid = "bafyPara3Consequence";
    let para3_src = PARA3_CONSEQUENCE_TEMPLATE
        .replace("{para2_uri}", para2_uri)
        .replace("{para2_cid}", para2_cid);
    let para3 = identify(&para3_src, para3_uri, para3_cid);

    let gance_uri = "at://did:plc:gance/org.jason-edelman.writtenworld.commit/rkey001";
    let gance_cid = "bafyGance1927";
    let gance = identify(GANCE_SRC, gance_uri, gance_cid);

    let para4_uri = "at://did:plc:form-reading-refined/org.jason-edelman.writtenworld.commit/rkey004";
    let para4_cid = "bafyPara4Naming";
    let para4_src = PARA4_NAMING_TEMPLATE
        .replace("{para3_uri}", para3_uri)
        .replace("{para3_cid}", para3_cid)
        .replace("{gance_uri}", gance_uri)
        .replace("{gance_cid}", gance_cid);
    let para4 = identify(&para4_src, para4_uri, para4_cid);
    println!(
        "=== NEW fine-grained reading: 4 paragraph-commits (history -> mechanism -> \
         consequence chain -> naming, citing Gance with a hedge) ===\n{para4_src}"
    );

    let revises_uri = "at://did:plc:form-reading-refined/org.jason-edelman.writtenworld.commit/rkey005";
    let revises_cid = "bafyRevisesSectionII";
    let revises_src = REVISES_TEMPLATE
        .replace("{coarse_uri}", coarse_uri)
        .replace("{coarse_cid}", coarse_cid)
        .replace("{fine_grained_uri}", para4_uri)
        .replace("{fine_grained_cid}", para4_cid);
    let revises = identify(&revises_src, revises_uri, revises_cid);
    println!("=== REVISES: consumes BOTH the coarse and the fine-grained readings ===\n{revises_src}");

    // Check 1: the coarse reading is real and stays real, unaffected by
    // anything built afterward.
    let coarse_alone = Materialized::from_identified_commits(&[coarse.clone()]);
    assert_eq!(
        coarse_alone.current_value("argument/section_ii", "claim"),
        Some(&dmml::lower::TripleValue::Str("auratic-element-named".to_string())),
        "the original coarse reading, materialized alone, still says exactly what it said before"
    );
    println!(
        "\nCheck 1: coarse reading, materialized alone: {:?} -- untouched.",
        coarse_alone.current_value("argument/section_ii", "claim"),
    );

    // Check 2: the revision genuinely cites BOTH readings -- 2 consumes,
    // not 1. This is the actual test of "consumes the old" rather than
    // "silently replaces the old."
    assert_eq!(
        revises.commit.consumes.len(),
        2,
        "the revision cites both the coarse reading and the new fine-grained derivation"
    );
    println!(
        "Check 2: revises.commit.consumes.len() = {} -- the coarse reading is a real \
         citation the revision depends on, not a fact it silently discards.",
        revises.commit.consumes.len(),
    );

    // Check 3: the current view over the full log shows the revised claim
    // -- but the revision's own dependency on the coarse reading is
    // checkable in the log, unlike a plain in-place edit would be.
    let full_log = vec![
        section_i.clone(),
        coarse.clone(),
        para1.clone(),
        para2.clone(),
        para3.clone(),
        gance.clone(),
        para4.clone(),
        revises.clone(),
    ];
    let materialized = Materialized::from_identified_commits(&full_log);
    assert_eq!(
        materialized.current_value("argument/section_ii", "claim"),
        Some(&dmml::lower::TripleValue::Str(
            "the coinage names a three-link chain -- authenticity, testimony, authority -- each explicitly dependent on the one before, not a single undifferentiated loss".to_string()
        ))
    );
    println!(
        "Check 3: current_value(argument/section_ii, claim) = {:?}\n\
         -- the CURRENT reading evolved; the coarse reading did not vanish, it was cited.",
        materialized.current_value("argument/section_ii", "claim"),
    );

    // Check 4: Gance is genuinely cited (referential integrity holds) even
    // though the produced claim explicitly hedges on endorsing him --
    // the hedge lives in the content, not a new grammar primitive.
    assert_eq!(para4.commit.consumes.len(), 2, "para4 cites both para3 and Gance's real quote");
    println!(
        "Check 4: para4.commit.consumes.len() = {} (para3 + Gance) -- Gance is cited for \
         real, same as Valery in Section I, even though the produced claim explicitly \
         declines to endorse his enthusiasm about it.",
        para4.commit.consumes.len(),
    );

    println!(
        "\n=== done: the coarse reading survives every later commit unaltered (Check 1); \
         the revision genuinely consumes it alongside new work rather than discarding it \
         (Check 2); the current view shows the evolved understanding while the log keeps the \
         dependency explicit (Check 3); a hedged external citation still holds referential \
         integrity (Check 4). ==="
    );
}
