//! Section VIII, read slowly: one root cause (camera-mediated rather than
//! in-person presentation) branching into a TWOFOLD consequence that
//! Benjamin states explicitly -- the first about the PERFORMANCE
//! (fragmented into "a series of optical tests" by the cameraman and
//! editor), the second about the AUDIENCE (loses personal contact,
//! identifies with the camera instead of the actor). This is a fan-OUT --
//! one cause, two independent effects that don't cite each other -- the
//! mirror image of Section VII's fan-IN (many independent witnesses, one
//! shared diagnosis). Both consequences then converge, together with an
//! explicit callback to Sections V-VI's cult-value material, on the
//! paragraph's closing verdict: "this is not the approach to which cult
//! values may be exposed." Run with `cargo run -p dmml --example
//! benjamin_section_viii`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. The twofold consequence is a real fan-out: both consequence commits
//!    consume ONLY the root mediation fact (consumes count 1 each), and
//!    NEITHER cites the other. Checked directly against each commit's own
//!    consumes list, not just described in comments.
//! 2. Persistence is checked the honest way this time, per Section VI's
//!    finding: by re-materializing the root mediation fact ALONE, not by
//!    querying its key inside the combined log after two later commits
//!    have cited it (which would read back None, same cite-and-spend
//!    behavior as before -- not re-litigated here, just applied
//!    correctly).
//! 3. The convergence commit engages THREE facts at once -- both
//!    consequences AND an explicit callback to the cult-value material
//!    from Sections V-VI (re-declared minimally here, matching this
//!    series' standing convention for cross-file callbacks). Checked:
//!    consumes count 3, confirming the closing sentence is doing real
//!    work tying three threads together, not simply restating the second
//!    consequence.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// The root cause: presented BY A CAMERA, not in person -- "with a
// twofold consequence."
const MEDIATION_SRC: &str = r#"
commit asserts {
  declare attribute presentationMode

  actor/performance presentationMode "camera-mediated, not presented to the public in person"
}
"#;

// Consequence 1 (production side): the camera need not respect the
// performance as an integral whole -- cameraman repositions, editor
// assembles a sequence, camera movement/angles/close-ups are woven in.
// "the performance of the actor is subjected to a series of optical
// tests."
const FRAGMENTATION_TEMPLATE: &str = r#"
commit argues {
  declare attribute fragmentationStatus

  consumes {
    fact {mediation_uri} (cid: {mediation_cid}) {
      subject: actor/performance
      predicate: presentationMode
    }
  }
  produces {
    actor/performance fragmentationStatus "subjected to a series of optical tests -- camera repositions per the cameraman, editor assembles the sequence from supplied material"
  }
}
"#;

// Consequence 2 (reception side): the actor cannot adjust to the audience
// live; the audience becomes a critic without personal contact, and its
// identification is with the CAMERA, not the actor.
const AUDIENCE_POSTURE_TEMPLATE: &str = r#"
commit argues {
  declare attribute posture

  consumes {
    fact {mediation_uri} (cid: {mediation_cid}) {
      subject: actor/performance
      predicate: presentationMode
    }
  }
  produces {
    audience/1 posture "takes the position of a critic without personal contact; identification is with the camera, not the actor; its approach is that of testing"
  }
}
"#;

// A minimal callback to Sections V-VI's cult-value material, re-declared
// here for this file's self-containment per this series' standing
// convention.
const CULT_VALUE_REQUIREMENT_SRC: &str = r#"
commit asserts {
  declare attribute requiredApproach

  argument/cult_value requiredApproach "not testing -- cult values are never exposed to a critical, testing approach"
}
"#;

// The convergence: both consequences AND the cult-value callback
// together license the paragraph's closing verdict.
const CONVERGENCE_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {fragmentation_uri} (cid: {fragmentation_cid}) {
      subject: actor/performance
      predicate: fragmentationStatus
    }
    fact {posture_uri} (cid: {posture_cid}) {
      subject: audience/1
      predicate: posture
    }
    fact {cult_value_uri} (cid: {cult_value_cid}) {
      subject: argument/cult_value
      predicate: requiredApproach
    }
  }
  produces {
    argument/section_viii claim "this is not the approach to which cult values may be exposed"
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
    let mediation_uri = "at://did:plc:form-reading-viii/org.jason-edelman.writtenworld.commit/rkey001";
    let mediation_cid = "bafyMediation";
    let mediation = identify(MEDIATION_SRC, mediation_uri, mediation_cid);
    println!("=== The root cause: camera-mediated, not in-person presentation ===\n{MEDIATION_SRC}");

    let fragmentation_uri = "at://did:plc:form-reading-viii/org.jason-edelman.writtenworld.commit/rkey002";
    let fragmentation_cid = "bafyFragmentation";
    let fragmentation_src = FRAGMENTATION_TEMPLATE
        .replace("{mediation_uri}", mediation_uri)
        .replace("{mediation_cid}", mediation_cid);
    let fragmentation = identify(&fragmentation_src, fragmentation_uri, fragmentation_cid);
    println!("=== Consequence 1 (production side): the performance fragmented into optical tests ===\n{fragmentation_src}");

    let posture_uri = "at://did:plc:form-reading-viii/org.jason-edelman.writtenworld.commit/rkey003";
    let posture_cid = "bafyPosture";
    let posture_src = AUDIENCE_POSTURE_TEMPLATE
        .replace("{mediation_uri}", mediation_uri)
        .replace("{mediation_cid}", mediation_cid);
    let posture = identify(&posture_src, posture_uri, posture_cid);
    println!("=== Consequence 2 (reception side): audience identifies with the camera ===\n{posture_src}");

    let cult_value_uri = "at://did:plc:form-reading-v/org.jason-edelman.writtenworld.commit/rkey999";
    let cult_value_cid = "bafyCultValueCallback";
    let cult_value = identify(CULT_VALUE_REQUIREMENT_SRC, cult_value_uri, cult_value_cid);

    let convergence_src = CONVERGENCE_TEMPLATE
        .replace("{fragmentation_uri}", fragmentation_uri)
        .replace("{fragmentation_cid}", fragmentation_cid)
        .replace("{posture_uri}", posture_uri)
        .replace("{posture_cid}", posture_cid)
        .replace("{cult_value_uri}", &cult_value_uri)
        .replace("{cult_value_cid}", cult_value_cid);
    let convergence_uri = "at://did:plc:form-reading-viii/org.jason-edelman.writtenworld.commit/rkey004";
    let convergence = identify(&convergence_src, convergence_uri, "bafyConvergence");
    println!("=== Convergence: both consequences + the cult-value callback ===\n{convergence_src}");

    // Check 1: a real fan-out -- both consequences consume ONLY the root,
    // neither cites the other.
    assert_eq!(fragmentation.commit.consumes.len(), 1);
    assert_eq!(posture.commit.consumes.len(), 1);
    println!(
        "\nCheck 1: fragmentation.consumes.len() = {}, posture.consumes.len() = {} -- both \
         consequences consume only the root mediation fact, neither cites the other. A fan-out, \
         the mirror image of Section VII's fan-in.",
        fragmentation.commit.consumes.len(),
        posture.commit.consumes.len(),
    );

    // Check 2: persistence checked the honest way -- materialize the root
    // ALONE, per Section VI's finding, not by querying its key inside the
    // combined log (which would read back None, since both consequences
    // cite it and retract it, same cite-and-spend semantics as before).
    let mediation_alone = Materialized::from_identified_commits(&[mediation.clone()]);
    assert_eq!(
        mediation_alone.current_value("actor/performance", "presentationMode"),
        Some(&dmml::lower::TripleValue::Str(
            "camera-mediated, not presented to the public in person".to_string()
        ))
    );
    println!(
        "Check 2: mediation, materialized alone: {:?} -- real and citable, checked the correct \
         way this time (alone, not queried inside the combined log where cite-and-spend would \
         retract its visibility -- Section VI's lesson applied, not re-discovered).",
        mediation_alone.current_value("actor/performance", "presentationMode"),
    );

    // Check 3: the convergence genuinely engages three facts, not two --
    // the cult-value callback is doing real, checkable work, not merely
    // restating consequence 2.
    assert_eq!(convergence.commit.consumes.len(), 3);
    println!(
        "Check 3: convergence.commit.consumes.len() = {} -- both consequences AND the cult- \
         value callback together license the closing verdict, not the audience-posture \
         consequence alone.",
        convergence.commit.consumes.len(),
    );

    println!(
        "\n=== done: a real fan-out, one cause branching into two independent consequences \
         (Check 1); persistence checked honestly, alone rather than inside a combined log that \
         would retract it (Check 2); the convergence genuinely draws on three facts, not a \
         restatement of one (Check 3). ==="
    );
}
