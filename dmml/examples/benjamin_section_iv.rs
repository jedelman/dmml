//! Section IV, read slowly: a three-stage genealogy (ritual -> secularized
//! cult of beauty -> defensive l'art pour l'art), each stage consuming the
//! one before, feeding a pivot commit that consumes BOTH the genealogy's
//! endpoint AND a freshly-restated (not backward-cited) authenticity
//! point. Also demonstrates deferring citation verification rather than
//! blocking on it: the Mallarme attribution is entered with an explicit
//! `verificationStatus: "unverified"` fact, checkable and revisable later
//! the same way Section II's coarse reading got revised, rather than
//! either omitted or asserted as if checked. Run with `cargo run -p dmml
//! --example benjamin_section_iv`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. The genealogy is three real stages, not one jump -- ritual, then
//!    secularized cult of beauty (still ritual-based, now "in decline"),
//!    then l'art pour l'art (a defensive reaction to a SENSED crisis, not
//!    the crisis itself). Each stage's commit consumes exactly the fact
//!    immediately before it -- checked below, one consumes per step, not
//!    a single commit spanning all three.
//! 2. The pivot commit -- "the total function of art is reversed...
//!    based on... politics" -- consumes the genealogy's endpoint AND a
//!    SEPARATELY-STATED authenticity restatement (the photographic-
//!    negative example), not a backward citation into Section II's own
//!    commits. This models a real rhetorical move: Benjamin re-derives the
//!    authenticity point with a fresh example rather than just pointing
//!    back at Section II. Checked: the restatement commit has zero
//!    consumes of anything in the Section II files -- it is independently
//!    stated, matching the text's own independent restatement.
//! 3. The Mallarme attribution ("in poetry, Mallarme was the first to take
//!    this position") is entered with its own `verificationStatus`
//!    attribute set to "unverified" -- a real, checkable fact about the
//!    fact, not a blocking gate. This is the concrete form of "citation
//!    checking can be done after": the claim is in the log now, openly
//!    marked as unverified, and a later commit could consume THIS fact
//!    and revise verificationStatus once checked -- same consumes-the-old
//!    pattern as `benjamin_understanding_evolves.rs`, just not yet
//!    exercised here.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// Stage 1: ritual origin. Aura tracks embeddedness in SOME living
// tradition, not any fixed meaning within it -- Greeks venerated the
// Venus, medieval clerics saw "an ominous idol"; both equally confronted
// its aura. Asserted, not derived -- the starting point of the genealogy.
const RITUAL_SRC: &str = r#"
commit asserts {
  declare attribute basis
  declare attribute interpretationHistory

  artwork/venus basis "ritual-magical-then-religious"
  artwork/venus interpretationHistory "veneration-by-greeks-then-ominous-idol-to-medieval-clerics-same-aura-throughout"
}
"#;

// Stage 2: the secular cult of beauty (Renaissance, three centuries) --
// still ritual-based, but showing that basis "in its decline."
const CULT_OF_BEAUTY_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute basis
  declare attribute crisisStatus

  consumes {
    fact {ritual_uri} (cid: {ritual_cid}) {
      subject: artwork/venus
      predicate: basis
    }
  }
  produces {
    artwork/venus basis "secularized-cult-of-beauty"
    artwork/venus crisisStatus "ritual-basis-in-decline"
  }
}
"#;

// Stage 3: l'art pour l'art -- a DEFENSIVE reaction (a "theology of art,"
// a negative theology of "pure" art) to a SENSED crisis, arising once
// photography (mechanical reproduction) and the rise of socialism are
// simultaneous. The Mallarme attribution is entered here, explicitly
// flagged as unverified rather than either omitted or silently trusted.
const ART_POUR_ART_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute basis
  declare attribute crisisStatus
  declare attribute attribution
  declare attribute verificationStatus

  consumes {
    fact {cult_uri} (cid: {cult_cid}) {
      subject: artwork/venus
      predicate: crisisStatus
    }
  }
  produces {
    artwork/venus basis "l-art-pour-l-art-defensive-theology-of-pure-art"
    artwork/venus crisisStatus "sensed-approaching-crisis-photography-and-socialism-simultaneous-not-claimed-causal"
    argument/section_iv_attribution attribution "Mallarme first to take this position in poetry"
    argument/section_iv_attribution verificationStatus "unverified"
  }
}
"#;

// A SEPARATE, freshly-stated authenticity restatement -- the
// photographic-negative example -- with ZERO consumes of Section II's own
// commits. This is Benjamin re-deriving the point, not backward-citing it.
const RESTATEMENT_SRC: &str = r#"
commit argues {
  declare attribute restatement

  argument/section_iv_restatement restatement "from a photographic negative one can make any number of prints; to ask for the authentic print makes no sense"
}
"#;

// The pivot: consumes BOTH the genealogy's endpoint and the independent
// restatement -- two premises, neither of which is a backward citation
// into Section II.
const PIVOT_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {art_pour_art_uri} (cid: {art_pour_art_cid}) {
      subject: artwork/venus
      predicate: crisisStatus
    }
    fact {restatement_uri} (cid: {restatement_cid}) {
      subject: argument/section_iv_restatement
      predicate: restatement
    }
  }
  produces {
    argument/section_iv claim "the total function of art is reversed: instead of being based on ritual, it begins to be based on another practice, politics"
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
    let ritual_uri = "at://did:plc:form-reading-iv/org.jason-edelman.writtenworld.commit/rkey001";
    let ritual_cid = "bafyRitualVenus";
    let ritual = identify(RITUAL_SRC, ritual_uri, ritual_cid);
    println!("=== Stage 1: ritual origin, aura persisting across contradictory interpretations ===\n{RITUAL_SRC}");

    let cult_uri = "at://did:plc:form-reading-iv/org.jason-edelman.writtenworld.commit/rkey002";
    let cult_cid = "bafyCultOfBeauty";
    let cult_src = CULT_OF_BEAUTY_TEMPLATE
        .replace("{ritual_uri}", ritual_uri)
        .replace("{ritual_cid}", ritual_cid);
    let cult = identify(&cult_src, cult_uri, cult_cid);
    println!("=== Stage 2: secularized cult of beauty, ritual basis in decline ===\n{cult_src}");

    let art_pour_art_uri = "at://did:plc:form-reading-iv/org.jason-edelman.writtenworld.commit/rkey003";
    let art_pour_art_cid = "bafyArtPourArt";
    let art_pour_art_src = ART_POUR_ART_TEMPLATE
        .replace("{cult_uri}", cult_uri)
        .replace("{cult_cid}", cult_cid);
    let art_pour_art = identify(&art_pour_art_src, art_pour_art_uri, art_pour_art_cid);
    println!("=== Stage 3: l'art pour l'art, defensive reaction, Mallarme flagged unverified ===\n{art_pour_art_src}");

    let restatement_uri = "at://did:plc:form-reading-iv/org.jason-edelman.writtenworld.commit/rkey004";
    let restatement_cid = "bafyRestatement";
    let restatement = identify(RESTATEMENT_SRC, restatement_uri, restatement_cid);
    println!("=== Independent restatement: photographic negative, zero consumes of Section II ===\n{RESTATEMENT_SRC}");

    let pivot_uri = "at://did:plc:form-reading-iv/org.jason-edelman.writtenworld.commit/rkey005";
    let pivot_cid = "bafyPivotIV";
    let pivot_src = PIVOT_TEMPLATE
        .replace("{art_pour_art_uri}", art_pour_art_uri)
        .replace("{art_pour_art_cid}", art_pour_art_cid)
        .replace("{restatement_uri}", restatement_uri)
        .replace("{restatement_cid}", restatement_cid);
    let pivot = identify(&pivot_src, pivot_uri, pivot_cid);
    println!("=== The pivot: consumes the genealogy AND the independent restatement ===\n{pivot_src}");

    // Check 1: each genealogy stage consumes exactly the fact immediately
    // before it -- three real steps, not one commit spanning all three.
    assert_eq!(cult.commit.consumes.len(), 1);
    assert_eq!(art_pour_art.commit.consumes.len(), 1);
    println!(
        "\nCheck 1: cult.consumes.len() = {}, art_pour_art.consumes.len() = {} -- three real \
         genealogy steps (ritual -> secularized cult of beauty -> l'art pour l'art), each \
         consuming only the stage before it.",
        cult.commit.consumes.len(),
        art_pour_art.commit.consumes.len(),
    );

    // Check 2: the restatement is independently stated -- zero consumes,
    // confirming it is NOT a backward citation into Section II's own
    // commits, matching Benjamin's own re-derivation rather than citation.
    assert_eq!(
        restatement.commit.consumes.len(),
        0,
        "the photographic-negative restatement is freshly stated, not cited backward into Section II"
    );
    assert_eq!(pivot.commit.consumes.len(), 2);
    println!(
        "Check 2: restatement.consumes.len() = {} (independently stated); \
         pivot.consumes.len() = {} (genealogy endpoint + restatement) -- the pivot re-derives \
         the authenticity point with a fresh example rather than citing Section II directly.",
        restatement.commit.consumes.len(),
        pivot.commit.consumes.len(),
    );

    // Check 3: every genealogy stage remains independently real and
    // citable -- the ritual stage, materialized alone, still says exactly
    // what it said, same property as every prior file in this series.
    let ritual_alone = Materialized::from_identified_commits(&[ritual.clone()]);
    assert_eq!(
        ritual_alone.current_value("artwork/venus", "basis"),
        Some(&dmml::lower::TripleValue::Str("ritual-magical-then-religious".to_string()))
    );
    println!(
        "Check 3: ritual stage, materialized alone: {:?} -- real and citable regardless of \
         how many later stages were built on top of it.",
        ritual_alone.current_value("artwork/venus", "basis"),
    );

    // Check 4: the Mallarme attribution carries its OWN verificationStatus
    // fact, "unverified" -- entered into the log now, checkable and
    // revisable later, not blocking, not silently trusted either.
    let full_log = vec![ritual.clone(), cult.clone(), art_pour_art.clone(), restatement.clone(), pivot.clone()];
    let materialized = Materialized::from_identified_commits(&full_log);
    assert_eq!(
        materialized.current_value("argument/section_iv_attribution", "verificationStatus"),
        Some(&dmml::lower::TripleValue::Str("unverified".to_string()))
    );
    println!(
        "Check 4: current_value(argument/section_iv_attribution, verificationStatus) = {:?} \
         -- the Mallarme claim is IN the log now, openly marked unverified. A later commit \
         could consume this exact fact and revise verificationStatus once checked, same \
         consumes-the-old pattern as benjamin_understanding_evolves.rs -- not yet done here, \
         left for when the citation check actually happens.",
        materialized.current_value("argument/section_iv_attribution", "verificationStatus"),
    );

    println!(
        "\n=== done: a real three-stage genealogy, not a single jump (Check 1); the pivot \
         re-derives rather than backward-cites (Check 2); every stage stays independently \
         citable (Check 3); an unverified attribution is held openly in the log rather than \
         blocking or being silently trusted (Check 4). ==="
    );
}
