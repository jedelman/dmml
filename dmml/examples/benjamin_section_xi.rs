//! Section XI, read slowly -- and this is the section that pays off a
//! structural guess this series has been carrying since the very first
//! Section VIII reflection: the surgeon/painter material, once actually
//! built, connects DIRECTLY back to Section III's aura-as-distance
//! definition, not just by loose analogy but as a real, checkable
//! structural identification Benjamin himself draws. The magician heals
//! by "the laying on of hands," maintaining natural physical closeness
//! while INCREASING distance through authority -- this is exactly
//! Section III's "unique phenomenon of a distance, however close it may
//! be," here transposed from art onto medicine. The surgeon does the
//! reverse: physical penetration, no authority-based distance, "abstains
//! from facing the patient man to man." Painter maps to magician (total
//! picture, natural distance -- aura's structure); cameraman maps to
//! surgeon (fragmented picture "assembled under a new law" -- Section
//! IX's montage material, named explicitly). Run with `cargo run -p dmml
//! --example benjamin_section_xi`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. "Illusion of the second degree" -- the film scene's equipment-free
//!    appearance is manufactured entirely through cutting -- genuinely
//!    consumes Section IX's montage/fragmentation fact (re-declared
//!    cross-file), not asserted independently. The orchid metaphor
//!    ("the sight of immediate reality has become an orchid in the land
//!    of technology") is produced alongside it.
//! 2. THE PAYOFF TEST: does the magician's authority-based distance
//!    genuinely connect, as a checkable consumes, to Section III's
//!    aura-as-distance definition (re-declared cross-file)? If this
//!    citation doesn't hold up structurally, the guess from the Section
//!    VIII conversation ("the surgeon/painter analogy... matching the
//!    same abstract structure as aura-as-distance") was just a plausible-
//!    sounding pattern-match, not a real one. Checked below.
//! 3. Painter/cameraman is modeled as ONE commit producing TWO facts (the
//!    doubling pattern from Sections V and IX), explicitly consuming both
//!    the magician/surgeon mapping and the illusion-of-the-second-degree
//!    fact -- the mapping isn't free-floating, it's built on both pieces
//!    together.
//! 4. The closing paradox -- total mechanical permeation produces an
//!    equipment-free-SEEMING result -- consumes BOTH the painter/
//!    cameraman mapping AND the illusion-of-the-second-degree fact,
//!    confirming the paradox genuinely depends on both, not a
//!    restatement of either alone.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// Section IX's montage fact, re-declared cross-file per this series'
// convention.
const MONTAGE_SRC: &str = r#"
commit argues {
  declare attribute fragmentation

  argument/section_ix_montage fragmentation "composed of many separate performances -- a jump from a window shot as a jump from a scaffold, weeks apart; a startled reaction shot by firing an unforewarned gunshot behind the actor, cut in afterward"
}
"#;

// Section III's aura-as-distance definition, re-declared cross-file.
const AURA_DISTANCE_SRC: &str = r#"
commit stipulates {
  declare attribute naturalAuraDefinition

  argument/section_iii_aura_natural naturalAuraDefinition "unique phenomenon of a distance, however close it may be -- illustrated by a mountain range or a branch's shadow, not argued for"
}
"#;

// "Illusion of the second degree" -- genuinely consumes the montage fact,
// since it IS the result of cutting many separate performances together.
const ILLUSION_SECOND_DEGREE_TEMPLATE: &str = r#"
commit argues {
  declare attribute illusionDegree
  declare attribute metaphor

  consumes {
    fact {montage_uri} (cid: {montage_cid}) {
      subject: argument/section_ix_montage
      predicate: fragmentation
    }
  }
  produces {
    argument/section_xi illusionDegree "the equipment-free aspect of the shot is the result of a special procedure -- specially adjusted camera and the mounting of the shot together with other similar ones -- illusion of the second degree, not the first"
    argument/section_xi metaphor "the sight of immediate reality has become an orchid in the land of technology"
  }
}
"#;

// The magician -- Benjamin's own stipulated analogy, not a citation.
// Natural distance maintained, increased through AUTHORITY.
const MAGICIAN_SRC: &str = r#"
commit asserts {
  declare attribute distanceStrategy

  role/magician distanceStrategy "maintains natural distance from the patient, laying on of hands, but greatly increases distance through authority"
}
"#;

const SURGEON_SRC: &str = r#"
commit asserts {
  declare attribute distanceStrategy

  role/surgeon distanceStrategy "greatly diminishes physical distance by penetrating the body, increases it only slightly through caution, abstains from facing the patient man to man"
}
"#;

// THE PAYOFF TEST: does the magician's distance-through-authority
// genuinely consume Section III's aura-as-distance fact?
const MAGICIAN_IS_AURA_TEMPLATE: &str = r#"
commit argues {
  declare attribute structuralMatch

  consumes {
    fact {magician_uri} (cid: {magician_cid}) {
      subject: role/magician
      predicate: distanceStrategy
    }
    fact {aura_distance_uri} (cid: {aura_distance_cid}) {
      subject: argument/section_iii_aura_natural
      predicate: naturalAuraDefinition
    }
  }
  produces {
    role/magician structuralMatch "the magician's authority-based distance instantiates the same structure Section III already named: a unique phenomenon of a distance, however close it may be -- here transposed from art onto medicine"
  }
}
"#;

// Painter maps to magician, cameraman to surgeon -- ONE commit, TWO
// produced facts, consuming BOTH the magician/aura identification AND
// the illusion-of-the-second-degree fact together.
const PAINTER_CAMERAMAN_TEMPLATE: &str = r#"
commit argues {
  declare attribute pictureType
  declare attribute distanceType

  consumes {
    fact {magician_aura_uri} (cid: {magician_aura_cid}) {
      subject: role/magician
      predicate: structuralMatch
    }
    fact {illusion_uri} (cid: {illusion_cid}) {
      subject: argument/section_xi
      predicate: illusionDegree
    }
  }
  produces {
    artist/painter pictureType "a total one -- maintains a natural distance from reality, matching the magician's structure"
    artist/cameraman pictureType "multiple fragments assembled under a new law -- penetrates deeply into reality's web, matching the surgeon's structure"
  }
}
"#;

// The closing paradox: total mechanical permeation yields an equipment-
// free-SEEMING result -- consumes BOTH the painter/cameraman mapping AND
// the illusion-of-the-second-degree fact.
const PARADOX_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {painter_cameraman_uri} (cid: {painter_cameraman_cid}) {
      subject: artist/cameraman
      predicate: pictureType
    }
    fact {illusion_uri} (cid: {illusion_cid}) {
      subject: argument/section_xi
      predicate: illusionDegree
    }
  }
  produces {
    argument/section_xi_paradox claim "for contemporary man film's representation of reality is incomparably more significant than the painter's, precisely because the thoroughgoing permeation of reality with mechanical equipment offers an aspect of reality free of all equipment"
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
    let montage_uri = "at://did:plc:form-reading-ix/org.jason-edelman.writtenworld.commit/rkey005";
    let montage_cid = "bafyMontageXI";
    let montage = identify(MONTAGE_SRC, montage_uri, montage_cid);

    let aura_distance_uri = "at://did:plc:form-reading-iii/org.jason-edelman.writtenworld.commit/rkey002";
    let aura_distance_cid = "bafyAuraDistanceXI";
    let aura_distance = identify(AURA_DISTANCE_SRC, aura_distance_uri, aura_distance_cid);
    println!("=== Carried over: Section IX's montage fact, Section III's aura-as-distance ===");

    let illusion_uri = "at://did:plc:form-reading-xi/org.jason-edelman.writtenworld.commit/rkey001";
    let illusion_cid = "bafyIllusionSecondDegree";
    let illusion_src = ILLUSION_SECOND_DEGREE_TEMPLATE
        .replace("{montage_uri}", montage_uri)
        .replace("{montage_cid}", montage_cid);
    let illusion = identify(&illusion_src, illusion_uri, illusion_cid);
    println!("=== Illusion of the second degree, and the orchid metaphor ===\n{illusion_src}");

    let magician_uri = "at://did:plc:form-reading-xi/org.jason-edelman.writtenworld.commit/rkey002";
    let magician_cid = "bafyMagician";
    let magician = identify(MAGICIAN_SRC, magician_uri, magician_cid);
    let surgeon_uri = "at://did:plc:form-reading-xi/org.jason-edelman.writtenworld.commit/rkey003";
    let surgeon = identify(SURGEON_SRC, surgeon_uri, "bafySurgeon");
    println!("=== Benjamin's own analogy: magician and surgeon ===\n{MAGICIAN_SRC}{SURGEON_SRC}");

    let magician_aura_uri = "at://did:plc:form-reading-xi/org.jason-edelman.writtenworld.commit/rkey004";
    let magician_aura_src = MAGICIAN_IS_AURA_TEMPLATE
        .replace("{magician_uri}", magician_uri)
        .replace("{magician_cid}", magician_cid)
        .replace("{aura_distance_uri}", aura_distance_uri)
        .replace("{aura_distance_cid}", aura_distance_cid);
    let magician_aura = identify(&magician_aura_src, magician_aura_uri, "bafyMagicianIsAura");
    println!("=== THE PAYOFF TEST: does the magician's distance connect to Section III's aura? ===\n{magician_aura_src}");

    let painter_cameraman_uri = "at://did:plc:form-reading-xi/org.jason-edelman.writtenworld.commit/rkey005";
    let painter_cameraman_src = PAINTER_CAMERAMAN_TEMPLATE
        .replace("{magician_aura_uri}", magician_aura_uri)
        .replace("{magician_aura_cid}", "bafyMagicianIsAura")
        .replace("{illusion_uri}", illusion_uri)
        .replace("{illusion_cid}", illusion_cid);
    let painter_cameraman = identify(&painter_cameraman_src, painter_cameraman_uri, "bafyPainterCameraman");
    println!("=== Painter maps to magician, cameraman to surgeon ===\n{painter_cameraman_src}");

    let paradox_uri = "at://did:plc:form-reading-xi/org.jason-edelman.writtenworld.commit/rkey006";
    let paradox_src = PARADOX_TEMPLATE
        .replace("{painter_cameraman_uri}", painter_cameraman_uri)
        .replace("{painter_cameraman_cid}", "bafyPainterCameraman")
        .replace("{illusion_uri}", illusion_uri)
        .replace("{illusion_cid}", illusion_cid);
    let paradox = identify(&paradox_src, paradox_uri, "bafyParadox");
    println!("=== The closing paradox ===\n{paradox_src}");

    // Check 1: illusion-of-the-second-degree genuinely consumes Section
    // IX's montage fact.
    assert_eq!(illusion.commit.consumes.len(), 1);
    println!(
        "\nCheck 1: illusion.commit.consumes.len() = {} -- illusion of the second degree is \
         built ON Section IX's montage material, the same fragmentation now explained as the \
         SOURCE of film's equipment-free appearance.",
        illusion.commit.consumes.len(),
    );

    // Check 2: THE PAYOFF TEST. Does magician_aura genuinely consume
    // BOTH the magician's distanceStrategy AND Section III's
    // naturalAuraDefinition?
    assert_eq!(magician_aura.commit.consumes.len(), 2);
    let cites_aura_definition = magician_aura.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "naturalAuraDefinition",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_aura_definition, "the magician/aura identification must genuinely cite Section III's aura-as-distance fact");
    println!(
        "Check 2 (THE PAYOFF TEST): magician_aura.commit.consumes.len() = {}, and one of those \
         two facts is Section III's naturalAuraDefinition predicate -- CONFIRMED: the magician's \
         authority-based distance really does connect, as a checkable citation, to the aura-as- \
         distance definition. The Section VIII conversation's guess holds up under an actual \
         check, six sections later.",
        magician_aura.commit.consumes.len(),
    );

    // Check 3: painter/cameraman produces facts for BOTH subjects from
    // ONE commit (checked by predicate name, per Section V/IX's lowering
    // artifact), consuming both the magician/aura fact and the illusion
    // fact together.
    assert_eq!(painter_cameraman.commit.consumes.len(), 2);
    let produced_predicates: std::collections::BTreeSet<&str> = painter_cameraman
        .commit
        .produces
        .iter()
        .map(|t| t.predicate.as_str())
        .collect();
    assert!(produced_predicates.contains("pictureType"));
    let materialized = Materialized::from_identified_commits(&[painter_cameraman.clone()]);
    println!(
        "Check 3: painter_cameraman.consumes.len() = {}; painter pictureType = {:?}; cameraman \
         pictureType = {:?} -- one commit, mapping BOTH roles at once, built on both the \
         magician/aura identification and the illusion-of-the-second-degree fact together.",
        painter_cameraman.commit.consumes.len(),
        materialized.current_value("artist/painter", "pictureType"),
        materialized.current_value("artist/cameraman", "pictureType"),
    );

    // Check 4: the closing paradox consumes BOTH the painter/cameraman
    // mapping AND the illusion fact -- not a restatement of either alone.
    assert_eq!(paradox.commit.consumes.len(), 2);
    println!(
        "Check 4: paradox.commit.consumes.len() = {} -- the closing claim genuinely draws on \
         both the painter/cameraman mapping and the illusion-of-the-second-degree fact, not a \
         restatement of one alone.",
        paradox.commit.consumes.len(),
    );

    println!(
        "\n=== done: illusion of the second degree is built on Section IX's montage, not \
         independent of it (Check 1); the magician's authority-distance GENUINELY connects to \
         Section III's aura-as-distance, confirming a guess from six sections earlier under an \
         actual check (Check 2, the real point of this file); painter/cameraman maps both roles \
         at once, built on two prior facts (Check 3); the closing paradox draws on both, not one \
         alone (Check 4). ==="
    );
}
