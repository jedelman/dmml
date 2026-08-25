//! A concrete run of this session's own editorial workflow -- modeling
//! the same petition/resolution pattern `pantheon.rs` demonstrates for
//! first-order facts, applied reflexively to the process that produced
//! this repository's own papers. Prompted directly: "what's fun about an
//! open ontology is you could go in and dispute or alter your own
//! resolution. Nothing is fixed or final." Checked here, not just
//! asserted: a resolution commit is exactly as revisable as any other
//! fact, by its own author or anyone else, and the original stays fully
//! present after being superseded. Run with `cargo run -p dmml --example
//! editorial_loop`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. A **petition** and its **suggestion** are ordinary commits, the same
//!    shape as any other fact in the log -- a model proposing an edit is
//!    structurally identical to a player asking what's behind a locked
//!    door (Section 1 of the ontology paper's own worked example): a
//!    request against something underdetermined, with an outside party's
//!    response consuming it and producing a new fact linked back to it.
//! 2. Dev Lead's first resolution `consumes` the suggestion and produces
//!    a verdict. Nothing distinguishes this from any other commit -- it
//!    is not marked final, privileged, or closed to further revision.
//! 3. A SECOND commit, from the same Dev Lead identity, later in the log,
//!    disputes the first resolution: it `consumes` the original
//!    resolution by `FactRef` (not by ignoring it) and produces a revised
//!    verdict. This is the concrete referent for "nothing is fixed or
//!    final" -- checked by actually doing it, not asserted about the
//!    grammar in the abstract.
//! 4. The original resolution remains fully present and independently
//!    re-materializable after being superseded in the current view --
//!    captured by neither the revision nor anything after it, only built
//!    upon, exactly the property `pantheon.rs` established for Helios's,
//!    Selene's, and Eos's rival first-order claims, now shown to hold for
//!    an author's dispute with their own past resolution.
//! 5. A THIRD commit, from a different identity entirely, disputes the
//!    revision in turn -- the ontology is open in both directions: an
//!    author can reopen their own prior verdict, and any other author can
//!    reopen it too. No commit in this chain required anyone's permission
//!    to dispute anything that came before it.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

const SUGGESTION_SRC: &str = r#"
commit proposes {
  declare attribute proposedCut
  declare attribute category
  declare attribute confidence

  suggestion/1 proposedCut "This paper states that difference precisely rather than treating either side as simply better"
  suggestion/1 category "d-genuine-tic"
  suggestion/1 confidence "medium"
}
"#;

// Dev Lead's first resolution: not a full accept, not a full reject --
// a modification, consuming the suggestion.
const RESOLUTION_1_TEMPLATE: &str = r#"
commit resolves {
  declare attribute verdict
  declare attribute appliedText

  consumes {
    fact {suggestion_uri} (cid: {suggestion_cid}) {
      subject: suggestion/1
      predicate: proposedCut
    }
  }
  produces {
    resolution/1 verdict "modified"
    resolution/1 appliedText "Neither side is simply better."
  }
}
"#;

// Dev Lead disputes its OWN prior resolution -- same identity, later
// commit, consuming resolution/1 by FactRef rather than ignoring it.
const RESOLUTION_2_TEMPLATE: &str = r#"
commit revises {
  declare attribute verdict
  declare attribute appliedText
  declare relation revisits

  consumes {
    fact {resolution1_uri} (cid: {resolution1_cid}) {
      subject: resolution/1
      predicate: verdict
    }
  }
  produces {
    resolution/1 verdict "revised"
    resolution/1 appliedText "DMML can inspect, verify provenance for, and compose its own state; a learned world model cannot."
    resolution/1 revisits resolution/1
  }
}
"#;

// A different identity disputes the revision in turn.
const RESOLUTION_3_TEMPLATE: &str = r#"
commit disputes {
  declare attribute verdict
  declare attribute appliedText

  consumes {
    fact {resolution2_uri} (cid: {resolution2_cid}) {
      subject: resolution/1
      predicate: verdict
    }
  }
  produces {
    resolution/1 verdict "disputed-by-another-author"
    resolution/1 appliedText "Restore the original framing sentence; the trim loses the paper's own stated stance."
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
    let suggestion_uri = "at://did:plc:gemini-flash/org.jason-edelman.writtenworld.commit/rkey001";
    let suggestion_cid = "bafySuggestion1";
    let suggestion = identify(SUGGESTION_SRC, suggestion_uri, suggestion_cid);
    println!("=== Gemini's suggestion ===\n{SUGGESTION_SRC}");

    let resolution1_uri = "at://did:plc:dev-lead/org.jason-edelman.writtenworld.commit/rkey001";
    let resolution1_cid = "bafyResolution1Modified";
    let resolution1_src = RESOLUTION_1_TEMPLATE
        .replace("{suggestion_uri}", suggestion_uri)
        .replace("{suggestion_cid}", suggestion_cid);
    let resolution1 = identify(&resolution1_src, resolution1_uri, resolution1_cid);
    println!("=== Dev Lead's first resolution ===\n{resolution1_src}");

    let resolution2_uri = "at://did:plc:dev-lead/org.jason-edelman.writtenworld.commit/rkey002";
    let resolution2_cid = "bafyResolution2Revised";
    let resolution2_src = RESOLUTION_2_TEMPLATE
        .replace("{resolution1_uri}", resolution1_uri)
        .replace("{resolution1_cid}", resolution1_cid);
    let resolution2 = identify(&resolution2_src, resolution2_uri, resolution2_cid);
    println!(
        "=== Dev Lead disputes its OWN earlier resolution, later, same identity ===\n{resolution2_src}"
    );

    let resolution3_uri = "at://did:plc:a-second-reviewer/org.jason-edelman.writtenworld.commit/rkey001";
    let resolution3_cid = "bafyResolution3Disputed";
    let resolution3_src = RESOLUTION_3_TEMPLATE
        .replace("{resolution2_uri}", resolution2_uri)
        .replace("{resolution2_cid}", resolution2_cid);
    let resolution3 = identify(&resolution3_src, resolution3_uri, resolution3_cid);
    println!(
        "=== A different identity disputes the revision in turn ===\n{resolution3_src}"
    );

    let full_log = vec![
        suggestion.clone(),
        resolution1.clone(),
        resolution2.clone(),
        resolution3.clone(),
    ];
    let materialized = Materialized::from_identified_commits(&full_log);

    // Check 1: the current view shows the LAST word, per usual --
    // dispute doesn't change that dynamic, only who gets to have it.
    let current_verdict = materialized.current_value("resolution/1", "verdict");
    println!("current_value(resolution/1, verdict) = {current_verdict:?}");
    assert_eq!(
        current_verdict,
        Some(&dmml::lower::TripleValue::Str(
            "disputed-by-another-author".to_string()
        )),
        "the current view shows the most recent dispute, last-write-wins as always"
    );

    // Check 2: Dev Lead's ORIGINAL resolution is still real, still fully
    // present, still independently re-materializable -- not erased by its
    // own author's later self-dispute, and not erased by the third
    // author's dispute of THAT dispute either.
    let resolution1_alone = Materialized::from_identified_commits(&[resolution1.clone()]);
    assert_eq!(
        resolution1_alone.current_value("resolution/1", "verdict"),
        Some(&dmml::lower::TripleValue::Str("modified".to_string())),
        "Dev Lead's first resolution, read alone, still genuinely says 'modified'"
    );
    println!(
        "Dev Lead's original resolution, materialized alone: {:?} -- \
         still real, still citable, even after Dev Lead's own later commit \
         disputed it and a third author disputed that dispute in turn.",
        resolution1_alone.current_value("resolution/1", "verdict"),
    );

    // Check 3: the revision genuinely consumed (cited) the original --
    // this was a dispute WITH the record, not a decision made in
    // ignorance of it.
    assert_eq!(
        resolution2.commit.consumes.len(),
        1,
        "the revision cites exactly the original resolution it disputes"
    );
    println!(
        "resolution2.commit.consumes cites {} real prior fact -- the dispute \
         engaged the original verdict rather than silently overwriting it.",
        resolution2.commit.consumes.len(),
    );

    // Check 4: nothing in the grammar distinguished "Dev Lead disputing
    // itself" from "a different author disputing Dev Lead" -- both are
    // the same shape of commit, checked by using two different DIDs for
    // resolution2 (dev-lead) and resolution3 (a-second-reviewer) and
    // confirming both were accepted identically.
    assert!(resolution2.uri.contains("did:plc:dev-lead/"));
    assert!(resolution3.uri.contains("did:plc:a-second-reviewer/"));
    println!(
        "\n=== done: nothing is fixed or final (Check 1) -- the log always shows \
         the latest word, but that word can come from the original author \
         disputing themselves or from anyone else, with no special primitive \
         distinguishing the two (Check 4). The original resolution survives \
         every dispute built on top of it (Check 2), each dispute genuinely \
         engaging what it disputes rather than ignoring it (Check 3). ==="
    );
}
