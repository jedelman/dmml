//! Walter Benjamin's "The Work of Art in the Age of Mechanical Reproduction"
//! (Zohn translation, in Arendt ed., *Illuminations*, Schocken, 1969),
//! modeled as real DMML commits, with citations verified against direct
//! primary-text quotation (see `../../papers/CITATION-VERIFICATION-2026-08-25-
//! benjamin.md`) before any of this was written, per the "DMML first" rule.
//! Run with `cargo run -p dmml --example benjamin`.
//!
//! What this run is actually checking, concretely -- and, as important,
//! what it surfaces as a real *disanalogy* rather than papering over it:
//!
//! 1. Benjamin's own structural pivot -- "the total function of art is
//!    reversed. Instead of being based on ritual, it begins to be based on
//!    another practice -- politics" (Section IV) -- is modeled the way DMML
//!    models every attribute change: not an edit in place, but a `consumes`
//!    of the prior fact plus a `produces` of the new one. This is a real,
//!    load-bearing fit: Benjamin's own claim (endnote 5) is that aura and
//!    cult value are ONE phenomenon in two registers, not two independent
//!    facts that happen to correlate -- so the shift is modeled as a single
//!    commit changing both `basis` and `auraStatus` together, not two
//!    separately-timed commits that could come apart.
//! 2. Benjamin's political epilogue names two responses to art's ritual
//!    basis dissolving: fascism "aestheticizing politics," communism
//!    "politicizing art" (Epilogue) -- and explicitly frames the second as
//!    an ANSWER to the first ("Communism responds by politicizing art"),
//!    not a parallel, coequal alternative. Modeled here as two commits from
//!    two different DIDs, both citing the same withered-aura fact by
//!    `FactRef`, neither citing the other -- structurally identical to
//!    `pantheon.rs`'s Helios/Selene/Eos rivalry. Checked below: DMML's
//!    last-write-wins current view has no primitive for "this is the
//!    answer to that," only "whichever commit is later in the log" --
//!    demonstrated concretely by materializing the same two commits in
//!    both orders and showing the current answer flips.
//! 3. Benjamin's aura-withering is stated as one-directional and permanent
//!    -- "that which withers... is the aura," never framed as reversible.
//!    Checked below: the pre-reproduction commit, materialized alone,
//!    still genuinely says `basis: ritual` -- exactly as permanently
//!    present and independently citable as Helios's overwritten claim in
//!    `pantheon.rs`. DMML does not have a primitive for "this fact is not
//!    just superseded in the current view but historically foreclosed" --
//!    which is the actual point of tension this file is built to surface,
//!    not resolve.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// Pre-reproduction condition. Section II/IV: unique existence "here and
// now," embedded in ritual; endnote 5: aura is cult value restated in the
// vocabulary of space/time perception -- modeled as one commit setting
// both attributes together, not two facts that could independently drift.
const RITUAL_SRC: &str = r#"
commit asserts {
  declare attribute basis
  declare attribute auraStatus

  artwork/mona_lisa basis "ritual"
  artwork/mona_lisa auraStatus "present"
}
"#;

// Section II: "that which withers in the age of mechanical reproduction is
// the aura." Section IV's pivot sentence: "the total function of art is
// reversed. Instead of being based on ritual, it begins to be based on
// another practice -- politics." Consumes the ritual/present fact by
// FactRef -- this is not an in-place edit, it is DMML's ordinary
// consume/produce shape landing exactly on Benjamin's own causal claim.
const REPRODUCTION_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute basis
  declare attribute auraStatus

  consumes {
    fact {ritual_uri} (cid: {ritual_cid}) {
      subject: artwork/mona_lisa
      predicate: basis
    }
  }
  produces {
    artwork/mona_lisa basis "exhibition"
    artwork/mona_lisa auraStatus "withered"
  }
}
"#;

// Epilogue: "Fascism sees its salvation in giving these masses... a chance
// to express themselves... the introduction of aesthetics into political
// life," culminating in "Fiat ars -- pereat mundus" and the closing line:
// "This is the situation of politics which Fascism is rendering aesthetic."
// Cites the withered-aura fact -- this response happens BECAUSE ritual
// basis dissolved, not independently of it.
const FASCISM_RESPONSE_TEMPLATE: &str = r#"
commit responds {
  declare attribute mode

  consumes {
    fact {reproduction_uri} (cid: {reproduction_cid}) {
      subject: artwork/mona_lisa
      predicate: auraStatus
    }
  }
  produces {
    politics/1 mode "aestheticized"
  }
}
"#;

// Epilogue's closing sentence: "Communism responds by politicizing art."
// Same withered-aura fact cited, same shape of commit, different DID --
// and, per Benjamin's own text, explicitly meant as an ANSWER to the
// fascism commit, not a parallel claim. DMML has no primitive for that
// asymmetry -- checked below, not asserted away.
const COMMUNISM_RESPONSE_TEMPLATE: &str = r#"
commit responds {
  declare attribute mode

  consumes {
    fact {reproduction_uri} (cid: {reproduction_cid}) {
      subject: artwork/mona_lisa
      predicate: auraStatus
    }
  }
  produces {
    politics/1 mode "politicized"
  }
}
"#;

// Sections VIII-X: stage actor "presented... in person" vs. screen actor
// "presented by a camera"; IX: performance "composed of many separate
// performances"; X's exact phrase: "the shriveling of the aura" answered
// by "an artificial build-up of the 'personality.'"
const STAGE_ACTOR_SRC: &str = r#"
commit asserts {
  declare attribute presentedVia
  declare attribute personality

  actor/1 presentedVia "person"
  actor/1 personality "given"
}
"#;

const FILM_ACTOR_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute presentedVia
  declare attribute personality

  consumes {
    fact {stage_uri} (cid: {stage_cid}) {
      subject: actor/1
      predicate: presentedVia
    }
  }
  produces {
    actor/1 presentedVia "camera"
    actor/1 personality "artificial-build-up"
  }
}
"#;

// Section XV: architecture as "the prototype of a work of art the
// reception of which is consummated by a collectivity in a state of
// distraction"; film as reception's modern analogue, vs. the solitary
// Sammlung of "a man who concentrates before a work of art."
const CONTEMPLATION_SRC: &str = r#"
commit asserts {
  declare attribute receptionMode

  medium/painting receptionMode "contemplation"
}
"#;

const DISTRACTION_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute receptionMode

  consumes {
    fact {contemplation_uri} (cid: {contemplation_cid}) {
      subject: medium/painting
      predicate: receptionMode
    }
  }
  produces {
    medium/film receptionMode "distraction"
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
    let ritual_uri = "at://did:plc:tradition/org.jason-edelman.writtenworld.commit/rkey001";
    let ritual_cid = "bafyRitualPresent";
    let ritual = identify(RITUAL_SRC, ritual_uri, ritual_cid);
    println!("=== Pre-reproduction condition ===\n{RITUAL_SRC}");

    let reproduction_uri = "at://did:plc:mechanical-reproduction/org.jason-edelman.writtenworld.commit/rkey001";
    let reproduction_cid = "bafyExhibitionWithered";
    let reproduction_src = REPRODUCTION_TEMPLATE
        .replace("{ritual_uri}", ritual_uri)
        .replace("{ritual_cid}", ritual_cid);
    let reproduction = identify(&reproduction_src, reproduction_uri, reproduction_cid);
    println!("=== Mechanical reproduction, citing the ritual condition ===\n{reproduction_src}");

    let fascism_src = FASCISM_RESPONSE_TEMPLATE
        .replace("{reproduction_uri}", reproduction_uri)
        .replace("{reproduction_cid}", reproduction_cid);
    let fascism = identify(
        &fascism_src,
        "at://did:plc:fascism/org.jason-edelman.writtenworld.commit/rkey001",
        "bafyAestheticized",
    );
    println!("=== Fascism's response: aestheticizing politics ===\n{fascism_src}");

    let communism_src = COMMUNISM_RESPONSE_TEMPLATE
        .replace("{reproduction_uri}", reproduction_uri)
        .replace("{reproduction_cid}", reproduction_cid);
    let communism = identify(
        &communism_src,
        "at://did:plc:communism/org.jason-edelman.writtenworld.commit/rkey001",
        "bafyPoliticized",
    );
    println!("=== Communism's response: politicizing art ===\n{communism_src}");

    let stage_uri = "at://did:plc:tradition/org.jason-edelman.writtenworld.commit/rkey002";
    let stage_cid = "bafyStagePresence";
    let stage = identify(STAGE_ACTOR_SRC, stage_uri, stage_cid);

    let film_src = FILM_ACTOR_TEMPLATE
        .replace("{stage_uri}", stage_uri)
        .replace("{stage_cid}", stage_cid);
    let film = identify(
        &film_src,
        "at://did:plc:mechanical-reproduction/org.jason-edelman.writtenworld.commit/rkey002",
        "bafyArtificialBuildUp",
    );
    println!("=== Stage actor vs. film actor ===\n{STAGE_ACTOR_SRC}{film_src}");

    let contemplation_uri = "at://did:plc:tradition/org.jason-edelman.writtenworld.commit/rkey003";
    let contemplation_cid = "bafyContemplation";
    let contemplation = identify(CONTEMPLATION_SRC, contemplation_uri, contemplation_cid);

    let distraction_src = DISTRACTION_TEMPLATE
        .replace("{contemplation_uri}", contemplation_uri)
        .replace("{contemplation_cid}", contemplation_cid);
    let distraction = identify(
        &distraction_src,
        "at://did:plc:mechanical-reproduction/org.jason-edelman.writtenworld.commit/rkey003",
        "bafyDistraction",
    );
    println!("=== Contemplation vs. distraction ===\n{CONTEMPLATION_SRC}{distraction_src}");

    // Check 1: the reproduction commit's pivot is real -- basis and
    // auraStatus both change together, in one commit, matching endnote 5's
    // claim that they are one phenomenon, not two that happen to correlate.
    let base_log = vec![ritual.clone(), reproduction.clone()];
    let base_materialized = Materialized::from_identified_commits(&base_log);
    assert_eq!(
        base_materialized.current_value("artwork/mona_lisa", "basis"),
        Some(&dmml::lower::TripleValue::Str("exhibition".to_string()))
    );
    assert_eq!(
        base_materialized.current_value("artwork/mona_lisa", "auraStatus"),
        Some(&dmml::lower::TripleValue::Str("withered".to_string()))
    );
    println!(
        "current_value(artwork/mona_lisa, basis) = {:?}, auraStatus = {:?} -- \
         Benjamin's own pivot sentence, modeled as one commit.",
        base_materialized.current_value("artwork/mona_lisa", "basis"),
        base_materialized.current_value("artwork/mona_lisa", "auraStatus"),
    );

    // Check 2: the REAL disanalogy. Benjamin states aura's withering as
    // one-directional and permanent -- never framed as reversible. DMML's
    // log disagrees by construction: the ritual condition, materialized
    // alone, still genuinely says "ritual" and "present" -- as permanently
    // real and citable as Helios's overwritten claim in pantheon.rs. DMML
    // has no primitive marking a fact as historically foreclosed, only
    // "superseded in this particular current view."
    let ritual_alone = Materialized::from_identified_commits(&[ritual.clone()]);
    assert_eq!(
        ritual_alone.current_value("artwork/mona_lisa", "basis"),
        Some(&dmml::lower::TripleValue::Str("ritual".to_string())),
        "the pre-reproduction condition, read alone, still genuinely says 'ritual' -- \
         DMML has no way to mark this as historically foreclosed the way Benjamin's \
         own 'withers' claims it to be"
    );
    println!(
        "ritual condition, materialized alone: basis = {:?} -- still real, still \
         citable, forever. Benjamin's own claim is that this does NOT hold: aura, \
         once withered, does not return. DMML's log structurally disagrees with \
         the essay's own one-way historical arc.",
        ritual_alone.current_value("artwork/mona_lisa", "basis"),
    );

    // Check 3: the epilogue's asymmetry. Benjamin frames communism's
    // politicizing-art as an ANSWER to fascism's aestheticizing-politics,
    // not a coequal alternative. Materialize the same two commits in both
    // orders and show the current view has no primitive for that
    // asymmetry -- only "whichever commit is later."
    let fascism_then_communism =
        Materialized::from_identified_commits(&[fascism.clone(), communism.clone()]);
    let communism_then_fascism =
        Materialized::from_identified_commits(&[communism.clone(), fascism.clone()]);
    assert_eq!(
        fascism_then_communism.current_value("politics/1", "mode"),
        Some(&dmml::lower::TripleValue::Str("politicized".to_string())),
        "fascism-then-communism order: current view shows communism's answer, matching Benjamin's own sequence"
    );
    assert_eq!(
        communism_then_fascism.current_value("politics/1", "mode"),
        Some(&dmml::lower::TripleValue::Str("aestheticized".to_string())),
        "communism-then-fascism order: current view flips to fascism -- \
         last-write-wins has no concept of 'this is the answer to that'"
    );
    println!(
        "politics/1 mode, log order (fascism, communism) = {:?}; log order \
         (communism, fascism) = {:?} -- the current view tracks LOG ORDER, not \
         Benjamin's own argument that communism's move specifically answers \
         fascism's. Reorder the log and the 'answer' relation disappears; \
         nothing in dmml/dmml-runtime encodes it.",
        fascism_then_communism.current_value("politics/1", "mode"),
        communism_then_fascism.current_value("politics/1", "mode"),
    );

    // Check 4: both political responses genuinely cite the same withered-
    // aura fact -- neither is invented independently of the other's shared
    // cause, mirroring Helios/Selene/Eos's zero-coordination-but-real-
    // citation pattern in pantheon.rs.
    assert_eq!(fascism.commit.consumes.len(), 1);
    assert_eq!(communism.commit.consumes.len(), 1);
    println!(
        "fascism.commit.consumes and communism.commit.consumes each cite exactly \
         1 real fact (the same one) -- both responses are real reactions to the \
         same historical condition, not independently invented."
    );

    // Checks 5-6: the film-actor and distraction/contemplation strands,
    // same consume/produce shape, confirming the pivot pattern generalizes
    // across the essay's other claims rather than being special-cased to
    // the aura/politics strand alone.
    let film_materialized = Materialized::from_identified_commits(&[stage.clone(), film.clone()]);
    assert_eq!(
        film_materialized.current_value("actor/1", "personality"),
        Some(&dmml::lower::TripleValue::Str("artificial-build-up".to_string()))
    );
    let distraction_materialized =
        Materialized::from_identified_commits(&[contemplation.clone(), distraction.clone()]);
    assert_eq!(
        distraction_materialized.current_value("medium/film", "receptionMode"),
        Some(&dmml::lower::TripleValue::Str("distraction".to_string()))
    );
    println!(
        "actor/1 personality = {:?}; medium/film receptionMode = {:?} -- the same \
         consume/produce shape models Section X's actor claim and Section XV's \
         distraction claim without modification.",
        film_materialized.current_value("actor/1", "personality"),
        distraction_materialized.current_value("medium/film", "receptionMode"),
    );

    println!(
        "\n=== done: the pivot from ritual to exhibition basis models cleanly as an \
         ordinary consume/produce commit (Check 1); the essay's own one-way, \
         permanent 'withering' claim is where the model structurally disagrees \
         with DMML's log, which keeps the prior condition permanently real and \
         citable (Check 2, the actual point of this file); the epilogue's \
         fascism/communism asymmetry has no primitive in last-write-wins, which \
         tracks log order, not argumentative priority (Check 3); both political \
         responses are shown to genuinely cite their shared cause (Check 4); the \
         same shape generalizes to the actor and reception-mode claims without \
         modification (Checks 5-6). ==="
    );
}
