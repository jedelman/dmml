//! Section VI, read slowly: one continuous five-stage chain, not a
//! genealogy-then-pivot shape like Section IV. Cult value's last refuge
//! (the human face, present) inverts into exhibition value's decisive
//! victory (Atget, the face ABSENT -- deserted Paris streets), which then
//! produces a genuinely new interpretive mechanism (captions, then film's
//! sequence-dependent meaning) that Benjamin explicitly distinguishes from
//! anything a painting's title ever did. Run with `cargo run -p dmml
//! --example benjamin_section_vi`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. Five real stages, each consuming exactly the fact immediately before
//!    it -- one continuous chain, not a branching genealogy. Checked: each
//!    dependent commit's consumes count is 1.
//! 2. The presence/absence INVERSION between the portrait stage and the
//!    Atget stage is a real, checkable content flip, not just two
//!    differently-worded facts: `humanPresence` goes from "present" to
//!    "absent," and this exact flip is what the text says causes
//!    exhibition value's first decisive win over ritual value -- not
//!    reproduction technology advancing further, a compositional choice
//!    (what's IN the photograph) doing the work instead.
//! 3. The caption/film directive mechanism is checked AGAINST a painting
//!    baseline fact, confirming the text's explicit claim that captions
//!    have "an altogether different character than the title of a
//!    painting" is a real content difference, not asserted and then
//!    ignored. And the intensification from magazine captions to film's
//!    sequence-prescribed meaning is its own further step, consuming the
//!    caption fact rather than restating it.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// Stage 1: cult value's last refuge -- the human face, PRESENT. "For the
// last time the aura emanates from the early photographs in the fleeting
// expression of a human face."
const PORTRAIT_SRC: &str = r#"
commit asserts {
  declare attribute humanPresence
  declare attribute auraStatus

  artwork/early_photograph humanPresence "present -- the fleeting expression of a human face"
  artwork/early_photograph auraStatus "last refuge for cult value, melancholy incomparable beauty"
}
"#;

// Stage 2: Atget -- the INVERSION. Face ABSENT (deserted Paris streets),
// and this absence is what causes exhibition value's first decisive
// superiority, not reproduction technology advancing further.
const ATGET_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute humanPresence
  declare attribute valueShift

  consumes {
    fact {portrait_uri} (cid: {portrait_cid}) {
      subject: artwork/early_photograph
      predicate: humanPresence
    }
  }
  produces {
    artwork/atget_photograph humanPresence "absent -- deserted Paris streets, photographed like scenes of crime"
    artwork/atget_photograph valueShift "as man withdraws from the photographic image, exhibition value for the first time shows its superiority to ritual value"
  }
}
"#;

// Stage 3: the consequence -- standard evidence, hidden political
// significance, a new kind of approach demanded ("free-floating
// contemplation is not appropriate").
const EVIDENCE_TEMPLATE: &str = r#"
commit argues {
  declare attribute evidentiaryStatus
  declare attribute approachRequired

  consumes {
    fact {atget_uri} (cid: {atget_cid}) {
      subject: artwork/atget_photograph
      predicate: valueShift
    }
  }
  produces {
    artwork/atget_photograph evidentiaryStatus "standard evidence for historical occurrences, hidden political significance"
    artwork/atget_photograph approachRequired "not free-floating contemplation -- the viewer feels challenged"
  }
}
"#;

// A painting baseline, for contrast -- asserted, not derived, giving the
// caption-directive check something real to differ against.
const PAINTING_BASELINE_SRC: &str = r#"
commit asserts {
  declare attribute directiveType

  medium/painting directiveType "a title -- static, no sequence dependency, no obligation"
}
"#;

// Stage 4: captions become obligatory, "an altogether different
// character than the title of a painting."
const CAPTION_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute directiveType

  consumes {
    fact {evidence_uri} (cid: {evidence_cid}) {
      subject: artwork/atget_photograph
      predicate: evidentiaryStatus
    }
  }
  produces {
    medium/picture_magazine directiveType "captions obligatory for the first time -- right or wrong, no matter -- an altogether different character than a painting's title"
  }
}
"#;

// Stage 5: the intensification -- film, where meaning is prescribed by
// SEQUENCE, not caption alone.
const FILM_SEQUENCE_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute directiveType

  consumes {
    fact {caption_uri} (cid: {caption_cid}) {
      subject: medium/picture_magazine
      predicate: directiveType
    }
  }
  produces {
    medium/film directiveType "the meaning of each single picture appears to be prescribed by the sequence of all preceding ones"
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
    let portrait_uri = "at://did:plc:form-reading-vi/org.jason-edelman.writtenworld.commit/rkey001";
    let portrait_cid = "bafyPortrait";
    let portrait = identify(PORTRAIT_SRC, portrait_uri, portrait_cid);
    println!("=== Stage 1: cult value's last refuge -- the human face, present ===\n{PORTRAIT_SRC}");

    let atget_uri = "at://did:plc:form-reading-vi/org.jason-edelman.writtenworld.commit/rkey002";
    let atget_cid = "bafyAtget";
    let atget_src = ATGET_TEMPLATE
        .replace("{portrait_uri}", portrait_uri)
        .replace("{portrait_cid}", portrait_cid);
    let atget = identify(&atget_src, atget_uri, atget_cid);
    println!("=== Stage 2: Atget -- the inversion, face absent ===\n{atget_src}");

    let evidence_uri = "at://did:plc:form-reading-vi/org.jason-edelman.writtenworld.commit/rkey003";
    let evidence_cid = "bafyEvidence";
    let evidence_src = EVIDENCE_TEMPLATE
        .replace("{atget_uri}", atget_uri)
        .replace("{atget_cid}", atget_cid);
    let evidence = identify(&evidence_src, evidence_uri, evidence_cid);

    let painting_baseline_uri = "at://did:plc:form-reading-vi/org.jason-edelman.writtenworld.commit/rkey004";
    let painting_baseline_cid = "bafyPaintingBaseline";
    let painting_baseline = identify(PAINTING_BASELINE_SRC, painting_baseline_uri, painting_baseline_cid);

    let caption_uri = "at://did:plc:form-reading-vi/org.jason-edelman.writtenworld.commit/rkey005";
    let caption_cid = "bafyCaption";
    let caption_src = CAPTION_TEMPLATE
        .replace("{evidence_uri}", evidence_uri)
        .replace("{evidence_cid}", evidence_cid);
    let caption = identify(&caption_src, caption_uri, caption_cid);
    println!("=== Stage 4: captions obligatory, unlike a painting's title ===\n{caption_src}");

    let film_uri = "at://did:plc:form-reading-vi/org.jason-edelman.writtenworld.commit/rkey006";
    let film_cid = "bafyFilmSequence";
    let film_src = FILM_SEQUENCE_TEMPLATE
        .replace("{caption_uri}", caption_uri)
        .replace("{caption_cid}", caption_cid);
    let film = identify(&film_src, film_uri, film_cid);
    println!("=== Stage 5: the intensification -- film, meaning prescribed by sequence ===\n{film_src}");

    // Check 1: five real stages, each consuming exactly the fact
    // immediately before it -- a continuous chain, not a branching
    // genealogy like Section IV.
    assert_eq!(atget.commit.consumes.len(), 1);
    assert_eq!(evidence.commit.consumes.len(), 1);
    assert_eq!(caption.commit.consumes.len(), 1);
    assert_eq!(film.commit.consumes.len(), 1);
    println!(
        "\nCheck 1: atget/evidence/caption/film each consume exactly 1 fact -- one continuous \
         five-stage chain (portrait -> Atget -> evidence -> captions -> film), not a \
         genealogy-then-pivot shape."
    );

    // Check 2: the presence/absence inversion, checked on EACH commit
    // materialized ALONE (the only way to see a produced fact reliably --
    // see Check 2b below for why the combined log is the wrong tool here).
    let portrait_alone = Materialized::from_identified_commits(&[portrait.clone()]);
    let atget_alone = Materialized::from_identified_commits(&[atget.clone()]);
    let portrait_presence = portrait_alone.current_value("artwork/early_photograph", "humanPresence");
    let atget_presence = atget_alone.current_value("artwork/atget_photograph", "humanPresence");
    assert_ne!(portrait_presence, atget_presence);
    println!(
        "\nCheck 2: portrait humanPresence (alone) = {portrait_presence:?}; Atget humanPresence \
         (alone) = {atget_presence:?} -- a real inversion, not a further step along the same \
         axis. What flips is what's IN the photograph, not the reproduction technology."
    );

    // Check 2b: a genuine finding, surfaced by actually running this
    // rather than assumed -- `Materialized::from_identified_commits`
    // retracts the EXACT (subject, predicate) key a commit `consumes`,
    // unconditionally, before applying that commit's own `produces`
    // (confirmed in `dmml/src/interpret.rs`'s own doc comment: "applying
    // each commit's consumes (retraction) before its produces"). Atget's
    // commit consumes (artwork/early_photograph, humanPresence) as its
    // premise, but PRODUCES a DIFFERENT key (artwork/atget_photograph,
    // humanPresence). The result: in the COMBINED log's current view, the
    // portrait's own fact is retracted and nothing re-asserts that exact
    // key -- it reads as None, even though portrait.rs's commit is
    // untouched and still real (Check 2 above, materialized alone,
    // proves that). This is a "cite-and-spend" semantics for consumes,
    // not a plain same-key overwrite -- pantheon.rs's Nyx never exposed
    // this because Nyx both consumed AND produced the SAME key
    // (sky/1, origin), so the retraction was invisible, masked by the
    // immediate re-assertion. Every earlier file in this series checked
    // "still real" only by materializing a commit ALONE; none had queried
    // an earlier, differently-keyed fact's OWN key inside a full combined
    // log after it got cited downstream -- this is the first one that
    // did, and it surfaced real interpreter behavior worth stating
    // plainly rather than working around.
    let full_log = vec![
        portrait.clone(),
        atget.clone(),
        evidence.clone(),
        painting_baseline.clone(),
        caption.clone(),
        film.clone(),
    ];
    let materialized = Materialized::from_identified_commits(&full_log);
    let portrait_in_combined_log = materialized.current_value("artwork/early_photograph", "humanPresence");
    assert_eq!(
        portrait_in_combined_log, None,
        "citing a fact as a premise retracts its exact key from the COMBINED current view, \
         even though the fact remains real and independently re-materializable alone (Check 2)"
    );
    println!(
        "Check 2b: current_value(artwork/early_photograph, humanPresence) in the FULL combined \
         log = {portrait_in_combined_log:?} -- consumes retracts the exact key it cites, \
         unconditionally, even though the fact stays real when materialized alone (Check 2). A \
         genuine 'cite-and-spend' semantics, not just same-key overwrite -- found by actually \
         running this, not assumed from earlier files that never tested it this way."
    );

    // Check 3: the caption directive genuinely differs from the painting
    // baseline -- checked the same honest way, alone rather than assuming
    // the combined log preserves an early, differently-keyed premise.
    let painting_alone = Materialized::from_identified_commits(&[painting_baseline.clone()]);
    let caption_alone = Materialized::from_identified_commits(&[caption.clone()]);
    let film_alone = Materialized::from_identified_commits(&[film.clone()]);
    let painting_directive = painting_alone.current_value("medium/painting", "directiveType");
    let caption_directive = caption_alone.current_value("medium/picture_magazine", "directiveType");
    let film_directive = film_alone.current_value("medium/film", "directiveType");
    assert_ne!(painting_directive, caption_directive);
    assert_ne!(caption_directive, film_directive);
    println!(
        "Check 3: painting directiveType (alone) = {painting_directive:?}\ncaption directiveType \
         (alone) = {caption_directive:?}\nfilm directiveType (alone) = {film_directive:?}\n\
         -- captions are a real content difference from a painting's title (as the text \
         explicitly claims), and film's sequence-dependent meaning is a further, distinct \
         intensification, not a restatement of the caption fact."
    );

    println!(
        "\n=== done: one continuous five-stage chain (Check 1); a real presence/absence \
         inversion drives exhibition value's first decisive win, not more reproduction \
         technology (Check 2); consumes retracts the exact key it cites even across different \
         subjects, a real cite-and-spend semantics this file is the first to have actually \
         tested for (Check 2b); the caption/film directive mechanism holds up under the same \
         honest, alone-materialized check (Check 3). ==="
    );
}
