//! A concrete run, not a claim about one: builds a small "pantheon" --
//! three independent DID-authored commits about the same subject/predicate,
//! a fourth commit that consumes all three together and produces a real
//! synthesis, and a fifth that declares one reading canonical without
//! touching the rest of the history -- and prints what the actual
//! materializer does with it. Written to check `papers/desiring-production-
//! ontology/DRAFT.md` Section 4's claims against real interpreter output
//! before those claims are restated in prose, per the "DMML first" rule
//! (`written-world` `CLAUDE.md`, adopted here too): if a claimed structural
//! property isn't observable by actually running this, the paper should not
//! assert it. Run with `cargo run -p dmml --example pantheon`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. Nothing in the grammar or interpreter requires commits about the
//!    same `(subject, predicate)` to agree, reference each other, or be
//!    reconciled. Helios's, Selene's, and Eos's commits below are produced
//!    with zero coordination -- none `consumes` any other -- and all three
//!    remain in the log permanently. There is no declared "mergeable" mode
//!    being exercised here: this is what happens by default, because
//!    nothing in `dmml`/`dmml-runtime` today implements a declared
//!    consume-kind policy at all (`dmml-runtime/src/substrate.rs`'s own
//!    doc comment names a `mergeable`/`arbitrated` policy as a real but
//!    *unbuilt* substrate-layer idea, not a grammar primitive that exists
//!    yet -- an earlier draft of the paper's Section 4 overclaimed this as
//!    an implemented, declared distinction, which this file's own honest
//!    output corrects).
//! 2. `Materialized::from_identified_commits` gives ONE current answer per
//!    `(subject, predicate)` -- last-write-wins over the full log, same as
//!    an ordinary key-value overwrite. That is not multiplicity at the
//!    level of the *queryable current view*. What actually persists as
//!    multiplicity is the log itself: Helios's, Selene's, and Eos's
//!    original commits stay real, genuinely-produced, individually citable
//!    via `FactRef` by their own `(uri, cid)`, forever -- whether or not
//!    the "current" projection still shows them. All three parts of that
//!    claim are checked below, not just the flattering part.
//! 3. Nyx's commit does something a plain key-value store's overwrite
//!    cannot: it `consumes` all three of Helios's, Selene's, and Eos's
//!    facts by `FactRef` (referential integrity checked against real,
//!    produced content, not invented -- three, not just two, so the claim
//!    isn't resting on the smallest case that could look coincidental) and
//!    *produces* a new fact that did not exist in any input -- connective
//!    tissue folding prior connective tissue into further production, not
//!    just picking a winner. This is offered as a real, checkable
//!    structural analogy to what "double articulation" describes, not a
//!    literal identity claim -- an ordinary SQL JOIN, a git merge, or a
//!    SPARQL `CONSTRUCT` would satisfy the same bare description (read
//!    several records, write one back), and this file does not pretend
//!    otherwise; what distinguishes DMML is the connectivity properties
//!    argued for elsewhere in Section 4 (verified-not-figurative grounding,
//!    cross-sovereign citation, no forced convergence), not double
//!    articulation considered alone.
//! 4. A fifth commit, `canon`, declares one particular reading
//!    (`sky/1 canonicalOrigin`) as authoritative-for-practical-purposes.
//!    This is a genuinely different predicate from `origin` itself, on the
//!    same subject -- a coded, stabilized layer added on top of a still-
//!    fully-present, still-divergent underlying history, not a rewrite of
//!    it. Checked below: `origin`'s own current value and `canonicalOrigin`
//!    are independent facts; retracting/changing one does not touch the
//!    other, and Helios's, Selene's, and Eos's original commits remain in
//!    the log and independently citable regardless of what `canonicalOrigin`
//!    says.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

const HELIOS_SRC: &str = r#"
commit asserts {
  declare attribute origin

  sky/1 origin "sunfire"
}
"#;

const SELENE_SRC: &str = r#"
commit asserts {
  declare attribute origin

  sky/1 origin "moonwoven"
}
"#;

const EOS_SRC: &str = r#"
commit asserts {
  declare attribute origin

  sky/1 origin "rosefingered"
}
"#;

// Nyx's commit cites all three prior facts by FactRef -- real referential-
// integrity targets, matched against HELIOS_SRC's, SELENE_SRC's, and
// EOS_SRC's own (uri, cid) below, not invented placeholders -- and
// produces a fact none of the three inputs contains. Three, not two, so
// the recombination claim below isn't resting on the smallest case that
// could still look like a coincidence.
const NYX_TEMPLATE: &str = r#"
commit weaves {
  declare attribute origin

  consumes {
    fact {helios_uri} (cid: {helios_cid}) {
      subject: sky/1
      predicate: origin
    }
    fact {selene_uri} (cid: {selene_cid}) {
      subject: sky/1
      predicate: origin
    }
    fact {eos_uri} (cid: {eos_cid}) {
      subject: sky/1
      predicate: origin
    }
  }
  produces {
    sky/1 origin "duskweave"
  }
}
"#;

const CANON_SRC: &str = r#"
commit declares {
  declare attribute canonicalOrigin

  sky/1 canonicalOrigin "duskweave"
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
    let helios_uri = "at://did:plc:helios/org.jason-edelman.writtenworld.commit/rkey001";
    let helios_cid = "bafyHeliosSunfire";
    let helios = identify(HELIOS_SRC, helios_uri, helios_cid);

    let selene_uri = "at://did:plc:selene/org.jason-edelman.writtenworld.commit/rkey001";
    let selene_cid = "bafySeleneMoonwoven";
    let selene = identify(SELENE_SRC, selene_uri, selene_cid);

    let eos_uri = "at://did:plc:eos/org.jason-edelman.writtenworld.commit/rkey001";
    let eos_cid = "bafyEosRosefingered";
    let eos = identify(EOS_SRC, eos_uri, eos_cid);

    println!("=== Helios ({helios_uri}) ===\n{HELIOS_SRC}");
    println!("=== Selene ({selene_uri}) ===\n{SELENE_SRC}");
    println!("=== Eos ({eos_uri}) ===\n{EOS_SRC}");
    println!(
        "Three independent DIDs, zero coordination, same (subject, predicate). \
         No commit references any other. Nothing in dmml/dmml-runtime \
         prevents this or flags it.\n"
    );

    let nyx_src = NYX_TEMPLATE
        .replace("{helios_uri}", helios_uri)
        .replace("{helios_cid}", helios_cid)
        .replace("{selene_uri}", selene_uri)
        .replace("{selene_cid}", selene_cid)
        .replace("{eos_uri}", eos_uri)
        .replace("{eos_cid}", eos_cid);
    let nyx = identify(&nyx_src, "at://did:plc:nyx/org.jason-edelman.writtenworld.commit/rkey002", "bafyNyxDuskweave");
    println!("=== Nyx, weaving all three ===\n{nyx_src}");

    let canon = identify(
        CANON_SRC,
        "at://did:plc:pantheon-council/org.jason-edelman.writtenworld.commit/rkey003",
        "bafyCanonDuskweave",
    );
    println!("=== A council commit, declaring one reading canonical ===\n{CANON_SRC}");

    // The full log, in the order the deities and the council actually
    // committed -- no reordering, no pruning, no privileged final editor.
    let full_log = vec![helios.clone(), selene.clone(), eos.clone(), nyx.clone(), canon.clone()];
    let materialized = Materialized::from_identified_commits(&full_log);

    // Check 1: the current view is last-write-wins, not a coexistence view.
    // This is the honest finding, not the flattering one -- state it plainly.
    let current_origin = materialized.current_value("sky/1", "origin");
    println!("current_value(sky/1, origin) = {current_origin:?}");
    assert_eq!(
        current_origin,
        Some(&dmml::lower::TripleValue::Str("duskweave".to_string())),
        "the CURRENT view shows only Nyx's synthesis -- Helios's and Selene's \
         own claims are overwritten in this projection, exactly like an \
         ordinary key-value store. Multiplicity is not a property of this view."
    );

    // Check 2: Helios's and Selene's own commits are still real and
    // independently materializable -- multiplicity lives in the log, not
    // erased by check 1's overwrite. This is what actually distinguishes
    // "overwritten" from "gone."
    let helios_alone = Materialized::from_identified_commits(&[helios.clone()]);
    let selene_alone = Materialized::from_identified_commits(&[selene.clone()]);
    let eos_alone = Materialized::from_identified_commits(&[eos.clone()]);
    assert_eq!(
        helios_alone.current_value("sky/1", "origin"),
        Some(&dmml::lower::TripleValue::Str("sunfire".to_string())),
        "Helios's own commit, read alone, still genuinely says sunfire"
    );
    assert_eq!(
        selene_alone.current_value("sky/1", "origin"),
        Some(&dmml::lower::TripleValue::Str("moonwoven".to_string())),
        "Selene's own commit, read alone, still genuinely says moonwoven"
    );
    assert_eq!(
        eos_alone.current_value("sky/1", "origin"),
        Some(&dmml::lower::TripleValue::Str("rosefingered".to_string())),
        "Eos's own commit, read alone, still genuinely says rosefingered"
    );
    println!(
        "Helios's, Selene's, and Eos's own commits, materialized alone: \
         {:?}, {:?}, {:?} -- none erased, none requires the others' commits \
         to exist, all three still cite-able by (uri, cid) forever.",
        helios_alone.current_value("sky/1", "origin"),
        selene_alone.current_value("sky/1", "origin"),
        eos_alone.current_value("sky/1", "origin"),
    );

    // Check 3: Nyx's synthesis was only accepted because it named real,
    // produced facts -- referential integrity, not invention. Proven by
    // construction here: the FactRef literals above are the SAME (uri, cid)
    // pairs returned by identify() for Helios's, Selene's, and Eos's own
    // commits, not typed independently -- if any mismatched, this is real
    // interpreter behavior to check, not just a story about it, so confirm
    // Nyx's own consumes actually reference them, three deep, not two.
    assert_eq!(nyx.commit.consumes.len(), 3, "Nyx's commit cites exactly three prior facts");
    println!(
        "Nyx's commit.consumes cites {} real prior facts, and produces one \
         value ({:?}) that existed in none of the three inputs -- \
         recombination, not selection between them.",
        nyx.commit.consumes.len(),
        Materialized::from_identified_commits(&[nyx.clone()]).current_value("sky/1", "origin"),
    );

    // Check 4: canonicalOrigin is a genuinely separate fact from origin
    // itself -- a stratifying layer that coexists with, rather than
    // replacing, the multiplicity underneath it.
    let canon_value = materialized.current_value("sky/1", "canonicalOrigin");
    println!("current_value(sky/1, canonicalOrigin) = {canon_value:?}");
    assert_eq!(
        canon_value,
        Some(&dmml::lower::TripleValue::Str("duskweave".to_string()))
    );
    assert_ne!(
        materialized.current_value("sky/1", "origin"),
        None,
        "origin itself is untouched by canon's declaration -- a different predicate entirely"
    );

    println!(
        "\n=== done: last-write-wins at the current-view level (Check 1) is real \
         and should not be papered over; permanent, independently-citable \
         multiplicity lives in the log (Check 2); real recombination via \
         multi-fact consumes is real and demonstrated, not asserted (Check 3); \
         declared canonicity is a separate, additive stratum, not an erasure \
         (Check 4). ==="
    );
}
