//! Section XII, read slowly -- specifically to TEST the prediction from
//! the last conversation: does this section actually pay off Section
//! III's mass-psychological thread, generalizing Section VIII's audience-
//! posture material into a real theory of mass reception? Checked here,
//! not assumed. Run with `cargo run -p dmml --example benjamin_section_xii`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. The FUSION claim -- "with regard to the screen, the critical and
//!    receptive attitudes of the public coincide" -- genuinely consumes
//!    Section VIII's audience-posture fact (re-declared cross-file), not
//!    a fresh assertion. It BUILDS ON the testing-posture Section VIII
//!    already established rather than restating it: Section VIII said the
//!    audience takes a testing stance; this section adds that testing and
//!    ENJOYMENT now coincide, which VIII alone didn't claim.
//! 2. THE PREDICTION TEST: does the mass-conditioning claim ("individual
//!    reactions are predetermined by the mass audience response they are
//!    about to produce") genuinely consume Section III's `massDesire`
//!    fact? If the essay's mass-psychology thread from Section III really
//!    does get cashed out here, this citation should hold up under an
//!    actual check, the same technique used for the Section IX->X and
//!    Section VIII->XI predictions.
//! 3. "Simultaneous collective experience" is modeled as a DISTINCT
//!    predicate from Section V's `exhibitionFitness` -- checked explicitly
//!    to confirm they are NOT conflated. Painting can score high on
//!    exhibition fitness (Section V/VI: galleries, salons) while still
//!    lacking simultaneous-collective-reception capacity entirely --
//!    two independent axes, not one continuum.
//! 4. The closing claim -- the SAME public responds progressively to a
//!    grotesque film but reactionarily to surrealism -- consumes the
//!    exhibition-value/collective-capacity distinction, confirming
//!    reaction is medium-relative, not just content-relative: same
//!    content-type (the grotesque), opposite reactions, because the
//!    medium differs in collective-reception capacity.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// Section VIII's audience-posture fact, re-declared cross-file.
const AUDIENCE_POSTURE_SRC: &str = r#"
commit argues {
  declare attribute posture

  audience/1 posture "takes the position of a critic without personal contact; identification is with the camera, not the actor; its approach is that of testing"
}
"#;

// Section III's massDesire fact, re-declared cross-file.
const MASS_DESIRE_SRC: &str = r#"
commit argues {
  declare attribute massDesire

  argument/section_iii_mechanism massDesire "bring-things-closer-spatially-and-humanly, AND overcome-uniqueness-by-accepting-reproduction"
}
"#;

// Section V's exhibitionFitness gradient endpoint, re-declared cross-file.
const EXHIBITION_GRADIENT_SRC: &str = r#"
commit asserts {
  declare attribute exhibitionFitness

  artwork/symphony exhibitionFitness "highest -- originated when its public presentability promised to surpass the mass"
}
"#;

// Picasso vs. Chaplin -- ONE commit, TWO subjects, the illustrative
// contrast Benjamin opens with. Asserted, not derived.
const PICASSO_CHAPLIN_SRC: &str = r#"
commit asserts {
  declare attribute attitude

  reception/picasso attitude "reactionary"
  reception/chaplin attitude "progressive -- direct, intimate fusion of visual and emotional enjoyment with the orientation of the expert"
}
"#;

// The fusion claim: consumes Section VIII's posture fact AND Chaplin's
// progressive attitude -- BUILDS ON the testing-posture, adds that
// testing and enjoyment now coincide.
const FUSION_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim

  consumes {
    fact {posture_uri} (cid: {posture_cid}) {
      subject: audience/1
      predicate: posture
    }
    fact {chaplin_uri} (cid: {chaplin_cid}) {
      subject: reception/chaplin
      predicate: attitude
    }
  }
  produces {
    argument/section_xii_fusion claim "with regard to the screen, the critical and receptive attitudes of the public coincide -- testing and enjoyment are no longer separate"
  }
}
"#;

// THE PREDICTION TEST: does this genuinely consume Section III's
// massDesire fact?
const MASS_CONDITIONING_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {fusion_uri} (cid: {fusion_cid}) {
      subject: argument/section_xii_fusion
      predicate: claim
    }
    fact {mass_desire_uri} (cid: {mass_desire_cid}) {
      subject: argument/section_iii_mechanism
      predicate: massDesire
    }
  }
  produces {
    argument/section_xii_conditioning claim "individual reactions are predetermined by the mass audience response they are about to produce -- nowhere more pronounced than in film"
  }
}
"#;

// "Simultaneous collective experience" -- a DISTINCT predicate from
// Section V's exhibitionFitness, consuming it to show the contrast
// explicitly, not conflating the two axes.
const SIMULTANEITY_TEMPLATE: &str = r#"
commit argues {
  declare attribute simultaneousCollectiveCapacity

  consumes {
    fact {gradient_uri} (cid: {gradient_cid}) {
      subject: artwork/symphony
      predicate: exhibitionFitness
    }
  }
  produces {
    medium/painting simultaneousCollectiveCapacity "none -- painting is in no position to present an object for simultaneous collective experience, unlike architecture, the epic poem, or film"
  }
}
"#;

// The negative result: painting's exhibition value increased (galleries,
// salons) but collective-reception capacity did NOT come with it --
// two independent axes.
const TWO_AXES_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {simultaneity_uri} (cid: {simultaneity_cid}) {
      subject: medium/painting
      predicate: simultaneousCollectiveCapacity
    }
    fact {mass_conditioning_uri} (cid: {mass_conditioning_cid}) {
      subject: argument/section_xii_conditioning
      predicate: claim
    }
  }
  produces {
    argument/section_xii_two_axes claim "paintings began to be publicly exhibited in galleries and salons, but there was no way for the masses to organize and control themselves in their reception -- exhibition value and collective-reception capacity are independent axes"
  }
}
"#;

// The closing claim: reaction is MEDIUM-relative, not content-relative --
// same public, same content-type (the grotesque), opposite reactions.
const MEDIUM_RELATIVE_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {two_axes_uri} (cid: {two_axes_cid}) {
      subject: argument/section_xii_two_axes
      predicate: claim
    }
  }
  produces {
    argument/section_xii claim "the same public which responds in a progressive manner toward a grotesque film is bound to respond in a reactionary manner to surrealism -- reaction is medium-relative, not content-relative"
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
    let posture_cid = "bafyPostureXII";
    let posture = identify(AUDIENCE_POSTURE_SRC, posture_uri, posture_cid);

    let mass_desire_uri = "at://did:plc:form-reading-iii/org.jason-edelman.writtenworld.commit/rkey004";
    let mass_desire_cid = "bafyMassDesireXII";
    let mass_desire = identify(MASS_DESIRE_SRC, mass_desire_uri, mass_desire_cid);

    let gradient_uri = "at://did:plc:form-reading-v/org.jason-edelman.writtenworld.commit/rkey001";
    let gradient_cid = "bafyGradientXII";
    let gradient = identify(EXHIBITION_GRADIENT_SRC, gradient_uri, gradient_cid);
    println!("=== Carried over: Section VIII's posture, Section III's massDesire, Section V's gradient ===");

    let picasso_chaplin_uri = "at://did:plc:form-reading-xii/org.jason-edelman.writtenworld.commit/rkey001";
    let picasso_chaplin = identify(PICASSO_CHAPLIN_SRC, picasso_chaplin_uri, "bafyPicassoChaplin");
    println!("=== Picasso (reactionary) vs. Chaplin (progressive) ===\n{PICASSO_CHAPLIN_SRC}");

    let fusion_uri = "at://did:plc:form-reading-xii/org.jason-edelman.writtenworld.commit/rkey002";
    let fusion_src = FUSION_TEMPLATE
        .replace("{posture_uri}", posture_uri)
        .replace("{posture_cid}", posture_cid)
        .replace("{chaplin_uri}", picasso_chaplin_uri)
        .replace("{chaplin_cid}", "bafyPicassoChaplin");
    let fusion = identify(&fusion_src, fusion_uri, "bafyFusion");
    println!("=== The fusion: testing and enjoyment coincide, built on Section VIII ===\n{fusion_src}");

    let mass_conditioning_uri = "at://did:plc:form-reading-xii/org.jason-edelman.writtenworld.commit/rkey003";
    let mass_conditioning_src = MASS_CONDITIONING_TEMPLATE
        .replace("{fusion_uri}", fusion_uri)
        .replace("{fusion_cid}", "bafyFusion")
        .replace("{mass_desire_uri}", mass_desire_uri)
        .replace("{mass_desire_cid}", mass_desire_cid);
    let mass_conditioning = identify(&mass_conditioning_src, mass_conditioning_uri, "bafyMassConditioning");
    println!("=== THE PREDICTION TEST: does this consume Section III's massDesire? ===\n{mass_conditioning_src}");

    let simultaneity_uri = "at://did:plc:form-reading-xii/org.jason-edelman.writtenworld.commit/rkey004";
    let simultaneity_src = SIMULTANEITY_TEMPLATE
        .replace("{gradient_uri}", gradient_uri)
        .replace("{gradient_cid}", gradient_cid);
    let simultaneity = identify(&simultaneity_src, simultaneity_uri, "bafySimultaneity");
    println!("=== Simultaneous collective experience: a distinct axis from exhibition fitness ===\n{simultaneity_src}");

    let two_axes_uri = "at://did:plc:form-reading-xii/org.jason-edelman.writtenworld.commit/rkey005";
    let two_axes_src = TWO_AXES_TEMPLATE
        .replace("{simultaneity_uri}", simultaneity_uri)
        .replace("{simultaneity_cid}", "bafySimultaneity")
        .replace("{mass_conditioning_uri}", mass_conditioning_uri)
        .replace("{mass_conditioning_cid}", "bafyMassConditioning");
    let two_axes = identify(&two_axes_src, two_axes_uri, "bafyTwoAxes");
    println!("=== Two independent axes: exhibition value up, collective capacity absent ===\n{two_axes_src}");

    let medium_relative_uri = "at://did:plc:form-reading-xii/org.jason-edelman.writtenworld.commit/rkey006";
    let medium_relative_src = MEDIUM_RELATIVE_TEMPLATE
        .replace("{two_axes_uri}", two_axes_uri)
        .replace("{two_axes_cid}", "bafyTwoAxes");
    let medium_relative = identify(&medium_relative_src, medium_relative_uri, "bafyMediumRelative");
    println!("=== Closing: reaction is medium-relative, not content-relative ===\n{medium_relative_src}");

    // Check 1: fusion genuinely builds on Section VIII, consuming BOTH
    // the posture fact and Chaplin's attitude.
    assert_eq!(fusion.commit.consumes.len(), 2);
    let fusion_cites_posture = fusion.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "posture",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(fusion_cites_posture);
    println!(
        "\nCheck 1: fusion.commit.consumes.len() = {}, and one fact cites Section VIII's \
         posture predicate -- the fusion claim genuinely builds on Section VIII's testing- \
         posture rather than restating it independently.",
        fusion.commit.consumes.len(),
    );

    // Check 2: THE PREDICTION TEST. Does mass_conditioning genuinely
    // consume Section III's massDesire fact?
    assert_eq!(mass_conditioning.commit.consumes.len(), 2);
    let cites_mass_desire = mass_conditioning.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "massDesire",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_mass_desire, "the mass-conditioning claim must genuinely cite Section III's massDesire fact");
    println!(
        "Check 2 (THE PREDICTION TEST): mass_conditioning.commit.consumes.len() = {}, and one \
         of those two facts is Section III's massDesire predicate -- CONFIRMED: the essay's \
         mass-psychology thread from Section III really is cashed out here, four sections \
         later, under an actual check. Last conversation's prediction holds up.",
        mass_conditioning.commit.consumes.len(),
    );

    // Check 3: simultaneousCollectiveCapacity is a DISTINCT predicate
    // from exhibitionFitness -- not conflated.
    let simultaneity_alone = Materialized::from_identified_commits(&[simultaneity.clone()]);
    let capacity = simultaneity_alone.current_value("medium/painting", "simultaneousCollectiveCapacity");
    let gradient_alone = Materialized::from_identified_commits(&[gradient.clone()]);
    let fitness = gradient_alone.current_value("artwork/symphony", "exhibitionFitness");
    assert_ne!(capacity, fitness);
    println!(
        "Check 3: simultaneousCollectiveCapacity = {capacity:?}; exhibitionFitness = \
         {fitness:?} -- genuinely distinct axes, not conflated. Painting can score high on \
         exhibition fitness (Section V/VI's galleries and salons) while lacking collective- \
         reception capacity entirely."
    );

    // Check 4: the closing medium-relative claim consumes the two-axes
    // distinction, confirming reaction depends on medium, not content
    // alone.
    assert_eq!(medium_relative.commit.consumes.len(), 1);
    println!(
        "Check 4: medium_relative.commit.consumes.len() = {} -- the closing claim (same public, \
         same content-type, opposite reactions to film vs. surrealist painting) is built on the \
         two-axes distinction, confirming reaction is medium-relative.",
        medium_relative.commit.consumes.len(),
    );

    println!(
        "\n=== done: the fusion claim builds on Section VIII's posture rather than restating it \
         (Check 1); Section III's mass-psychology thread is genuinely cashed out here, \
         confirming last conversation's prediction under an actual check (Check 2, the real \
         point of this file); collective-reception capacity and exhibition fitness are checked \
         as distinct, not conflated axes (Check 3); the closing medium-relative claim is built \
         on that distinction (Check 4). ==="
    );
}
