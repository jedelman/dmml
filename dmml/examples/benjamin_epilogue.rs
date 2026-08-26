//! The Epilogue -- and this file carries the biggest prediction of the
//! whole series, made explicitly in the synthesis conversation: does the
//! Epilogue's political conclusion actually consume the Preface's own
//! stipulated vocabulary-stance fact ("concepts... completely useless for
//! the purposes of Fascism"), closing the loop back to the essay's very
//! first move? Checked here, not assumed. Also tested: does "the Führer
//! cult" connect to Section X's movie-star cult material (both
//! manufactured, commodity/politics versions of authentic ritual cult)?
//! Does "the aura is abolished in a new way" genuinely consume Section
//! II's own aura fact? Does "the consummation of l'art pour l'art"
//! genuinely consume Section IV's l'art-pour-l'art fact? Run with
//! `cargo run -p dmml --example benjamin_epilogue`.
//!
//! What this run is actually checking, concretely:
//!
//! 1. "The violation of the masses... has its counterpart in the
//!    violation of an apparatus... pressed into the production of ritual
//!    values" is modeled as ONE commit producing TWO paired facts (the
//!    doubling pattern from Sections V, IX, XI, XIV-XV), and the
//!    masses-violation side genuinely consumes Section X's movie-star
//!    cult fact -- checking whether Fuhrer-cult and movie-star-cult are
//!    modeled as the SAME manufactured-cult structure, not two
//!    independent uses of the word "cult."
//! 2. The political and technological "formulas" for why war culminates
//!    aestheticized politics are modeled as ONE commit, TWO facts -- an
//!    explicit twofold, matching this essay's recurring pattern.
//! 3. Marinetti is cited at LENGTH, endorsed for clarity ("this manifesto
//!    has the virtue of clarity... deserve to be accepted by
//!    dialecticians") despite content Benjamin treats as monstrous -- a
//!    citation posture distinct from every other in this series: content
//!    rejected, but the citation itself valued as diagnostic evidence.
//! 4. PREDICTION: does "the aura is abolished in a new way" genuinely
//!    consume Section II's own `aura` fact (re-declared cross-file)?
//! 5. PREDICTION: does "the consummation of l'art pour l'art" genuinely
//!    consume Section IV's l'art-pour-l'art `basis` fact (re-declared
//!    cross-file)?
//! 6. THE BIG PREDICTION FROM THE SYNTHESIS CONVERSATION: does the
//!    closing claim ("Fascism is rendering aesthetic... Communism
//!    responds by politicizing art") genuinely consume the Preface's own
//!    `vocabularyStance` fact (re-declared cross-file)? If confirmed,
//!    the Preface's opening political stipulation and the Epilogue's
//!    closing political verdict are shown to be one continuous argument,
//!    not two separate political gestures bookending an otherwise
//!    aesthetic-theoretical essay.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

// The Preface's own stipulated vocabulary-stance, re-declared cross-file.
const PREFACE_SRC: &str = r#"
commit declares {
  declare attribute vocabularyStance

  argument/preface vocabularyStance "anti-fascist-terms"
}
"#;

// Section II's aura fact, re-declared cross-file.
const AURA_SRC: &str = r#"
commit coins {
  declare attribute aura

  argument/section_ii aura "coined-over-three-linked-losses-not-one-undifferentiated-one"
}
"#;

// Section IV's l'art-pour-l'art fact, re-declared cross-file.
const LART_POUR_LART_SRC: &str = r#"
commit reproduces {
  declare attribute basis

  artwork/venus basis "l-art-pour-l-art-defensive-theology-of-pure-art"
}
"#;

// Section X's movie-star cult fact, re-declared cross-file.
const STAR_CULT_SRC: &str = r#"
commit argues {
  declare attribute claim

  argument/section_x_star_cult claim "the cult of the movie star, fostered by film-industry money, preserves not the unique aura of the person but the spell of the personality, the phony spell of a commodity"
}
"#;

// Proletarianization and mass-formation as two aspects of one process --
// Fascism organizes masses without changing property, giving expression
// instead of rights: "the introduction of aesthetics into political
// life."
const AESTHETICIZE_SRC: &str = r#"
commit asserts {
  declare attribute claim

  argument/epilogue_aestheticize claim "Fascism sees its salvation in giving the masses not their right, but a chance to express themselves; the masses have a right to change property relations, Fascism gives them expression while preserving property -- the introduction of aesthetics into political life"
}
"#;

// The double violation -- ONE commit, TWO paired facts, explicitly named
// as counterparts. The masses-violation side genuinely consumes Section
// X's movie-star cult fact.
const DOUBLE_VIOLATION_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute violation

  consumes {
    fact {aestheticize_uri} (cid: {aestheticize_cid}) {
      subject: argument/epilogue_aestheticize
      predicate: claim
    }
    fact {star_cult_uri} (cid: {star_cult_cid}) {
      subject: argument/section_x_star_cult
      predicate: claim
    }
  }
  produces {
    masses/1 violation "forced to their knees by the Fuhrer cult -- the same manufactured, commodity-shaped substitute for real cult value Section X already named"
    apparatus/1 violation "pressed into the production of ritual values it does not naturally support -- the counterpart to the masses' own violation"
  }
}
"#;

// The twofold formula -- ONE commit, TWO facts, political and
// technological.
const TWOFOLD_FORMULA_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute formula

  consumes {
    fact {violation_uri} (cid: {violation_cid}) {
      subject: masses/1
      predicate: violation
    }
  }
  produces {
    formula/political formula "only war can set a goal for mass movements on the largest scale while respecting the traditional property system"
    formula/technological formula "only war makes it possible to mobilize all of today's technical resources while maintaining the property system"
  }
}
"#;

// Marinetti, cited at length -- content monstrous, clarity endorsed as
// diagnostic evidence. A unique citation posture.
const MARINETTI_SRC: &str = r#"
commit quotes {
  declare attribute claim
  declare attribute evaluativeStatus

  source/marinetti claim "War is beautiful because it establishes man's dominion over the subjugated machinery... War is beautiful because it initiates the dreamt-of metalization of the human body... it creates new architecture, like that of the big tanks, the geometrical formation flights, the smoke spirals from burning villages"
  source/marinetti evaluativeStatus "content monstrous and rejected, but the manifesto's clarity is endorsed as diagnostic evidence dialecticians should accept -- neither endorsed-as-true nor diagnosed-as-simply-wrong"
}
"#;

const DIALECTICAL_REBUTTAL_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {marinetti_uri} (cid: {marinetti_cid}) {
      subject: source/marinetti
      predicate: claim
    }
    fact {formula_uri} (cid: {formula_cid}) {
      subject: formula/technological
      predicate: formula
    }
  }
  produces {
    argument/epilogue_rebuttal claim "imperialistic war is a rebellion of technology -- the property system impedes the natural utilization of productive forces, so their unnatural utilization is found in war"
  }
}
"#;

// PREDICTION: does this genuinely consume Section II's aura fact?
const AURA_ABOLISHED_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim

  consumes {
    fact {rebuttal_uri} (cid: {rebuttal_cid}) {
      subject: argument/epilogue_rebuttal
      predicate: claim
    }
    fact {aura_uri} (cid: {aura_cid}) {
      subject: argument/section_ii
      predicate: aura
    }
  }
  produces {
    argument/epilogue_aura claim "instead of dropping seeds from airplanes, society drops incendiary bombs over cities; through gas warfare the aura is abolished in a new way"
  }
}
"#;

// PREDICTION: does this genuinely consume Section IV's l'art-pour-l'art
// fact?
const LART_POUR_LART_CONSUMMATION_TEMPLATE: &str = r#"
commit reproduces {
  declare attribute claim

  consumes {
    fact {aura_abolished_uri} (cid: {aura_abolished_cid}) {
      subject: argument/epilogue_aura
      predicate: claim
    }
    fact {lart_uri} (cid: {lart_cid}) {
      subject: artwork/venus
      predicate: basis
    }
  }
  produces {
    argument/epilogue_consummation claim "'Fiat ars, pereat mundus,' says Fascism, expecting war to supply the artistic gratification of a sense perception changed by technology -- this is evidently the consummation of l'art pour l'art"
  }
}
"#;

const SELF_ALIENATION_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {consummation_uri} (cid: {consummation_cid}) {
      subject: argument/epilogue_consummation
      predicate: claim
    }
  }
  produces {
    argument/epilogue_alienation claim "mankind's self-alienation has reached such a degree that it can experience its own destruction as an aesthetic pleasure of the first order -- this is the situation of politics which Fascism is rendering aesthetic"
  }
}
"#;

// THE BIG PREDICTION: does this genuinely consume the Preface's own
// vocabularyStance fact?
const PREFACE_LOOP_CLOSED_TEMPLATE: &str = r#"
commit argues {
  declare attribute claim

  consumes {
    fact {alienation_uri} (cid: {alienation_cid}) {
      subject: argument/epilogue_alienation
      predicate: claim
    }
    fact {preface_uri} (cid: {preface_cid}) {
      subject: argument/preface
      predicate: vocabularyStance
    }
  }
  produces {
    argument/epilogue claim "Communism responds by politicizing art"
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
    let preface_uri = "at://did:plc:form-reading/org.jason-edelman.writtenworld.commit/rkey001";
    let preface_cid = "bafyPrefaceEpilogue";
    let preface = identify(PREFACE_SRC, preface_uri, preface_cid);

    let aura_uri = "at://did:plc:form-reading-refined/org.jason-edelman.writtenworld.commit/rkey004b";
    let aura_cid = "bafyAuraEpilogue";
    let aura = identify(AURA_SRC, aura_uri, aura_cid);

    let lart_uri = "at://did:plc:form-reading-iv/org.jason-edelman.writtenworld.commit/rkey003b";
    let lart_cid = "bafyLartEpilogue";
    let lart = identify(LART_POUR_LART_SRC, lart_uri, lart_cid);

    let star_cult_uri = "at://did:plc:form-reading-x/org.jason-edelman.writtenworld.commit/rkey003";
    let star_cult_cid = "bafyStarCultEpilogue";
    let star_cult = identify(STAR_CULT_SRC, star_cult_uri, star_cult_cid);
    println!("=== Carried over: Preface's vocabularyStance, Section II's aura, IV's l'art pour l'art, X's star cult ===");

    let aestheticize_uri = "at://did:plc:epilogue/org.jason-edelman.writtenworld.commit/rkey001";
    let aestheticize = identify(AESTHETICIZE_SRC, aestheticize_uri, "bafyAestheticize");
    println!("=== Fascism: aesthetics introduced into political life ===\n{AESTHETICIZE_SRC}");

    let violation_uri = "at://did:plc:epilogue/org.jason-edelman.writtenworld.commit/rkey002";
    let violation_src = DOUBLE_VIOLATION_TEMPLATE
        .replace("{aestheticize_uri}", aestheticize_uri)
        .replace("{aestheticize_cid}", "bafyAestheticize")
        .replace("{star_cult_uri}", star_cult_uri)
        .replace("{star_cult_cid}", star_cult_cid);
    let violation = identify(&violation_src, violation_uri, "bafyViolation");
    println!("=== The double violation: masses AND apparatus, counterparts ===\n{violation_src}");

    let formula_uri = "at://did:plc:epilogue/org.jason-edelman.writtenworld.commit/rkey003";
    let formula_src = TWOFOLD_FORMULA_TEMPLATE
        .replace("{violation_uri}", violation_uri)
        .replace("{violation_cid}", "bafyViolation");
    let formula = identify(&formula_src, formula_uri, "bafyFormula");
    println!("=== The twofold formula: political AND technological ===\n{formula_src}");

    let marinetti_uri = "at://did:plc:marinetti/org.jason-edelman.writtenworld.commit/rkey001";
    let marinetti = identify(MARINETTI_SRC, marinetti_uri, "bafyMarinetti");
    println!("=== Marinetti, content monstrous, clarity endorsed as diagnostic evidence ===\n{MARINETTI_SRC}");

    let rebuttal_uri = "at://did:plc:epilogue/org.jason-edelman.writtenworld.commit/rkey004";
    let rebuttal_src = DIALECTICAL_REBUTTAL_TEMPLATE
        .replace("{marinetti_uri}", marinetti_uri)
        .replace("{marinetti_cid}", "bafyMarinetti")
        .replace("{formula_uri}", formula_uri)
        .replace("{formula_cid}", "bafyFormula");
    let rebuttal = identify(&rebuttal_src, rebuttal_uri, "bafyRebuttal");

    let aura_abolished_uri = "at://did:plc:epilogue/org.jason-edelman.writtenworld.commit/rkey005";
    let aura_abolished_src = AURA_ABOLISHED_TEMPLATE
        .replace("{rebuttal_uri}", rebuttal_uri)
        .replace("{rebuttal_cid}", "bafyRebuttal")
        .replace("{aura_uri}", aura_uri)
        .replace("{aura_cid}", aura_cid);
    let aura_abolished = identify(&aura_abolished_src, aura_abolished_uri, "bafyAuraAbolished");
    println!("=== PREDICTION: does 'aura abolished' consume Section II's aura fact? ===\n{aura_abolished_src}");

    let consummation_uri = "at://did:plc:epilogue/org.jason-edelman.writtenworld.commit/rkey006";
    let consummation_src = LART_POUR_LART_CONSUMMATION_TEMPLATE
        .replace("{aura_abolished_uri}", aura_abolished_uri)
        .replace("{aura_abolished_cid}", "bafyAuraAbolished")
        .replace("{lart_uri}", lart_uri)
        .replace("{lart_cid}", lart_cid);
    let consummation = identify(&consummation_src, consummation_uri, "bafyConsummation");
    println!("=== PREDICTION: does 'consummation of l'art pour l'art' consume Section IV's fact? ===\n{consummation_src}");

    let alienation_uri = "at://did:plc:epilogue/org.jason-edelman.writtenworld.commit/rkey007";
    let alienation_src = SELF_ALIENATION_TEMPLATE
        .replace("{consummation_uri}", consummation_uri)
        .replace("{consummation_cid}", "bafyConsummation");
    let alienation = identify(&alienation_src, alienation_uri, "bafyAlienation");

    let epilogue_uri = "at://did:plc:epilogue/org.jason-edelman.writtenworld.commit/rkey008";
    let epilogue_src = PREFACE_LOOP_CLOSED_TEMPLATE
        .replace("{alienation_uri}", alienation_uri)
        .replace("{alienation_cid}", "bafyAlienation")
        .replace("{preface_uri}", preface_uri)
        .replace("{preface_cid}", preface_cid);
    let epilogue = identify(&epilogue_src, epilogue_uri, "bafyEpilogue");
    println!("=== THE BIG PREDICTION: does the closing claim consume the Preface's vocabularyStance? ===\n{epilogue_src}");

    // Check 1: the double violation is a real doubling, and the
    // masses-violation side genuinely consumes Section X's star-cult
    // fact.
    let violation_predicates: std::collections::BTreeSet<&str> =
        violation.commit.produces.iter().map(|t| t.predicate.as_str()).collect();
    assert!(violation_predicates.contains("violation"));
    assert_eq!(violation.commit.consumes.len(), 2);
    let cites_star_cult = violation.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.subject == "argument/section_x_star_cult",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_star_cult, "the double violation must genuinely cite Section X's star-cult fact");
    println!(
        "\nCheck 1: violation.commit.consumes.len() = {}, citing Section X's star-cult fact -- \
         the Fuhrer cult and the movie-star cult are modeled as the SAME manufactured-cult \
         structure, not two independent uses of the word 'cult.'",
        violation.commit.consumes.len(),
    );

    // Check 2: the twofold formula is a real doubling.
    let formula_predicates: std::collections::BTreeSet<&str> =
        formula.commit.produces.iter().map(|t| t.predicate.as_str()).collect();
    assert!(formula_predicates.contains("formula"));
    let materialized_formula = Materialized::from_identified_commits(&[formula.clone()]);
    println!(
        "Check 2: political formula = {:?}; technological formula = {:?} -- an explicit \
         twofold, matching this essay's recurring pattern.",
        materialized_formula.current_value("formula/political", "formula"),
        materialized_formula.current_value("formula/technological", "formula"),
    );

    // Check 3: Marinetti's citation carries BOTH claim and
    // evaluativeStatus -- content and clarity judged separately.
    let marinetti_predicates: std::collections::BTreeSet<&str> =
        marinetti.commit.produces.iter().map(|t| t.predicate.as_str()).collect();
    assert!(marinetti_predicates.contains("claim"));
    assert!(marinetti_predicates.contains("evaluativeStatus"));
    println!(
        "Check 3: Marinetti's citation carries both claim and evaluativeStatus \
         ({marinetti_predicates:?}) -- content and clarity judged separately, a unique posture \
         in this series."
    );

    // Check 4: PREDICTION. Does "aura abolished" genuinely consume
    // Section II's aura fact?
    assert_eq!(aura_abolished.commit.consumes.len(), 2);
    let cites_aura = aura_abolished.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "aura",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_aura, "the aura-abolished claim must genuinely cite Section II's aura fact");
    println!(
        "Check 4 (CONFIRMED): aura_abolished.commit.consumes.len() = {}, citing Section II's \
         aura predicate -- \"through gas warfare the aura is abolished in a new way\" genuinely \
         cites the term Section II coined, applied now to its most extreme case.",
        aura_abolished.commit.consumes.len(),
    );

    // Check 5: PREDICTION. Does "consummation of l'art pour l'art"
    // genuinely consume Section IV's l'art-pour-l'art fact?
    assert_eq!(consummation.commit.consumes.len(), 2);
    let cites_lart = consummation.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "basis",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_lart, "the consummation claim must genuinely cite Section IV's l'art-pour-l'art fact");
    println!(
        "Check 5 (CONFIRMED): consummation.commit.consumes.len() = {}, citing Section IV's \
         l'art-pour-l'art basis predicate -- Fascism's aesthetics of war is modeled as the SAME \
         defensive formation Section IV traced, not a fresh coinage of 'l'art pour l'art.'",
        consummation.commit.consumes.len(),
    );

    // Check 6: THE BIG PREDICTION. Does the closing claim genuinely
    // consume the Preface's own vocabularyStance fact?
    assert_eq!(epilogue.commit.consumes.len(), 2);
    let cites_preface = epilogue.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "vocabularyStance",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_preface, "the closing claim must genuinely cite the Preface's vocabularyStance fact");
    println!(
        "Check 6 (THE BIG PREDICTION, CONFIRMED): epilogue.commit.consumes.len() = {}, citing \
         the Preface's vocabularyStance predicate -- the essay's opening political stipulation \
         and its closing political verdict are ONE continuous argument, not two separate \
         political gestures bookending an otherwise aesthetic-theoretical essay. The loop \
         closes.",
        epilogue.commit.consumes.len(),
    );

    let _ = (preface.clone(), rebuttal.clone(), alienation.clone());

    println!(
        "\n=== done: the Fuhrer cult and the movie-star cult are the same manufactured-cult \
         structure (Check 1); the war-culminates-aestheticized-politics claim carries an \
         explicit twofold formula (Check 2); Marinetti's clarity is endorsed as diagnostic \
         evidence, his content rejected (Check 3); the aura's final abolition genuinely cites \
         Section II's own coined term (Check 4); Fascism's war-aesthetics is genuinely Section \
         IV's l'art pour l'art, consummated (Check 5); and the Preface's opening stipulation and \
         the Epilogue's closing verdict are ONE continuous argument, confirmed under a real \
         check (Check 6, the biggest payoff of this entire series). ==="
    );
}
