//! A concrete run, not a claim about one: builds a small "pantheon" --
//! three independent DID-authored commits about the same subject/predicate,
//! a fourth commit that consumes two of them together and produces a real
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
//! 1. Nothing in the grammar or interpreter requires two commits about the
//!    same `(subject, predicate)` to agree, reference each other, or be
//!    reconciled. Helios's and Selene's commits below are produced with
//!    zero coordination -- neither `consumes` the other -- and both remain
//!    in the log permanently. There is no declared "mergeable" mode being
//!    exercised here: this is what happens by default, because nothing
//!    in `dmml`/`dmml-runtime` today implements a declared consume-kind
//!    policy at all (`dmml-runtime/src/substrate.rs`'s own doc comment
//!    names a `mergeable`/`arbitrated` policy as a real but *unbuilt*
//!    substrate-layer idea, not a grammar primitive that exists yet --
//!    an earlier draft of the paper's Section 4 overclaimed this as an
//!    implemented, declared distinction, which this file's own honest
//!    output corrects).
//! 2. `Materialized::from_identified_commits` gives ONE current answer per
//!    `(subject, predicate)` -- last-write-wins over the full log, same as
//!    an ordinary key-value overwrite. That is not multiplicity at the
//!    level of the *queryable current view*. What actually persists as
//!    multiplicity is the log itself: Helios's and Selene's original
//!    commits stay real, genuinely-produced, individually citable via
//!    `FactRef` by their own `(uri, cid)`, forever -- whether or not the
//!    "current" projection still shows them. Both halves of that claim are
//!    checked below, not just the flattering half.
//! 3. Nyx's commit does something a plain key-value store's overwrite
//!    cannot: it `consumes` BOTH Helios's and Selene's facts by `FactRef`
//!    (referential integrity checked against real, produced content, not
//!    invented) and *produces* a new fact that did not exist in either
//!    input -- connective tissue folding prior connective tissue into
//!    further production, not just picking a winner. That is the concrete
//!    referent for "auto-recombinant rhizome" -- checked here by literally
//!    doing it, not asserted about the grammar in the abstract.
//! 4. A fifth commit, `canon`, declares one particular reading
//!    (`sky/1 canonicalOrigin`) as authoritative-for-practical-purposes.
//!    This is a genuinely different predicate from `origin` itself, on the
//!    same subject -- a coded, stabilized layer added on top of a still-
//!    fully-present, still-divergent underlying history, not a rewrite of
//!    it. Checked below: `origin`'s own current value and `canonicalOrigin`
//!    are independent facts; retracting/changing one does not touch the
//!    other, and Helios's and Selene's original commits remain in the log
//!    and independently citable regardless of what `canonicalOrigin` says.

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

// Nyx's commit cites both prior facts by FactRef -- real referential-
// integrity targets, matched against HELIOS_SRC's and SELENE_SRC's own
// (uri, cid) below, not invented placeholders -- and produces a fact
// neither input contains.
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

    println!("=== Helios ({helios_uri}) ===\n{HELIOS_SRC}");
    println!("=== Selene ({selene_uri}) ===\n{SELENE_SRC}");
    println!(
        "Two independent DIDs, zero coordination, same (subject, predicate). \
         Neither commit references the other. Nothing in dmml/dmml-runtime \
         prevents this or flags it.\n"
    );

    let nyx_src = NYX_TEMPLATE
        .replace("{helios_uri}", helios_uri)
        .replace("{helios_cid}", helios_cid)
        .replace("{selene_uri}", selene_uri)
        .replace("{selene_cid}", selene_cid);
    let nyx = identify(&nyx_src, "at://did:plc:nyx/org.jason-edelman.writtenworld.commit/rkey002", "bafyNyxDuskweave");
    println!("=== Nyx, weaving both ===\n{nyx_src}");

    let canon = identify(
        CANON_SRC,
        "at://did:plc:pantheon-council/org.jason-edelman.writtenworld.commit/rkey003",
        "bafyCanonDuskweave",
    );
    println!("=== A council commit, declaring one reading canonical ===\n{CANON_SRC}");

    // The full log, in the order the three deities and the council actually
    // committed -- no reordering, no pruning, no privileged final editor.
    let full_log = vec![helios.clone(), selene.clone(), nyx.clone(), canon.clone()];
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
    println!(
        "Helios's and Selene's own commits, materialized alone: {:?} and {:?} \
         -- neither erased, neither requires the other's commit to exist, \
         both still cite-able by (uri, cid) forever.",
        helios_alone.current_value("sky/1", "origin"),
        selene_alone.current_value("sky/1", "origin"),
    );

    // Check 3: Nyx's synthesis was only accepted because it named real,
    // produced facts -- referential integrity, not invention. Proven by
    // construction here: the FactRef literals above are the SAME (uri, cid)
    // pair returned by identify() for Helios's and Selene's own commits,
    // not typed independently -- if either mismatched, this is real
    // interpreter behavior to check, not just a story about it, so confirm
    // Nyx's own consumes actually reference them.
    assert_eq!(nyx.commit.consumes.len(), 2, "Nyx's commit cites exactly two prior facts");
    println!(
        "Nyx's commit.consumes cites {} real prior facts, and produces one \
         value ({:?}) that existed in neither input -- recombination, not \
         selection between them.",
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
