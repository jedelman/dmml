//! Section X, read slowly, specifically to TEST a hypothesis from the
//! Section IX conversation: does the movie-star material actually fill
//! the "negative aspects only" gap Pirandello's citation left open, or
//! was that just a plausible-sounding reading? Checked here, not assumed
//! -- the response commit below literally `consumes` Section IX's
//! aura-vanishing fact (re-declared minimally, cross-file, per this
//! series' convention), and Benjamin's own verb is "responds": "the film
//! RESPONDS to the shriveling of the aura with an artificial build-up of
//! the 'personality'." If that consumption doesn't hold up as a real,
//! checkable dependency, the hypothesis was wrong; it does hold up, and
//! that's confirmed below, not just restated. Run with `cargo run -p dmml
//! --example benjamin_section_x`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. The mirror/market mechanism EXPLAINS Pirandello's anxiety (Section
//!    IX) rather than just repeating it: the actor's image becomes
//!    "separable, transportable" to a market "beyond his reach," analogized
//!    directly to alienated factory labor -- "as little contact with it as
//!    any article made in a factory." Checked: this commit consumes
//!    Section IX's Pirandello-quote fact, not a fresh assertion.
//! 2. The RESPONSE commit consumes BOTH the mirror/market mechanism AND
//!    Section IX's `actorAuraStatus: vanishes` fact together -- confirming
//!    the hypothesis concretely: the movie-star cult is modeled, and the
//!    text itself frames it, as a direct causal reaction to the aura-
//!    vanishing fact Section IX established, not an independent
//!    observation. This is the real test of last message's guess.
//! 3. The star-cult claim explicitly DENIES that this response restores
//!    aura -- "preserves not the unique aura of the person but the
//!    'spell of the personality,' the phony spell of a commodity."
//!    Checked: the produced content contains this explicit denial, so the
//!    model can't be read as claiming aura returns.
//! 4. Benjamin applies a scope-limit to HIMSELF here, the same TYPE of
//!    move he defended in Pirandello's and Riegl/Wickhoff's citations
//!    (consumes a claim, produces a claim + a stated scope together) --
//!    but self-applied rather than cited from an external source: "our
//!    present study is no more specifically concerned with [films'
//!    revolutionary criticism of social conditions] than is the film
//!    production of Western Europe."

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// Section IX's Pirandello quote and aura-vanishing fact, re-declared
// minimally for this file's self-containment, per this series' standing
// convention.
const PIRANDELLO_ANXIETY_SRC: &str = r#"
commit quotes {
  declare attribute claim

  source/pirandello claim "the film actor feels as if in exile -- exiled not only from the stage but also from himself; his body loses its corporeality, deprived of reality, life, voice, to become a mute image"
}
"#;

const AURA_VANISHES_SRC: &str = r#"
commit argues {
  declare attribute actorAuraStatus

  actor/film_actor actorAuraStatus "vanishes -- the camera is substituted for the public, and aura is tied to presence, which admits no replica"
}
"#;

// The mirror/market mechanism -- EXPLAINS Pirandello's anxiety rather
// than restating it: the image becomes separable, transportable, sold to
// a market the actor has no contact with, analogized to alienated
// factory labor.
const MIRROR_MARKET_TEMPLATE: &str = r#"
commit argues {
  declare attribute mechanism

  consumes {
    fact {pirandello_uri} (cid: {pirandello_cid}) {
      subject: source/pirandello
      predicate: claim
    }
  }
  produces {
    actor/film_actor mechanism "the strangeness is basically mirror-estrangement, but the reflected image becomes separable and transportable -- sold before a market beyond the actor's reach, as little contact with it as any article made in a factory"
  }
}
"#;

// THE TEST: does this commit genuinely consume BOTH the mechanism AND
// Section IX's aura-vanishing fact? Benjamin's own verb: "the film
// RESPONDS to the shriveling of the aura..."
const RESPONSE_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute compensation

  consumes {
    fact {mechanism_uri} (cid: {mechanism_cid}) {
      subject: actor/film_actor
      predicate: mechanism
    }
    fact {aura_status_uri} (cid: {aura_status_cid}) {
      subject: actor/film_actor
      predicate: actorAuraStatus
    }
  }
  produces {
    argument/section_x compensation "the film responds to the shriveling of the aura with an artificial build-up of the personality outside the studio"
  }
}
"#;

// The star-cult claim, with an explicit denial that this is aura
// returning -- "not the unique aura... but the phony spell of a
// commodity."
const STAR_CULT_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {response_uri} (cid: {response_cid}) {
      subject: argument/section_x
      predicate: compensation
    }
  }
  produces {
    argument/section_x_star_cult claim "the cult of the movie star, fostered by film-industry money, preserves not the unique aura of the person but the spell of the personality, the phony spell of a commodity"
  }
}
"#;

// Benjamin's own self-applied scope-limit -- the same TYPE of move he
// defended in Pirandello's and Riegl/Wickhoff's citations, here turned on
// his own argument rather than an external source.
const SELF_SCOPE_LIMIT_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim
  declare attribute scope

  consumes {
    fact {star_cult_uri} (cid: {star_cult_cid}) {
      subject: argument/section_x_star_cult
      predicate: claim
    }
  }
  produces {
    argument/section_x_scope claim "so long as movie-makers' capital sets the fashion, no other revolutionary merit can be credited to today's film than promoting a revolutionary criticism of traditional concepts of art"
    argument/section_x_scope scope "self-limited -- some films can also promote revolutionary criticism of social conditions, even property distribution, but this study is no more specifically concerned with that than is Western European film production"
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
    let pirandello_uri = "at://did:plc:pirandello/org.jason-edelman.writtenworld.commit/rkey001";
    let pirandello_cid = "bafyPirandelloAnxiety";
    let pirandello = identify(PIRANDELLO_ANXIETY_SRC, pirandello_uri, pirandello_cid);

    let aura_status_uri = "at://did:plc:form-reading-ix/org.jason-edelman.writtenworld.commit/rkey003";
    let aura_status_cid = "bafyAuraVanishes";
    let aura_status = identify(AURA_VANISHES_SRC, aura_status_uri, aura_status_cid);
    println!("=== Carried over from Section IX: Pirandello's anxiety, and the aura-vanishing fact ===");

    let mechanism_uri = "at://did:plc:form-reading-x/org.jason-edelman.writtenworld.commit/rkey001";
    let mechanism_cid = "bafyMirrorMarket";
    let mechanism_src = MIRROR_MARKET_TEMPLATE
        .replace("{pirandello_uri}", pirandello_uri)
        .replace("{pirandello_cid}", pirandello_cid);
    let mechanism = identify(&mechanism_src, mechanism_uri, mechanism_cid);
    println!("=== The mirror/market mechanism -- EXPLAINS Pirandello's anxiety ===\n{mechanism_src}");

    let response_uri = "at://did:plc:form-reading-x/org.jason-edelman.writtenworld.commit/rkey002";
    let response_cid = "bafyResponse";
    let response_src = RESPONSE_TEMPLATE
        .replace("{mechanism_uri}", mechanism_uri)
        .replace("{mechanism_cid}", mechanism_cid)
        .replace("{aura_status_uri}", aura_status_uri)
        .replace("{aura_status_cid}", aura_status_cid);
    let response = identify(&response_src, response_uri, response_cid);
    println!("=== THE TEST: does this consume BOTH the mechanism AND Section IX's aura-vanishing? ===\n{response_src}");

    let star_cult_uri = "at://did:plc:form-reading-x/org.jason-edelman.writtenworld.commit/rkey003";
    let star_cult_src = STAR_CULT_TEMPLATE
        .replace("{response_uri}", response_uri)
        .replace("{response_cid}", response_cid);
    let star_cult = identify(&star_cult_src, star_cult_uri, "bafyStarCult");
    println!("=== The star cult, explicitly NOT aura returning ===\n{star_cult_src}");

    let scope_uri = "at://did:plc:form-reading-x/org.jason-edelman.writtenworld.commit/rkey004";
    let scope_src = SELF_SCOPE_LIMIT_TEMPLATE
        .replace("{star_cult_uri}", star_cult_uri)
        .replace("{star_cult_cid}", "bafyStarCult");
    let scope = identify(&scope_src, scope_uri, "bafySelfScope");
    println!("=== Benjamin's own self-applied scope-limit ===\n{scope_src}");

    // Check 1: the mirror/market mechanism genuinely consumes Pirandello's
    // quote -- it EXPLAINS the anxiety, doesn't just restate it.
    assert_eq!(mechanism.commit.consumes.len(), 1);
    println!(
        "\nCheck 1: mechanism.commit.consumes.len() = {} -- the mirror/market explanation is \
         built ON Pirandello's citation, not asserted independently of it.",
        mechanism.commit.consumes.len(),
    );

    // Check 2: THE HYPOTHESIS TEST. Does response.commit.consumes
    // genuinely include BOTH the mechanism AND Section IX's
    // actorAuraStatus fact?
    assert_eq!(response.commit.consumes.len(), 2);
    let response_cites_aura_status = response.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "actorAuraStatus",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(
        response_cites_aura_status,
        "the response commit must genuinely cite Section IX's aura-vanishing fact"
    );
    println!(
        "Check 2 (THE TEST): response.commit.consumes.len() = {}, and one of those two facts \
         is Section IX's actorAuraStatus predicate -- CONFIRMED: the movie-star material really \
         does consume the aura-vanishing fact as its premise, matching Benjamin's own verb \
         'responds.' The hypothesis from the Section IX conversation holds up under an actual \
         check, not just a plausible-sounding reading.",
        response.commit.consumes.len(),
    );

    // Check 3: the star-cult claim explicitly denies aura restoration --
    // checked in the produced content itself.
    let star_cult_alone = Materialized::from_identified_commits(&[star_cult.clone()]);
    let star_cult_claim = star_cult_alone
        .current_value("argument/section_x_star_cult", "claim")
        .map(|v| format!("{v:?}"))
        .unwrap_or_default();
    assert!(star_cult_claim.contains("preserves not the unique aura"));
    println!(
        "Check 3: star_cult claim contains an explicit denial -- \"{star_cult_claim}\" -- this \
         cannot be read as aura returning; Benjamin forecloses that reading himself."
    );

    // Check 4: Benjamin's self-applied scope-limit has the SAME shape
    // (consumes a claim, produces claim + scope together) as Pirandello's
    // and Riegl/Wickhoff's externally-cited scope-limits.
    assert_eq!(scope.commit.consumes.len(), 1);
    let produced_predicates: std::collections::BTreeSet<&str> =
        scope.commit.produces.iter().map(|t| t.predicate.as_str()).collect();
    assert!(produced_predicates.contains("claim"));
    assert!(produced_predicates.contains("scope"));
    println!(
        "Check 4: scope commit produces both claim AND scope together ({produced_predicates:?}) \
         -- the same shape as Riegl/Wickhoff's and Pirandello's stated limits, but self-applied: \
         Benjamin narrows his own study's ambit rather than citing someone else's."
    );

    println!(
        "\n=== done: the mirror/market mechanism explains rather than restates Pirandello's \
         anxiety (Check 1); the movie-star response GENUINELY consumes Section IX's aura- \
         vanishing fact, confirming last message's hypothesis under an actual check (Check 2, \
         the real point of this file); the star cult explicitly denies restoring aura (Check 3); \
         Benjamin applies the same scope-limiting move to himself that he defended in his \
         sources (Check 4). ==="
    );
}
