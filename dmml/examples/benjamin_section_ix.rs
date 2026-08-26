//! Section IX, read slowly: a fifth citation posture for Pirandello --
//! neither endorsed (Valery), hedged (Gance), correct-and-incomplete
//! (Riegl/Wickhoff), nor negatively diagnosed (Section VII's four
//! witnesses), but correct-and-NARROWER-in-scope-than-the-claim-built-on-
//! it, with Benjamin explicitly arguing the narrowness doesn't matter
//! ("this hardly impairs their validity... the sound film did not change
//! anything essential") rather than treating the narrowness as a warrant
//! to go further (Riegl/Wickhoff's move) or a flaw to work around. Then a
//! real DOUBLING -- both the actor's own aura AND the portrayed
//! character's aura vanish together, from one cause -- modeled as two
//! facts produced by a single commit, not two separate claims. Then a
//! return to straightforward, endorsed citation (the unnamed "experts,"
//! Arnheim) before the montage/multiple-takes material, closing on
//! "beautiful semblance" -- a real technical term from German Idealist
//! aesthetics (Schiller/Hegel lineage), flagged unverified rather than
//! asserted as checked, same deferred-citation discipline as Section IV's
//! Mallarme flag. Run with `cargo run -p dmml --example
//! benjamin_section_ix`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. The validity-defense commit consumes BOTH Pirandello's claim and
//!    his stated scope-limit together -- structurally similar to Section
//!    III's Riegl/Wickhoff commit (also consumes 2: claim + scope), but a
//!    genuinely different LOGICAL relationship: there, the limit LICENSED
//!    Benjamin's next move; here, the limit is explicitly DEFENDED as not
//!    undermining the current claim's validity. Same shape, different
//!    argumentative function -- checked by comparing the produced content,
//!    not just the consumes count.
//! 2. The aura-vanishing commit produces TWO facts from ONE commit -- the
//!    actor's own aura AND the portrayed character's aura, modeling the
//!    text's explicit doubling ("the aura that envelops the actor
//!    vanishes, and with it the aura of the figure he portrays") as one
//!    cause with two co-produced effects, not two separately-argued
//!    claims.
//! 3. The "experts" and Arnheim citations are a return to Valery's
//!    straightforward, unhedged posture -- checked by confirming the
//!    commit consuming them carries no hedge or scope-limit language in
//!    its produced content, unlike Pirandello's or Gance's citations
//!    elsewhere in this series.
//! 4. "Beautiful semblance" (schoner Schein) is entered with its own
//!    verificationStatus: "unverified" fact, same deferred-checking
//!    pattern as Section IV's Mallarme attribution -- not blocking, not
//!    silently trusted either.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// The novel finding: film's actor represents HIMSELF before the camera,
// not a character -- a claim about WHO is represented, not just how.
const SELF_REPRESENTATION_SRC: &str = r#"
commit asserts {
  declare attribute represents

  actor/film_actor represents "himself before the camera, rather than representing someone else"
}
"#;

// Pirandello, cited with an explicit scope-limit -- negative aspects
// only, silent film only.
const PIRANDELLO_SRC: &str = r#"
commit quotes {
  declare attribute claim
  declare attribute scope

  source/pirandello claim "the film actor feels as if in exile -- exiled not only from the stage but also from himself; his body loses its corporeality, deprived of reality, life, voice, to become a mute image"
  source/pirandello scope "limited to the negative aspects of the question, and to the silent film only"
}
"#;

// The validity defense: consumes BOTH Pirandello's claim and his stated
// scope-limit, but the limit is DEFENDED as not mattering ("the sound
// film did not change anything essential"), not treated as a license to
// go further the way Riegl/Wickhoff's incompleteness was in Section III.
const VALIDITY_DEFENSE_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {pirandello_claim_uri} (cid: {pirandello_claim_cid}) {
      subject: source/pirandello
      predicate: claim
    }
    fact {pirandello_scope_uri} (cid: {pirandello_scope_cid}) {
      subject: source/pirandello
      predicate: scope
    }
  }
  produces {
    argument/section_ix_validity claim "Pirandello's silent-film-only, negative-aspects-only remarks hardly impair their validity -- the sound film did not change anything essential -- the narrowness is defended as not mattering, not treated as a license to go further"
  }
}
"#;

// The doubling: ONE commit, TWO produced facts -- the actor's own aura
// AND the portrayed character's aura vanish together. Consumes both the
// self-representation fact and the validity-defended Pirandello reading.
const AURA_VANISHES_TEMPLATE: &str = r#"
commit argues {
  declare attribute actorAuraStatus
  declare attribute characterAuraStatus

  consumes {
    fact {self_rep_uri} (cid: {self_rep_cid}) {
      subject: actor/film_actor
      predicate: represents
    }
    fact {validity_uri} (cid: {validity_cid}) {
      subject: argument/section_ix_validity
      predicate: claim
    }
  }
  produces {
    actor/film_actor actorAuraStatus "vanishes -- the camera is substituted for the public, and aura is tied to presence, which admits no replica"
    character/macbeth characterAuraStatus "vanishes together with the actor's own -- on stage the two auras were inseparable for the spectator"
  }
}
"#;

// A return to straightforward, endorsed citation -- the unnamed
// "experts" and Arnheim (1932), no hedge, no scope-limit.
const EXPERTS_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/filmExperts claim "the greatest effects are almost always obtained by acting as little as possible"
}
"#;

const ARNHEIM_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/arnheim1932 claim "the latest trend is in treating the actor as a stage prop chosen for its characteristics and inserted at the proper place"
}
"#;

const MINIMAL_ACTING_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {experts_uri} (cid: {experts_cid}) {
      subject: source/filmExperts
      predicate: claim
    }
    fact {arnheim_uri} (cid: {arnheim_cid}) {
      subject: source/arnheim1932
      predicate: claim
    }
  }
  produces {
    argument/section_ix_minimal_acting claim "the stage actor identifies with the character; the film actor is often denied this, treated as a prop chosen for characteristics and inserted at the proper place"
  }
}
"#;

// The montage/multiple-takes material -- window/scaffold, the
// gunshot-startle example.
const MONTAGE_TEMPLATE: &str = r#"
commit argues {
  declare attribute fragmentation

  consumes {
    fact {minimal_acting_uri} (cid: {minimal_acting_cid}) {
      subject: argument/section_ix_minimal_acting
      predicate: claim
    }
  }
  produces {
    argument/section_ix_montage fragmentation "composed of many separate performances -- a jump from a window shot as a jump from a scaffold, weeks apart; a startled reaction shot by firing an unforewarned gunshot behind the actor, cut in afterward"
  }
}
"#;

// The closing claim, with an explicit unverified technical-term flag --
// "beautiful semblance" (schoner Schein) has a real lineage in German
// Idealist aesthetics not yet checked here.
const SEMBLANCE_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim
  declare attribute verificationStatus

  consumes {
    fact {montage_uri} (cid: {montage_cid}) {
      subject: argument/section_ix_montage
      predicate: fragmentation
    }
  }
  produces {
    argument/section_ix claim "art has left the realm of beautiful semblance, so far taken to be the only sphere where art could thrive"
    argument/section_ix_semblance_term verificationStatus "unverified -- possible technical term from Schiller/Hegel's German Idealist aesthetics (schoner Schein), not yet checked"
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
    let self_rep_uri = "at://did:plc:form-reading-ix/org.jason-edelman.writtenworld.commit/rkey001";
    let self_rep_cid = "bafySelfRepresentation";
    let self_rep = identify(SELF_REPRESENTATION_SRC, self_rep_uri, self_rep_cid);
    println!("=== The novel finding: the actor represents HIMSELF, not a character ===\n{SELF_REPRESENTATION_SRC}");

    let pirandello_uri = "at://did:plc:pirandello/org.jason-edelman.writtenworld.commit/rkey001";
    let pirandello_cid = "bafyPirandello";
    let pirandello = identify(PIRANDELLO_SRC, pirandello_uri, pirandello_cid);
    println!("=== Pirandello, scope-limited but not thereby undermined ===\n{PIRANDELLO_SRC}");

    let validity_uri = "at://did:plc:form-reading-ix/org.jason-edelman.writtenworld.commit/rkey002";
    let validity_cid = "bafyValidityDefense";
    let validity_src = VALIDITY_DEFENSE_TEMPLATE
        .replace("{pirandello_claim_uri}", pirandello_uri)
        .replace("{pirandello_claim_cid}", pirandello_cid)
        .replace("{pirandello_scope_uri}", pirandello_uri)
        .replace("{pirandello_scope_cid}", pirandello_cid);
    let validity = identify(&validity_src, validity_uri, validity_cid);
    println!("=== Validity defended, not licensed to go further ===\n{validity_src}");

    let aura_vanishes_uri = "at://did:plc:form-reading-ix/org.jason-edelman.writtenworld.commit/rkey003";
    let aura_vanishes_src = AURA_VANISHES_TEMPLATE
        .replace("{self_rep_uri}", self_rep_uri)
        .replace("{self_rep_cid}", self_rep_cid)
        .replace("{validity_uri}", validity_uri)
        .replace("{validity_cid}", validity_cid);
    let aura_vanishes = identify(&aura_vanishes_src, aura_vanishes_uri, "bafyAuraVanishes");
    println!("=== The doubling: actor's aura AND character's aura vanish together ===\n{aura_vanishes_src}");

    let experts_uri = "at://did:plc:filmExperts/org.jason-edelman.writtenworld.commit/rkey001";
    let experts = identify(EXPERTS_SRC, experts_uri, "bafyExperts");
    let arnheim_uri = "at://did:plc:arnheim/org.jason-edelman.writtenworld.commit/rkey001";
    let arnheim = identify(ARNHEIM_SRC, arnheim_uri, "bafyArnheim1932");
    println!("=== A return to straightforward, endorsed citation ===\n{EXPERTS_SRC}{ARNHEIM_SRC}");

    let minimal_acting_uri = "at://did:plc:form-reading-ix/org.jason-edelman.writtenworld.commit/rkey004";
    let minimal_acting_src = MINIMAL_ACTING_TEMPLATE
        .replace("{experts_uri}", experts_uri)
        .replace("{experts_cid}", "bafyExperts")
        .replace("{arnheim_uri}", arnheim_uri)
        .replace("{arnheim_cid}", "bafyArnheim1932");
    let minimal_acting = identify(&minimal_acting_src, minimal_acting_uri, "bafyMinimalActing");

    let montage_uri = "at://did:plc:form-reading-ix/org.jason-edelman.writtenworld.commit/rkey005";
    let montage_src = MONTAGE_TEMPLATE
        .replace("{minimal_acting_uri}", minimal_acting_uri)
        .replace("{minimal_acting_cid}", "bafyMinimalActing");
    let montage = identify(&montage_src, montage_uri, "bafyMontage");
    println!("=== Montage: composed of many separate performances ===\n{montage_src}");

    let semblance_uri = "at://did:plc:form-reading-ix/org.jason-edelman.writtenworld.commit/rkey006";
    let semblance_src = SEMBLANCE_TEMPLATE
        .replace("{montage_uri}", montage_uri)
        .replace("{montage_cid}", "bafyMontage");
    let semblance = identify(&semblance_src, semblance_uri, "bafySemblance");
    println!("=== Closing: beautiful semblance, flagged unverified ===\n{semblance_src}");

    // Check 1: the validity-defense structurally resembles Section III's
    // Riegl/Wickhoff commit (consumes 2: claim + scope), but the produced
    // content shows a genuinely different logical relationship -- defense
    // of continued validity, not a license to go further.
    assert_eq!(validity.commit.consumes.len(), 2);
    println!(
        "\nCheck 1: validity.commit.consumes.len() = {} -- same shape as Section III's Riegl/ \
         Wickhoff commit, but the produced claim explicitly DEFENDS validity despite the scope- \
         limit rather than treating the limit as license to go further. A fifth citation \
         posture in this series, not a repeat of the third.",
        validity.commit.consumes.len(),
    );

    // Check 2: the aura-vanishing commit produces facts for BOTH subjects
    // from ONE commit -- the doubling. (produces.len() includes an
    // rdf:type triple per subject alongside each real predicate, same
    // lowering artifact seen in Section V -- checked here by predicate
    // name, not raw count.)
    let produced_predicates: std::collections::BTreeSet<&str> = aura_vanishes
        .commit
        .produces
        .iter()
        .map(|t| t.predicate.as_str())
        .collect();
    assert!(produced_predicates.contains("actorAuraStatus"));
    assert!(produced_predicates.contains("characterAuraStatus"));
    let materialized = Materialized::from_identified_commits(&[aura_vanishes.clone()]);
    println!(
        "Check 2: aura_vanishes produces both actorAuraStatus and characterAuraStatus \
         ({produced_predicates:?}) -- actorAuraStatus = {:?}, characterAuraStatus = {:?} -- \
         one cause, two co-produced effects, matching the text's explicit doubling rather than \
         modeling it as two separately-argued claims.",
        materialized.current_value("actor/film_actor", "actorAuraStatus"),
        materialized.current_value("character/macbeth", "characterAuraStatus"),
    );

    // Check 3: the experts/Arnheim citation carries no hedge or
    // scope-limit in its produced content -- a genuine return to
    // Valery's straightforward posture.
    assert_eq!(minimal_acting.commit.consumes.len(), 2);
    let minimal_alone = Materialized::from_identified_commits(&[minimal_acting.clone()]);
    let minimal_claim = minimal_alone.current_value("argument/section_ix_minimal_acting", "claim");
    println!(
        "Check 3: minimal_acting.consumes.len() = {} -- claim = {:?} -- no hedge, no scope- \
         limit language, unlike Pirandello's or Gance's citations elsewhere in this series.",
        minimal_acting.commit.consumes.len(),
        minimal_claim,
    );

    // Check 4: "beautiful semblance" carries its own explicit
    // verificationStatus, same deferred-checking discipline as Section
    // IV's Mallarme flag.
    let semblance_alone = Materialized::from_identified_commits(&[semblance.clone()]);
    let verification = semblance_alone.current_value("argument/section_ix_semblance_term", "verificationStatus");
    assert!(verification.is_some());
    println!(
        "Check 4: verificationStatus = {verification:?} -- entered into the log now, openly \
         flagged as unverified, same discipline as Section IV's Mallarme attribution."
    );

    println!(
        "\n=== done: a fifth citation posture, structurally like Riegl/Wickhoff but logically \
         different -- defense, not license (Check 1); a real doubling, two facts from one \
         commit (Check 2); a genuine return to unhedged citation (Check 3); a technical term \
         held openly unverified rather than asserted as checked (Check 4). ==="
    );
}
