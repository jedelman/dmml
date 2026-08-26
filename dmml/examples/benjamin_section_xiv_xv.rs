//! Sections XIV-XV, read together as one argument: Dadaism as the first
//! DELIBERATE instance of aura-destruction (every prior section described
//! aura's decline as an external historical process; Dada does it to
//! itself, on purpose, as strategy), then a generalization through
//! distraction/concentration to architecture, testing TWO major threads
//! this series has been carrying: does "quantity transmuted into
//! quality" genuinely echo Section V's own language, and does
//! architecture's distraction-compatible collective reception genuinely
//! consume Section XII's simultaneous-collective-capacity fact? The
//! closing line -- "the public is an examiner, but an absent-minded
//! one" -- is checked as the fusion point of Section VIII's testing-
//! posture and this section's own distraction material, not a fresh
//! coinage. Run with `cargo run -p dmml --example
//! benjamin_section_xiv_xv`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. Dada's aura-destruction is modeled with an explicit `agency: "
//!    deliberate"` fact, checked to differ in kind from Section II's
//!    passive/historical framing of aura's decline -- the first time in
//!    this series an artist movement is shown INTENDING the same outcome
//!    the essay elsewhere describes as happening TO art.
//! 2. Duhamel's STRUCTURAL observation ("my thoughts have been replaced
//!    by moving images") is cited and treated as accurate despite his own
//!    hostility to film -- a citation posture distinct from every prior
//!    one: correct content, disregarded evaluative stance.
//! 3. PREDICTION: does "the mass is a matrix... quantity has been
//!    transmuted into quality" genuinely consume Section V's
//!    `qualitativeShift` fact (re-declared cross-file)? Checked directly.
//! 4. Duhamel's SECOND, hostile quote ("a pastime for helots...") is
//!    cited then explicitly DISMISSED as "the same ancient lament... a
//!    commonplace" -- correct as a species of a known complaint,
//!    therefore analytically unhelpful. A seventh citation posture,
//!    checked in the produced content.
//! 5. Concentration and distraction are modeled as an explicit CHIASMUS
//!    -- one commit, two paired facts (viewer absorbed into the work vs.
//!    the mass absorbing the work) -- the doubling pattern from Sections
//!    V, IX, XI, applied to a genuine reversal this time, not a mirror.
//! 6. THE BIG PREDICTION: does architecture's distraction-compatible
//!    collective reception genuinely consume Section XII's
//!    `simultaneousCollectiveCapacity` fact? If confirmed, Section XII's
//!    "architecture, epic, film -- not painting" claim and this section's
//!    architecture-as-paradigm claim are the same fact, not two
//!    independent observations that happen to both mention architecture.
//! 7. THE FUSION TEST: does the closing line -- "the public is an
//!    examiner, but an absent-minded one" -- genuinely consume BOTH
//!    Section VIII's posture fact (critic, testing) AND this section's
//!    own distraction material? If so, the essay's two major threads
//!    (testing-posture, distraction) are shown fusing at the essay's own
//!    penultimate claim before the Epilogue, not running in parallel to
//!    the end.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// Section V's qualitativeShift fact, re-declared cross-file.
const QUALITATIVE_SHIFT_SRC: &str = r#"
commit argues {
  declare attribute qualitativeShift

  argument/section_v qualitativeShift "the quantitative shift between cult and exhibition value turned into a qualitative transformation of art's nature"
}
"#;

// Section XII's simultaneousCollectiveCapacity fact, re-declared
// cross-file.
const COLLECTIVE_CAPACITY_SRC: &str = r#"
commit argues {
  declare attribute simultaneousCollectiveCapacity

  medium/painting simultaneousCollectiveCapacity "none -- painting is in no position to present an object for simultaneous collective experience, unlike architecture, the epic poem, or film"
}
"#;

// Section VIII's posture fact, re-declared cross-file.
const AUDIENCE_POSTURE_SRC: &str = r#"
commit argues {
  declare attribute posture

  audience/1 posture "takes the position of a critic without personal contact; identification is with the camera, not the actor; its approach is that of testing"
}
"#;

// Art creates demands only a LATER art form can satisfy -- Dada
// anticipated film, unconsciously.
const DEMAND_BEFORE_SATISFACTION_SRC: &str = r#"
commit asserts {
  declare attribute claim

  argument/section_xiv_demand claim "one of the foremost tasks of art has always been the creation of a demand which could be fully satisfied only later -- Dadaism attempted by pictorial and literary means the effects the public today seeks in film"
}
"#;

// Dada's aura-destruction -- DELIBERATE, not passive/historical, unlike
// Section II's framing.
const DADA_AURA_DESTRUCTION_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim
  declare attribute agency

  consumes {
    fact {demand_uri} (cid: {demand_cid}) {
      subject: argument/section_xiv_demand
      predicate: claim
    }
  }
  produces {
    argument/section_xiv_dada claim "what the Dadaists intended and achieved was a relentless destruction of the aura of their creations, which they branded as reproductions with the very means of production"
    argument/section_xiv_dada agency "deliberate -- unlike Section II's passive, historical framing of aura's decline, this is a strategy an artist movement pursues on purpose"
  }
}
"#;

// Duhamel's structural observation, correct despite his hostility.
const DUHAMEL_STRUCTURAL_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/duhamelStructural claim "I can no longer think what I want to think. My thoughts have been replaced by moving images"
}
"#;

const TACTILE_SHOCK_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim

  consumes {
    fact {dada_uri} (cid: {dada_cid}) {
      subject: argument/section_xiv_dada
      predicate: claim
    }
    fact {duhamel_uri} (cid: {duhamel_cid}) {
      subject: source/duhamelStructural
      predicate: claim
    }
  }
  produces {
    argument/section_xiv claim "the work of art became an instrument of ballistics, hitting the spectator like a bullet, acquiring a tactile quality -- this promoted a demand for film, whose shock effect the Dadaists could only approach by moral means"
  }
}
"#;

// PREDICTION: does this genuinely consume Section V's qualitativeShift?
const QUANTITY_QUALITY_XV_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim

  consumes {
    fact {tactile_uri} (cid: {tactile_cid}) {
      subject: argument/section_xiv
      predicate: claim
    }
    fact {qualitative_shift_uri} (cid: {qualitative_shift_cid}) {
      subject: argument/section_v
      predicate: qualitativeShift
    }
  }
  produces {
    argument/section_xv claim "the mass is a matrix from which all traditional behavior toward works of art issues today in a new form -- quantity has been transmuted into quality"
  }
}
"#;

// Duhamel's second, hostile quote -- cited then DISMISSED as a
// commonplace, correct-as-a-species-of-a-known-complaint.
const DUHAMEL_HOSTILE_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/duhamelHostile claim "a pastime for helots, a diversion for uneducated, wretched, worn-out creatures who are consumed by their worries"
}
"#;

const DISMISSAL_TEMPLATE: &str = r#"
commit argues {
  declare attribute verdict

  consumes {
    fact {duhamel_hostile_uri} (cid: {duhamel_hostile_cid}) {
      subject: source/duhamelHostile
      predicate: claim
    }
  }
  produces {
    argument/section_xv_dismissal verdict "this is at bottom the same ancient lament that the masses seek distraction whereas art demands concentration -- a commonplace, correct as a species of a known complaint, but not a platform for analysis"
  }
}
"#;

// The chiasmus: ONE commit, TWO paired, contrasted facts.
const CHIASMUS_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute absorptionDirection

  consumes {
    fact {quantity_quality_uri} (cid: {quantity_quality_cid}) {
      subject: argument/section_xv
      predicate: claim
    }
    fact {dismissal_uri} (cid: {dismissal_cid}) {
      subject: argument/section_xv_dismissal
      predicate: verdict
    }
  }
  produces {
    reception/concentration absorptionDirection "the viewer is absorbed INTO the work -- enters it, the way legend tells of the Chinese painter viewing his finished painting"
    reception/distraction absorptionDirection "the distracted mass ABSORBS the work -- the direction is reversed"
  }
}
"#;

// THE BIG PREDICTION: does architecture genuinely consume Section XII's
// simultaneousCollectiveCapacity fact?
const ARCHITECTURE_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim

  consumes {
    fact {chiasmus_uri} (cid: {chiasmus_cid}) {
      subject: reception/distraction
      predicate: absorptionDirection
    }
    fact {collective_capacity_uri} (cid: {collective_capacity_cid}) {
      subject: medium/painting
      predicate: simultaneousCollectiveCapacity
    }
  }
  produces {
    medium/architecture claim "architecture has always represented the prototype of a work of art the reception of which is consummated by a collectivity in a state of distraction -- the laws of its reception are most instructive"
  }
}
"#;

// The asymmetric twofold: habit/touch governs BOTH sides, not two equal
// partners.
const TWOFOLD_APPROPRIATION_TEMPLATE: &str = r#"
commit argues {
  declare attribute appropriationMode
  declare attribute symmetry

  consumes {
    fact {architecture_uri} (cid: {architecture_cid}) {
      subject: medium/architecture
      predicate: claim
    }
  }
  produces {
    medium/architecture appropriationMode "by use and by perception -- touch and sight -- but habit, the tactile side's own mode, determines to a large extent even optical reception too"
    medium/architecture symmetry "asymmetric -- unlike Section II's, III's, and VIII's twofolds, one side here governs the other rather than standing as an equal, independent partner"
  }
}
"#;

// THE FUSION TEST: does this genuinely consume BOTH Section VIII's
// posture fact AND the distraction material built in this file?
const FINAL_PAYOFF_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim

  consumes {
    fact {posture_uri} (cid: {posture_cid}) {
      subject: audience/1
      predicate: posture
    }
    fact {appropriation_uri} (cid: {appropriation_cid}) {
      subject: medium/architecture
      predicate: appropriationMode
    }
  }
  produces {
    argument/section_xv_final claim "the film makes cult value recede not only by putting the public in the position of the critic, but by the fact that this position requires no attention -- the public is an examiner, but an absent-minded one"
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
    let qualitative_shift_uri = "at://did:plc:form-reading-v/org.jason-edelman.writtenworld.commit/rkey002";
    let qualitative_shift_cid = "bafyQualitativeShiftXIV";
    let qualitative_shift = identify(QUALITATIVE_SHIFT_SRC, qualitative_shift_uri, qualitative_shift_cid);

    let collective_capacity_uri = "at://did:plc:form-reading-xii/org.jason-edelman.writtenworld.commit/rkey004";
    let collective_capacity_cid = "bafyCollectiveCapacityXIV";
    let collective_capacity = identify(COLLECTIVE_CAPACITY_SRC, collective_capacity_uri, collective_capacity_cid);

    let posture_uri = "at://did:plc:form-reading-viii/org.jason-edelman.writtenworld.commit/rkey003";
    let posture_cid = "bafyPostureXIV";
    let posture = identify(AUDIENCE_POSTURE_SRC, posture_uri, posture_cid);
    println!("=== Carried over: Section V's qualitativeShift, XII's collective capacity, VIII's posture ===");

    let demand_uri = "at://did:plc:form-reading-xiv/org.jason-edelman.writtenworld.commit/rkey001";
    let demand = identify(DEMAND_BEFORE_SATISFACTION_SRC, demand_uri, "bafyDemand");
    println!("=== Art creates demand only a later form can satisfy -- Dada anticipated film ===\n{DEMAND_BEFORE_SATISFACTION_SRC}");

    let dada_uri = "at://did:plc:form-reading-xiv/org.jason-edelman.writtenworld.commit/rkey002";
    let dada_src = DADA_AURA_DESTRUCTION_TEMPLATE
        .replace("{demand_uri}", demand_uri)
        .replace("{demand_cid}", "bafyDemand");
    let dada = identify(&dada_src, dada_uri, "bafyDada");
    println!("=== Dada's aura-destruction: DELIBERATE, unlike Section II's passive framing ===\n{dada_src}");

    let duhamel_structural_uri = "at://did:plc:duhamelStructural/org.jason-edelman.writtenworld.commit/rkey001";
    let duhamel_structural = identify(DUHAMEL_STRUCTURAL_SRC, duhamel_structural_uri, "bafyDuhamelStructural");
    println!("=== Duhamel's structural observation, correct despite his hostility ===\n{DUHAMEL_STRUCTURAL_SRC}");

    let tactile_uri = "at://did:plc:form-reading-xiv/org.jason-edelman.writtenworld.commit/rkey003";
    let tactile_src = TACTILE_SHOCK_TEMPLATE
        .replace("{dada_uri}", dada_uri)
        .replace("{dada_cid}", "bafyDada")
        .replace("{duhamel_uri}", duhamel_structural_uri)
        .replace("{duhamel_cid}", "bafyDuhamelStructural");
    let tactile = identify(&tactile_src, tactile_uri, "bafyTactile");

    let xv_uri = "at://did:plc:form-reading-xv/org.jason-edelman.writtenworld.commit/rkey001";
    let xv_src = QUANTITY_QUALITY_XV_TEMPLATE
        .replace("{tactile_uri}", tactile_uri)
        .replace("{tactile_cid}", "bafyTactile")
        .replace("{qualitative_shift_uri}", qualitative_shift_uri)
        .replace("{qualitative_shift_cid}", qualitative_shift_cid);
    let xv = identify(&xv_src, xv_uri, "bafyXV");
    println!("=== PREDICTION: does 'quantity into quality' consume Section V's qualitativeShift? ===\n{xv_src}");

    let duhamel_hostile_uri = "at://did:plc:duhamelHostile/org.jason-edelman.writtenworld.commit/rkey001";
    let duhamel_hostile = identify(DUHAMEL_HOSTILE_SRC, duhamel_hostile_uri, "bafyDuhamelHostile");

    let dismissal_uri = "at://did:plc:form-reading-xv/org.jason-edelman.writtenworld.commit/rkey002";
    let dismissal_src = DISMISSAL_TEMPLATE
        .replace("{duhamel_hostile_uri}", duhamel_hostile_uri)
        .replace("{duhamel_hostile_cid}", "bafyDuhamelHostile");
    let dismissal = identify(&dismissal_src, dismissal_uri, "bafyDismissal");
    println!("=== Duhamel's hostile quote, cited then DISMISSED as a commonplace ===\n{DUHAMEL_HOSTILE_SRC}{dismissal_src}");

    let chiasmus_uri = "at://did:plc:form-reading-xv/org.jason-edelman.writtenworld.commit/rkey003";
    let chiasmus_src = CHIASMUS_TEMPLATE
        .replace("{quantity_quality_uri}", xv_uri)
        .replace("{quantity_quality_cid}", "bafyXV")
        .replace("{dismissal_uri}", dismissal_uri)
        .replace("{dismissal_cid}", "bafyDismissal");
    let chiasmus = identify(&chiasmus_src, chiasmus_uri, "bafyChiasmus");
    println!("=== The chiasmus: concentration enters the work, distraction absorbs it ===\n{chiasmus_src}");

    let architecture_uri = "at://did:plc:form-reading-xv/org.jason-edelman.writtenworld.commit/rkey004";
    let architecture_src = ARCHITECTURE_TEMPLATE
        .replace("{chiasmus_uri}", chiasmus_uri)
        .replace("{chiasmus_cid}", "bafyChiasmus")
        .replace("{collective_capacity_uri}", collective_capacity_uri)
        .replace("{collective_capacity_cid}", collective_capacity_cid);
    let architecture = identify(&architecture_src, architecture_uri, "bafyArchitecture");
    println!("=== THE BIG PREDICTION: does architecture consume Section XII's collective capacity? ===\n{architecture_src}");

    let appropriation_uri = "at://did:plc:form-reading-xv/org.jason-edelman.writtenworld.commit/rkey005";
    let appropriation_src = TWOFOLD_APPROPRIATION_TEMPLATE
        .replace("{architecture_uri}", architecture_uri)
        .replace("{architecture_cid}", "bafyArchitecture");
    let appropriation = identify(&appropriation_src, appropriation_uri, "bafyAppropriation");
    println!("=== The asymmetric twofold: habit governs both touch AND sight ===\n{appropriation_src}");

    let final_uri = "at://did:plc:form-reading-xv/org.jason-edelman.writtenworld.commit/rkey006";
    let final_src = FINAL_PAYOFF_TEMPLATE
        .replace("{posture_uri}", posture_uri)
        .replace("{posture_cid}", posture_cid)
        .replace("{appropriation_uri}", appropriation_uri)
        .replace("{appropriation_cid}", "bafyAppropriation");
    let final_commit = identify(&final_src, final_uri, "bafyFinal");
    println!("=== THE FUSION TEST: does this consume BOTH VIII's posture AND the distraction material? ===\n{final_src}");

    // Check 1: Dada's aura-destruction is flagged deliberate, distinct
    // from Section II's passive framing.
    let dada_alone = Materialized::from_identified_commits(&[dada.clone()]);
    let agency = dada_alone.current_value("argument/section_xiv_dada", "agency");
    assert!(agency.map(|v| format!("{v:?}").contains("deliberate")).unwrap_or(false));
    println!(
        "\nCheck 1: agency = {agency:?} -- Dada's aura-destruction is flagged deliberate, the \
         first artist movement in this essay shown INTENDING what elsewhere happens TO art."
    );

    // Check 2: PREDICTION. Does Section XV's quantity-into-quality
    // genuinely consume Section V's qualitativeShift?
    assert_eq!(xv.commit.consumes.len(), 2);
    let cites_v = xv.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "qualitativeShift",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_v, "Section XV's quantity-into-quality claim must genuinely cite Section V's qualitativeShift fact");
    println!(
        "Check 2 (CONFIRMED): xv.commit.consumes.len() = {}, citing Section V's \
         qualitativeShift predicate -- the verbatim echo is a real citation, not just repeated \
         phrasing.",
        xv.commit.consumes.len(),
    );

    // Check 3: Duhamel's hostile quote is cited then explicitly
    // dismissed, checked in the produced content.
    let dismissal_alone = Materialized::from_identified_commits(&[dismissal.clone()]);
    let verdict = dismissal_alone
        .current_value("argument/section_xv_dismissal", "verdict")
        .map(|v| format!("{v:?}"))
        .unwrap_or_default();
    assert!(verdict.contains("commonplace"));
    println!("Check 3: verdict contains \"commonplace\" -- Duhamel's complaint is correct-as-a-species-of-a-known-lament but explicitly denied analytical value, a seventh citation posture in this series.");

    // Check 4: the chiasmus produces both directions from one commit.
    let chiasmus_predicates: std::collections::BTreeSet<&str> =
        chiasmus.commit.produces.iter().map(|t| t.predicate.as_str()).collect();
    assert!(chiasmus_predicates.contains("absorptionDirection"));
    let materialized_chiasmus = Materialized::from_identified_commits(&[chiasmus.clone()]);
    println!(
        "Check 4: concentration = {:?}; distraction = {:?} -- one commit, a real reversal, not \
         a mirror.",
        materialized_chiasmus.current_value("reception/concentration", "absorptionDirection"),
        materialized_chiasmus.current_value("reception/distraction", "absorptionDirection"),
    );

    // Check 5: THE BIG PREDICTION. Does architecture genuinely consume
    // Section XII's simultaneousCollectiveCapacity fact?
    assert_eq!(architecture.commit.consumes.len(), 2);
    let cites_xii = architecture.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "simultaneousCollectiveCapacity",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_xii, "architecture's claim must genuinely cite Section XII's simultaneousCollectiveCapacity fact");
    println!(
        "Check 5 (BIG PREDICTION CONFIRMED): architecture.commit.consumes.len() = {}, citing \
         Section XII's simultaneousCollectiveCapacity predicate -- Section XII's \"architecture, \
         epic, film, not painting\" claim and this section's architecture-as-paradigm claim are \
         the SAME fact, not two independent observations.",
        architecture.commit.consumes.len(),
    );

    // Check 6: THE FUSION TEST. Does the final payoff genuinely consume
    // BOTH Section VIII's posture fact AND the distraction material?
    assert_eq!(final_commit.commit.consumes.len(), 2);
    let cites_posture = final_commit.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "posture",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_posture, "the final payoff must genuinely cite Section VIII's posture fact");
    println!(
        "Check 6 (FUSION CONFIRMED): final_commit.commit.consumes.len() = {}, citing Section \
         VIII's posture predicate -- \"the public is an examiner, but an absent-minded one\" \
         genuinely fuses Section VIII's testing-posture with this section's own distraction \
         material, not a fresh coinage arriving independently at the essay's close.",
        final_commit.commit.consumes.len(),
    );

    println!(
        "\n=== done: Dada's aura-destruction is deliberate, not passive (Check 1); the quantity- \
         into-quality echo of Section V is a real citation (Check 2); Duhamel's hostile quote is \
         dismissed as a commonplace, a seventh posture (Check 3); concentration/distraction is a \
         real reversal from one commit (Check 4); architecture's claim IS Section XII's claim \
         (Check 5, a major payoff); the essay's two major threads fuse at its penultimate claim \
         (Check 6, the real point of this file). ==="
    );
}
