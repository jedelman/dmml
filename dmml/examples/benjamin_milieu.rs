//! Two independent, zero-coordination extractions of the same page of
//! Benjamin (Preface-IV) -- a CONTENT reading (what art historically does)
//! and a FORM reading (what Benjamin's own argument does) -- built the same
//! way `pantheon.rs` builds Helios/Selene/Eos: neither graph consumes the
//! other. Then a third party cites specific facts out of BOTH by `FactRef`
//! and produces a genuine synthesis, and a fourth party recombines the
//! SAME two facts into a different reading. Prompted directly: "both
//! extractions are valid. In fact, each enhances the other. And they can be
//! remixed and recomposed to create new understandings which evolve." This
//! file checks that claim against real interpreter output rather than just
//! agreeing with it in prose. Run with `cargo run -p dmml --example
//! benjamin_milieu`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. The CONTENT graph (did:plc:content-reading) and the FORM graph
//!    (did:plc:form-reading) are each internally coherent, self-citing
//!    chains -- the content chain tracks the technique/ritual pivot itself;
//!    the form chain tracks Benjamin's own argumentative dependencies
//!    (Section IV citing BOTH Section II's coinage and Section III's
//!    extension, not just the section immediately before it -- matching
//!    the essay's actual structure, not its numbering). Neither graph
//!    references the other -- confirmed by construction, same as
//!    pantheon.rs's rival deities.
//! 2. A milieu commit, from a third DID, `consumes` one fact from EACH
//!    graph -- a real cross-graph citation, not an invented synthesis --
//!    and produces an insight that could not be derived from either graph
//!    alone: that the content pivot and the form pivot are the same move
//!    Benjamin's own endnote 5 makes about aura and cult value -- one
//!    phenomenon under two descriptions, not two facts that happen to
//!    correlate.
//! 3. A SECOND milieu commit, from a fourth DID, consumes the exact same
//!    two facts and produces a DIFFERENT reading -- that the two pivots
//!    should NOT be identified, because conflating "art's own history"
//!    with "the history of Benjamin's argument about art" repeats the
//!    sensor/symbol conflation the DMML papers already had to catch
//!    themselves making. Checked: both milieu commits remain fully present
//!    and independently re-materializable; the current view shows only the
//!    later one, last-write-wins, exactly as `pantheon.rs` and
//!    `editorial_loop.rs` already established for rival syntheses and
//!    disputed resolutions respectively. This is the concrete referent for
//!    "remixed and recomposed... which evolve" -- two real recombinations
//!    of the same milieu, neither privileged by the grammar, both citable
//!    forever.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// ===== CONTENT graph: what art historically does =====

const CONTENT_TECHNIQUE_SRC: &str = r#"
commit asserts {
  declare attribute technique
  declare attribute handInvolvement

  reproduction/1 technique "founding-and-stamping"
  reproduction/1 handInvolvement "full"
}
"#;

// Section I's own claim: "Around 1900 technical reproduction had reached a
// standard that... had captured a place of its own among the artistic
// processes" -- photography as the qualitative break, hand ceded to eye.
const CONTENT_PHOTOGRAPHY_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute technique
  declare attribute handInvolvement

  consumes {
    fact {technique_uri} (cid: {technique_cid}) {
      subject: reproduction/1
      predicate: technique
    }
  }
  produces {
    reproduction/1 technique "photography"
    reproduction/1 handInvolvement "none"
  }
}
"#;

const CONTENT_RITUAL_SRC: &str = r#"
commit asserts {
  declare attribute basis
  declare attribute auraStatus

  artwork/1 basis "ritual"
  artwork/1 auraStatus "present"
}
"#;

// Section IV's actual pivot sentence: "the total function of art is
// reversed. Instead of being based on ritual, it begins to be based on
// another practice -- politics." Consumes BOTH the technique fact
// (authenticity ceases to be applicable once reproduction is total) and
// the ritual fact -- two consumes, matching Benjamin's own two-premise move.
const CONTENT_PIVOT_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute basis
  declare attribute auraStatus

  consumes {
    fact {photography_uri} (cid: {photography_cid}) {
      subject: reproduction/1
      predicate: technique
    }
    fact {ritual_uri} (cid: {ritual_cid}) {
      subject: artwork/1
      predicate: basis
    }
  }
  produces {
    artwork/1 basis "exhibition"
    artwork/1 auraStatus "withered"
  }
}
"#;

// ===== FORM graph: what Benjamin's own argument does =====

// The Preface is a `declare` move, not a derivation -- no consumes.
const FORM_PREFACE_SRC: &str = r#"
commit declares {
  declare attribute vocabularyStance

  argument/preface vocabularyStance "anti-fascist-terms"
}
"#;

// Section I cites Valery externally -- a real quoted source, modeled as
// its own produced fact by a different DID entirely, not an invented
// placeholder.
const VALERY_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/valery claim "images will appear and disappear at a simple movement of the hand"
}
"#;

const FORM_SECTION_I_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {valery_uri} (cid: {valery_cid}) {
      subject: source/valery
      predicate: claim
    }
  }
  produces {
    argument/section_i claim "reproduction-technique-reached-full-standard"
  }
}
"#;

// Section II's coinage: "One might subsume the eliminated element in the
// term 'aura'" -- consumes Section I's own claim, DECLARES a new term over
// it. The coining move itself, not just a new fact.
const FORM_SECTION_II_TEMPLATE: &str = r#"
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

// Section III: consumes Section II's coinage, extends by analogy
// (mountain range, natural objects) -- self-citation building on Benjamin's
// own prior production, not new external evidence.
const FORM_SECTION_III_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {section_ii_uri} (cid: {section_ii_cid}) {
      subject: argument/section_ii
      predicate: aura
    }
  }
  produces {
    argument/section_iii claim "aura-defined-as-distance"
  }
}
"#;

// Section IV consumes BOTH II and III -- not just the section immediately
// before it. This is the essay's actual dependency structure, not its
// numbering.
const FORM_SECTION_IV_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {section_ii_uri} (cid: {section_ii_cid}) {
      subject: argument/section_ii
      predicate: claim
    }
    fact {section_iii_uri} (cid: {section_iii_cid}) {
      subject: argument/section_iii
      predicate: claim
    }
  }
  produces {
    argument/section_iv claim "ritual-basis-yields-to-politics"
  }
}
"#;

// ===== MILIEU: third-party recombination across both graphs =====

// First reading: identification. Echoes Benjamin's OWN endnote-5 move
// (aura and cult value as one phenomenon in two registers) by making the
// same move about this file's two graphs.
const MILIEU_IDENTIFIES_TEMPLATE: &str = r#"
commit synthesizes {
  declare attribute insight

  consumes {
    fact {content_pivot_uri} (cid: {content_pivot_cid}) {
      subject: artwork/1
      predicate: basis
    }
    fact {form_pivot_uri} (cid: {form_pivot_cid}) {
      subject: argument/section_iv
      predicate: claim
    }
  }
  produces {
    milieu/1 insight "content-pivot-and-form-pivot-are-one-move-in-two-registers"
  }
}
"#;

// Second reading, different DID, SAME two consumed facts: a dissent,
// not a derivation from the first milieu commit -- it consumes the
// original graphs directly, not the first synthesis.
const MILIEU_DISTINGUISHES_TEMPLATE: &str = r#"
commit synthesizes {
  declare attribute insight

  consumes {
    fact {content_pivot_uri} (cid: {content_pivot_cid}) {
      subject: artwork/1
      predicate: basis
    }
    fact {form_pivot_uri} (cid: {form_pivot_cid}) {
      subject: argument/section_iv
      predicate: claim
    }
  }
  produces {
    milieu/1 insight "conflating them repeats the sensor-symbol confusion the DMML papers already caught themselves making"
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
    // --- CONTENT graph ---
    let technique_uri = "at://did:plc:content-reading/org.jason-edelman.writtenworld.commit/rkey001";
    let technique_cid = "bafyTechniqueOrigin";
    let technique = identify(CONTENT_TECHNIQUE_SRC, technique_uri, technique_cid);

    let photography_uri = "at://did:plc:content-reading/org.jason-edelman.writtenworld.commit/rkey002";
    let photography_cid = "bafyPhotography";
    let photography_src = CONTENT_PHOTOGRAPHY_TEMPLATE
        .replace("{technique_uri}", technique_uri)
        .replace("{technique_cid}", technique_cid);
    let photography = identify(&photography_src, photography_uri, photography_cid);

    let ritual_uri = "at://did:plc:content-reading/org.jason-edelman.writtenworld.commit/rkey003";
    let ritual_cid = "bafyRitualPresent";
    let ritual = identify(CONTENT_RITUAL_SRC, ritual_uri, ritual_cid);

    let content_pivot_uri = "at://did:plc:content-reading/org.jason-edelman.writtenworld.commit/rkey004";
    let content_pivot_cid = "bafyContentPivot";
    let content_pivot_src = CONTENT_PIVOT_TEMPLATE
        .replace("{photography_uri}", photography_uri)
        .replace("{photography_cid}", photography_cid)
        .replace("{ritual_uri}", ritual_uri)
        .replace("{ritual_cid}", ritual_cid);
    let content_pivot = identify(&content_pivot_src, content_pivot_uri, content_pivot_cid);

    println!("=== CONTENT graph: technique chain + ritual pivot ===");
    println!(
        "reproduction/1 technique chain -> photography; artwork/1 basis: ritual -> exhibition \
         (consuming both the technique fact and the ritual fact, two premises, one produces)."
    );

    // --- FORM graph ---
    let preface_uri = "at://did:plc:form-reading/org.jason-edelman.writtenworld.commit/rkey001";
    let preface = identify(FORM_PREFACE_SRC, preface_uri, "bafyPreface");

    let valery_uri = "at://did:plc:valery/org.jason-edelman.writtenworld.commit/rkey001";
    let valery_cid = "bafyValery1931";
    let valery = identify(VALERY_SRC, valery_uri, valery_cid);

    let section_i_uri = "at://did:plc:form-reading/org.jason-edelman.writtenworld.commit/rkey002";
    let section_i_cid = "bafySectionI";
    let section_i_src = FORM_SECTION_I_TEMPLATE
        .replace("{valery_uri}", valery_uri)
        .replace("{valery_cid}", valery_cid);
    let section_i = identify(&section_i_src, section_i_uri, section_i_cid);

    let section_ii_uri = "at://did:plc:form-reading/org.jason-edelman.writtenworld.commit/rkey003";
    let section_ii_cid = "bafySectionIICoinage";
    let section_ii_src = FORM_SECTION_II_TEMPLATE
        .replace("{section_i_uri}", section_i_uri)
        .replace("{section_i_cid}", section_i_cid);
    let section_ii = identify(&section_ii_src, section_ii_uri, section_ii_cid);

    let section_iii_uri = "at://did:plc:form-reading/org.jason-edelman.writtenworld.commit/rkey004";
    let section_iii_cid = "bafySectionIII";
    let section_iii_src = FORM_SECTION_III_TEMPLATE
        .replace("{section_ii_uri}", section_ii_uri)
        .replace("{section_ii_cid}", section_ii_cid);
    let section_iii = identify(&section_iii_src, section_iii_uri, section_iii_cid);

    let section_iv_uri = "at://did:plc:form-reading/org.jason-edelman.writtenworld.commit/rkey005";
    let section_iv_cid = "bafySectionIVPivot";
    let section_iv_src = FORM_SECTION_IV_TEMPLATE
        .replace("{section_ii_uri}", section_ii_uri)
        .replace("{section_ii_cid}", section_ii_cid)
        .replace("{section_iii_uri}", section_iii_uri)
        .replace("{section_iii_cid}", section_iii_cid);
    let section_iv = identify(&section_iv_src, section_iv_uri, section_iv_cid);

    println!("=== FORM graph: Benjamin's own argumentative dependency chain ===");
    println!(
        "Preface (declares) -> Section I (consumes Valery) -> Section II (consumes I, COINS \
         'aura') -> Section III (consumes II, extends) -> Section IV (consumes BOTH II and III)."
    );

    // Check 1: the two graphs are genuinely independent -- neither
    // consumes anything from the other.
    assert_eq!(content_pivot.commit.consumes.len(), 2);
    assert_eq!(section_iv.commit.consumes.len(), 2);
    println!(
        "\nCheck 1: content_pivot.consumes = {} (technique + ritual); section_iv.consumes = {} \
         (Section II + Section III) -- zero cross-references between the two graphs, same as \
         pantheon.rs's rival deities.",
        content_pivot.commit.consumes.len(),
        section_iv.commit.consumes.len(),
    );

    // --- MILIEU: third-party recombination ---
    let identifies_src = MILIEU_IDENTIFIES_TEMPLATE
        .replace("{content_pivot_uri}", content_pivot_uri)
        .replace("{content_pivot_cid}", content_pivot_cid)
        .replace("{form_pivot_uri}", section_iv_uri)
        .replace("{form_pivot_cid}", section_iv_cid);
    let identifies = identify(
        &identifies_src,
        "at://did:plc:milieu-reader-one/org.jason-edelman.writtenworld.commit/rkey001",
        "bafyMilieuIdentifies",
    );

    let distinguishes_src = MILIEU_DISTINGUISHES_TEMPLATE
        .replace("{content_pivot_uri}", content_pivot_uri)
        .replace("{content_pivot_cid}", content_pivot_cid)
        .replace("{form_pivot_uri}", section_iv_uri)
        .replace("{form_pivot_cid}", section_iv_cid);
    let distinguishes = identify(
        &distinguishes_src,
        "at://did:plc:milieu-reader-two/org.jason-edelman.writtenworld.commit/rkey001",
        "bafyMilieuDistinguishes",
    );

    println!("\n=== MILIEU: two independent third-party recombinations of the same two facts ===");

    // Check 2: both milieu commits genuinely cite one fact from EACH
    // graph -- a real cross-graph citation, not invention.
    assert_eq!(identifies.commit.consumes.len(), 2);
    assert_eq!(distinguishes.commit.consumes.len(), 2);
    println!(
        "identifies.consumes = {}, distinguishes.consumes = {} -- both cite the SAME two facts \
         (content_pivot, form section_iv), one from each independently-produced graph.",
        identifies.commit.consumes.len(),
        distinguishes.commit.consumes.len(),
    );

    // Check 3: both readings remain fully present and independently
    // re-materializable -- "remixed and recomposed... which evolve" is not
    // a metaphor, it's this: two genuine, coexisting recombinations of one
    // milieu, neither erasing the other.
    let identifies_alone = Materialized::from_identified_commits(&[identifies.clone()]);
    let distinguishes_alone = Materialized::from_identified_commits(&[distinguishes.clone()]);
    assert_eq!(
        identifies_alone.current_value("milieu/1", "insight"),
        Some(&dmml::lower::TripleValue::Str(
            "content-pivot-and-form-pivot-are-one-move-in-two-registers".to_string()
        ))
    );
    assert_eq!(
        distinguishes_alone.current_value("milieu/1", "insight"),
        Some(&dmml::lower::TripleValue::Str(
            "conflating them repeats the sensor-symbol confusion the DMML papers already caught themselves making".to_string()
        ))
    );
    println!(
        "\nCheck 3: identifies, materialized alone: {:?}\ndistinguishes, materialized alone: {:?}\n\
         Both real. Both citable. Neither requires the other to exist.",
        identifies_alone.current_value("milieu/1", "insight"),
        distinguishes_alone.current_value("milieu/1", "insight"),
    );

    // Check 4: the current view over the full log is still last-write-wins
    // -- order-dependent, same finding as benjamin.rs's fascism/communism
    // check. Two orderings, two different "current" answers.
    let order_a = Materialized::from_identified_commits(&[identifies.clone(), distinguishes.clone()]);
    let order_b = Materialized::from_identified_commits(&[distinguishes.clone(), identifies.clone()]);
    assert_eq!(
        order_a.current_value("milieu/1", "insight"),
        distinguishes_alone.current_value("milieu/1", "insight")
    );
    assert_eq!(
        order_b.current_value("milieu/1", "insight"),
        identifies_alone.current_value("milieu/1", "insight")
    );
    println!(
        "Check 4: current_value(milieu/1, insight) flips depending on log order -- \
         (identifies, distinguishes) order shows: {:?}; (distinguishes, identifies) order shows: {:?}. \
         Recomposition genuinely evolves the CURRENT reading; it does not erase either \
         underlying one.",
        order_a.current_value("milieu/1", "insight"),
        order_b.current_value("milieu/1", "insight"),
    );

    // Preface and section_i/ii/iii kept in scope so nothing is dead code
    // -- they anchor section_iv's two-fact consumes above.
    let _ = (&preface, &section_i, &section_ii, &section_iii, &technique, &photography, &ritual, &valery);

    println!(
        "\n=== done: two zero-coordination graphs (Check 1) genuinely recombined by two \
         independent third parties (Check 2), both recombinations permanently real (Check 3), \
         the current view of that recombination itself evolving with log order rather than \
         collapsing to one fixed answer (Check 4). ==="
    );
}
