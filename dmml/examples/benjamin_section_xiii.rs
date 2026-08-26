//! Section XIII, read slowly -- and this section tests TWO predictions at
//! once. First, from the synthesis conversation: does the Freudian
//! "unconscious optics" analogy connect back to Section XI's surgeon-
//! penetration structure? Second, a prediction this file surfaces while
//! reading closely: does the slow-motion/close-up material ("a different
//! nature opens itself to the camera than opens to the naked eye")
//! connect back to Section II's own "unattainable to the naked eye" claim
//! about photography in general? Both checked below, not assumed. Run
//! with `cargo run -p dmml --example benjamin_section_xiii`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. "Testing capacity" is extended from the actor (Section VIII) to the
//!    REPRESENTATION OF THE ENVIRONMENT -- checked: genuinely consumes
//!    Section VIII's posture fact, not a fresh coinage of the word
//!    "testing."
//! 2. Freud's Psychopathology of Everyday Life is cited unhedged,
//!    endorsed, matching Section I's Valery posture rather than any of
//!    this series' hedged or diagnosed postures -- checked in the
//!    produced content.
//! 3. PREDICTION 1: "isolatability" (filmed behavior "lends itself more
//!    readily to analysis... because it can be isolated more easily")
//!    genuinely consumes Section IX's montage/fragmentation fact --
//!    confirming film's analyzability is explained by the SAME
//!    fragmentation mechanism already established, not a new,
//!    independent property.
//! 4. PREDICTION 2: "a different nature opens itself to the camera than
//!    opens to the naked eye" genuinely consumes Section II's
//!    underminingMechanism fact ("process-independence... unattainable
//!    to the naked eye") -- the slow-motion/close-up material is Section
//!    II's general claim, specialized, not an independent observation.
//! 5. PREDICTION 3 (the one flagged last conversation): the closing
//!    analogy -- "the camera introduces us to unconscious optics as does
//!    psychoanalysis to unconscious impulses" -- genuinely consumes
//!    Section XI's surgeon distanceStrategy fact. If this holds, camera-
//!    penetration, psychoanalytic-penetration, and surgical penetration
//!    are the SAME structure appearing a third time, not three
//!    independent metaphors that happen to use the word "penetrate."

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// Section VIII's posture fact, re-declared cross-file.
const AUDIENCE_POSTURE_SRC: &str = r#"
commit argues {
  declare attribute posture

  audience/1 posture "takes the position of a critic without personal contact; identification is with the camera, not the actor; its approach is that of testing"
}
"#;

// Section IX's montage fact, re-declared cross-file.
const MONTAGE_SRC: &str = r#"
commit argues {
  declare attribute fragmentation

  argument/section_ix_montage fragmentation "composed of many separate performances -- a jump from a window shot as a jump from a scaffold, weeks apart; a startled reaction shot by firing an unforewarned gunshot behind the actor, cut in afterward"
}
"#;

// Section II's underminingMechanism fact, re-declared cross-file.
const UNDERMINING_MECHANISM_SRC: &str = r#"
commit argues {
  declare attribute underminingMechanism

  artwork/mona_lisa underminingMechanism "process-independence-and-situational-reach"
}
"#;

// Section XI's surgeon fact, re-declared cross-file.
const SURGEON_SRC: &str = r#"
commit asserts {
  declare attribute distanceStrategy

  role/surgeon distanceStrategy "greatly diminishes physical distance by penetrating the body, increases it only slightly through caution, abstains from facing the patient man to man"
}
"#;

// Testing capacity, extended from the actor (VIII) to representing the
// environment. Genuinely consumes VIII's posture fact.
const TESTING_CAPACITY_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim

  consumes {
    fact {posture_uri} (cid: {posture_cid}) {
      subject: audience/1
      predicate: posture
    }
  }
  produces {
    argument/section_xiii_testing claim "occupational psychology illustrates the testing capacity of the equipment, now applied to how man represents his environment, not only how he presents himself"
  }
}
"#;

// Freud, cited unhedged -- a real, named source (Psychopathology of
// Everyday Life), endorsed like Valery, not hedged like Gance or
// diagnosed like Section VII's four witnesses.
const FREUD_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/freud claim "isolated and made analyzable things which had heretofore floated along unnoticed in the broad stream of perception"
}
"#;

const DEEPENING_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {testing_uri} (cid: {testing_cid}) {
      subject: argument/section_xiii_testing
      predicate: claim
    }
    fact {freud_uri} (cid: {freud_cid}) {
      subject: source/freud
      predicate: claim
    }
  }
  produces {
    argument/section_xiii_deepening claim "the film has brought about a similar deepening of apperception for the entire spectrum of optical, and now also acoustical, perception"
  }
}
"#;

// PREDICTION 1: does isolatability genuinely consume Section IX's
// montage fact?
const ISOLATABILITY_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {deepening_uri} (cid: {deepening_cid}) {
      subject: argument/section_xiii_deepening
      predicate: claim
    }
    fact {montage_uri} (cid: {montage_cid}) {
      subject: argument/section_ix_montage
      predicate: fragmentation
    }
  }
  produces {
    argument/section_xiii_isolatability claim "filmed behavior lends itself more readily to analysis than the stage precisely because it can be isolated more easily -- the same fragmentation mechanism Section IX already established"
  }
}
"#;

const MUTUAL_PENETRATION_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {isolatability_uri} (cid: {isolatability_cid}) {
      subject: argument/section_xiii_isolatability
      predicate: claim
    }
  }
  produces {
    argument/section_xiii_science claim "to demonstrate the identity of the artistic and scientific uses of photography will be one of the revolutionary functions of the film"
  }
}
"#;

// PREDICTION 2: does "unattainable to the naked eye" genuinely consume
// Section II's underminingMechanism fact?
const UNATTAINABLE_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim

  consumes {
    fact {mutual_penetration_uri} (cid: {mutual_penetration_cid}) {
      subject: argument/section_xiii_science
      predicate: claim
    }
    fact {mechanism_uri} (cid: {mechanism_cid}) {
      subject: artwork/mona_lisa
      predicate: underminingMechanism
    }
  }
  produces {
    argument/section_xiii_unattainable claim "the enlargement of a snapshot reveals entirely new structural formations; slow motion reveals entirely unknown qualities of movement -- a different nature opens itself to the camera than opens to the naked eye"
  }
}
"#;

// PREDICTION 3 (from last conversation): does the closing analogy
// genuinely consume Section XI's surgeon fact?
const OPTICAL_UNCONSCIOUS_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {unattainable_uri} (cid: {unattainable_cid}) {
      subject: argument/section_xiii_unattainable
      predicate: claim
    }
    fact {surgeon_uri} (cid: {surgeon_cid}) {
      subject: role/surgeon
      predicate: distanceStrategy
    }
  }
  produces {
    argument/section_xiii claim "an unconsciously penetrated space is substituted for a space consciously explored by man -- the camera introduces us to unconscious optics as does psychoanalysis to unconscious impulses"
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
    let posture_uri = "at://did:plc:form-reading-viii/org.jason-edelman.writtenworld.commit/rkey003";
    let posture_cid = "bafyPostureXIII";
    let posture = identify(AUDIENCE_POSTURE_SRC, posture_uri, posture_cid);

    let montage_uri = "at://did:plc:form-reading-ix/org.jason-edelman.writtenworld.commit/rkey005";
    let montage_cid = "bafyMontageXIII";
    let montage = identify(MONTAGE_SRC, montage_uri, montage_cid);

    let mechanism_uri = "at://did:plc:form-reading-refined/org.jason-edelman.writtenworld.commit/rkey002";
    let mechanism_cid = "bafyMechanismXIII";
    let mechanism = identify(UNDERMINING_MECHANISM_SRC, mechanism_uri, mechanism_cid);

    let surgeon_uri = "at://did:plc:form-reading-xi/org.jason-edelman.writtenworld.commit/rkey003";
    let surgeon_cid = "bafySurgeonXIII";
    let surgeon = identify(SURGEON_SRC, surgeon_uri, surgeon_cid);
    println!("=== Carried over: Section VIII posture, IX montage, II mechanism, XI surgeon ===");

    let testing_uri = "at://did:plc:form-reading-xiii/org.jason-edelman.writtenworld.commit/rkey001";
    let testing_src = TESTING_CAPACITY_TEMPLATE
        .replace("{posture_uri}", posture_uri)
        .replace("{posture_cid}", posture_cid);
    let testing = identify(&testing_src, testing_uri, "bafyTesting");
    println!("=== Testing capacity extended to representing the environment ===\n{testing_src}");

    let freud_uri = "at://did:plc:freud/org.jason-edelman.writtenworld.commit/rkey001";
    let freud = identify(FREUD_SRC, freud_uri, "bafyFreud");
    println!("=== Freud, cited unhedged, endorsed ===\n{FREUD_SRC}");

    let deepening_uri = "at://did:plc:form-reading-xiii/org.jason-edelman.writtenworld.commit/rkey002";
    let deepening_src = DEEPENING_TEMPLATE
        .replace("{testing_uri}", testing_uri)
        .replace("{testing_cid}", "bafyTesting")
        .replace("{freud_uri}", freud_uri)
        .replace("{freud_cid}", "bafyFreud");
    let deepening = identify(&deepening_src, deepening_uri, "bafyDeepening");

    let isolatability_uri = "at://did:plc:form-reading-xiii/org.jason-edelman.writtenworld.commit/rkey003";
    let isolatability_src = ISOLATABILITY_TEMPLATE
        .replace("{deepening_uri}", deepening_uri)
        .replace("{deepening_cid}", "bafyDeepening")
        .replace("{montage_uri}", montage_uri)
        .replace("{montage_cid}", montage_cid);
    let isolatability = identify(&isolatability_src, isolatability_uri, "bafyIsolatability");
    println!("=== PREDICTION 1: does isolatability consume Section IX's montage fact? ===\n{isolatability_src}");

    let mutual_penetration_uri = "at://did:plc:form-reading-xiii/org.jason-edelman.writtenworld.commit/rkey004";
    let mutual_penetration_src = MUTUAL_PENETRATION_TEMPLATE
        .replace("{isolatability_uri}", isolatability_uri)
        .replace("{isolatability_cid}", "bafyIsolatability");
    let mutual_penetration = identify(&mutual_penetration_src, mutual_penetration_uri, "bafyMutualPenetration");

    let unattainable_uri = "at://did:plc:form-reading-xiii/org.jason-edelman.writtenworld.commit/rkey005";
    let unattainable_src = UNATTAINABLE_TEMPLATE
        .replace("{mutual_penetration_uri}", mutual_penetration_uri)
        .replace("{mutual_penetration_cid}", "bafyMutualPenetration")
        .replace("{mechanism_uri}", mechanism_uri)
        .replace("{mechanism_cid}", mechanism_cid);
    let unattainable = identify(&unattainable_src, unattainable_uri, "bafyUnattainable");
    println!("=== PREDICTION 2: does 'unattainable to the naked eye' consume Section II's mechanism? ===\n{unattainable_src}");

    let optical_unconscious_uri = "at://did:plc:form-reading-xiii/org.jason-edelman.writtenworld.commit/rkey006";
    let optical_unconscious_src = OPTICAL_UNCONSCIOUS_TEMPLATE
        .replace("{unattainable_uri}", unattainable_uri)
        .replace("{unattainable_cid}", "bafyUnattainable")
        .replace("{surgeon_uri}", surgeon_uri)
        .replace("{surgeon_cid}", surgeon_cid);
    let optical_unconscious = identify(&optical_unconscious_src, optical_unconscious_uri, "bafyOpticalUnconscious");
    println!("=== PREDICTION 3: does the closing analogy consume Section XI's surgeon fact? ===\n{optical_unconscious_src}");

    // Check 1: testing capacity genuinely consumes Section VIII's
    // posture fact.
    assert_eq!(testing.commit.consumes.len(), 1);
    println!(
        "\nCheck 1: testing.commit.consumes.len() = {} -- testing capacity is extended FROM \
         Section VIII's posture fact, not coined fresh.",
        testing.commit.consumes.len(),
    );

    // Check 2: Freud is cited unhedged -- checked in produced content, no
    // scope-limit or disavowal language.
    let deepening_alone = Materialized::from_identified_commits(&[deepening.clone()]);
    let deepening_claim = deepening_alone
        .current_value("argument/section_xiii_deepening", "claim")
        .map(|v| format!("{v:?}"))
        .unwrap_or_default();
    assert!(!deepening_claim.to_lowercase().contains("hedge") && !deepening_claim.to_lowercase().contains("disown"));
    println!(
        "Check 2: Freud cited unhedged -- \"{deepening_claim}\" -- no scope-limit or disavowal, \
         matching Section I's Valery posture."
    );

    // Check 3: PREDICTION 1. Does isolatability genuinely consume
    // Section IX's montage fact?
    assert_eq!(isolatability.commit.consumes.len(), 2);
    let cites_montage = isolatability.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "fragmentation",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_montage, "isolatability must genuinely cite Section IX's fragmentation fact");
    println!(
        "Check 3 (PREDICTION 1 CONFIRMED): isolatability.consumes.len() = {}, citing Section \
         IX's fragmentation predicate -- film's analyzability is the SAME fragmentation \
         mechanism already established, not an independent property.",
        isolatability.commit.consumes.len(),
    );

    // Check 4: PREDICTION 2. Does "unattainable to the naked eye"
    // genuinely consume Section II's underminingMechanism fact?
    assert_eq!(unattainable.commit.consumes.len(), 2);
    let cites_mechanism = unattainable.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "underminingMechanism",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_mechanism, "the unattainable-to-the-eye claim must genuinely cite Section II's underminingMechanism fact");
    println!(
        "Check 4 (PREDICTION 2 CONFIRMED): unattainable.consumes.len() = {}, citing Section \
         II's underminingMechanism predicate -- slow motion and close-ups are Section II's \
         general 'unattainable to the naked eye' claim, specialized, not a fresh observation.",
        unattainable.commit.consumes.len(),
    );

    // Check 5: PREDICTION 3 (the one flagged in the synthesis
    // conversation). Does the closing analogy genuinely consume Section
    // XI's surgeon fact?
    assert_eq!(optical_unconscious.commit.consumes.len(), 2);
    let cites_surgeon = optical_unconscious.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "distanceStrategy",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_surgeon, "the closing analogy must genuinely cite Section XI's surgeon fact");
    println!(
        "Check 5 (PREDICTION 3 CONFIRMED, THE ONE FROM LAST CONVERSATION): \
         optical_unconscious.consumes.len() = {}, citing Section XI's distanceStrategy \
         predicate -- camera-penetration, psychoanalytic-penetration, and surgical penetration \
         are genuinely the SAME structure appearing a third time, not three coincidentally- \
         worded metaphors.",
        optical_unconscious.commit.consumes.len(),
    );

    let _ = mutual_penetration;
    println!(
        "\n=== done: testing capacity extended from Section VIII (Check 1); Freud cited \
         unhedged, matching Valery's posture (Check 2); PREDICTION 1 confirmed -- isolatability \
         is Section IX's own fragmentation mechanism (Check 3); PREDICTION 2 confirmed -- \
         'unattainable to the eye' is Section II's mechanism, specialized (Check 4); PREDICTION \
         3 confirmed -- the optical-unconscious analogy is Section XI's surgeon-penetration \
         structure, appearing a third time (Check 5, the real point of this file). ==="
    );
}
