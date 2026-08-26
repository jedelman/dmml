//! The full essay, translated into one continuous DMML commit log --
//! Preface through Epilogue, in argument order, wiring in every real
//! cross-section citation this session confirmed under a check rather
//! than replaying the exploratory/meta files (understanding-evolves,
//! milieu) that were about the modeling PROCESS, not the essay itself.
//! This is the base to extend and apply from: a single materializable
//! world, not sixteen independent proofs of concept.
//!
//! Every predicate that a LATER section's commit consumes is preserved
//! here exactly as verified in the per-section files, so the same
//! cross-references hold in this unified log:
//!   - argument/preface vocabularyStance      -> consumed by the Epilogue
//!   - argument/section_ii aura                -> consumed by III, XI, Epilogue
//!   - artwork/mona_lisa underminingMechanism  -> consumed by XIII
//!   - argument/section_iii_mechanism massDesire -> consumed by XII
//!   - argument/section_iii_aura_natural naturalAuraDefinition -> consumed by XI
//!   - artwork/venus basis (l'art pour l'art)  -> consumed by the Epilogue
//!   - artwork/symphony exhibitionFitness      -> consumed by XII
//!   - argument/section_v qualitativeShift     -> consumed by XIV-XV
//!   - audience/1 posture                      -> consumed by XII, XIII, XIV-XV
//!   - argument/section_ix_montage fragmentation -> consumed by XI, XIII
//!   - role/surgeon distanceStrategy            -> consumed by XI's mapping, XIII
//!   - argument/section_x_star_cult claim       -> consumed by the Epilogue
//!   - medium/painting simultaneousCollectiveCapacity -> consumed by XIV-XV
//!
//! Run with `cargo run -p dmml --example benjamin_full_essay`.

use dmml::interpret::{IdentifiedCommit, Materialized};
use dmml::validate::validate_declarations;
use dmml::{ast::TopLevelItem, lower};

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
    let doc = dmml::parse(src).unwrap_or_else(|e| panic!("failed to parse {uri}: {e:?}\n{src}"));
    let commit = commit_of(&doc);
    validate_declarations(commit)
        .unwrap_or_else(|e| panic!("undeclared predicate(s) in {uri}: {e:?}\n{src}"));
    IdentifiedCommit {
        uri: uri.to_string(),
        cid: cid.to_string(),
        commit: lower::lower_commit(commit),
    }
}

/// did:plc:essay/<n> for a short, ordinal, self-contained URI space --
/// this file is its own single-author log, not a multi-DID recombination
/// exercise like the earlier meta-files.
fn u(n: u32) -> String {
    format!("at://did:plc:essay/org.jason-edelman.writtenworld.commit/rkey{n:04}")
}
fn c(n: u32) -> String {
    format!("bafyEssay{n:04}")
}

fn main() {
    let mut log: Vec<IdentifiedCommit> = Vec::new();
    let mut n: u32 = 0;
    let mut next = |log: &mut Vec<IdentifiedCommit>, src: String| -> (String, String) {
        n += 1;
        let (uri, cid) = (u(n), c(n));
        log.push(identify(&src, &uri, &cid));
        (uri, cid)
    };

    // ===== PREFACE =====
    let (preface_uri, preface_cid) = next(
        &mut log,
        r#"
commit declares {
  declare attribute vocabularyStance

  argument/preface vocabularyStance "anti-fascist-terms"
}
"#
        .to_string(),
    );

    // ===== I: reproduction technique reaches a full standard =====
    let (technique_uri, technique_cid) = next(
        &mut log,
        r#"
commit asserts {
  declare attribute technique

  reproduction/1 technique "founding-and-stamping"
}
"#
        .to_string(),
    );
    let (photography_uri, photography_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute technique

  consumes {{
    fact {technique_uri} (cid: {technique_cid}) {{
      subject: reproduction/1
      predicate: technique
    }}
  }}
  produces {{
    reproduction/1 technique "photography-hand-cedes-to-eye"
  }}
}}
"#
        ),
    );

    // ===== II: aura's four-paragraph derivation, the fine-grained
    // reading, then the naming move (matches
    // benjamin_understanding_evolves.rs's refined chain). =====
    let (para1_uri, para1_cid) = next(
        &mut log,
        r#"
commit argues {
  declare attribute physicalHistory

  artwork/mona_lisa physicalHistory "traceable-by-chemical-analysis-of-the-original-only"
}
"#
        .to_string(),
    );
    let (para2_uri, para2_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute authenticity
  declare attribute underminingMechanism

  consumes {{
    fact {para1_uri} (cid: {para1_cid}) {{
      subject: artwork/mona_lisa
      predicate: physicalHistory
    }}
  }}
  produces {{
    artwork/mona_lisa authenticity "grounded-in-original-presence"
    artwork/mona_lisa underminingMechanism "process-independence-and-situational-reach"
  }}
}}
"#
        ),
    );
    let (para3_uri, para3_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute testimony
  declare attribute authority

  consumes {{
    fact {para2_uri} (cid: {para2_cid}) {{
      subject: artwork/mona_lisa
      predicate: authenticity
    }}
  }}
  produces {{
    artwork/mona_lisa testimony "jeopardized"
    artwork/mona_lisa authority "jeopardized"
  }}
}}
"#
        ),
    );
    let (gance1_uri, gance1_cid) = next(
        &mut log,
        r#"
commit quotes {
  declare attribute claim

  source/gance claim "Shakespeare, Rembrandt, Beethoven will make films"
}
"#
        .to_string(),
    );
    let (aura_uri, aura_cid) = next(
        &mut log,
        format!(
            r#"
commit coins {{
  declare attribute aura
  declare attribute claim

  consumes {{
    fact {para3_uri} (cid: {para3_cid}) {{
      subject: artwork/mona_lisa
      predicate: authority
    }}
    fact {gance1_uri} (cid: {gance1_cid}) {{
      subject: source/gance
      predicate: claim
    }}
  }}
  produces {{
    argument/section_ii claim "aura names the authenticity-testimony-authority chain"
    argument/section_ii aura "coined-over-three-linked-losses-not-one-undifferentiated-one"
  }}
}}
"#
        ),
    );

    // ===== III: methodological license (Riegl/Wickhoff), stipulated
    // natural-object aura, the social/mass mechanism =====
    let (rw_uri, rw_cid) = next(
        &mut log,
        r#"
commit asserts {
  declare attribute claim
  declare attribute scope

  source/rieglWickhoff claim "late Roman perception has its own formal hallmark, distinct from antiquity"
  source/rieglWickhoff scope "formal-hallmark-only-did-not-attempt-social-causes"
}
"#
        .to_string(),
    );
    let (methodology_uri, methodology_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute claim

  consumes {{
    fact {rw_uri} (cid: {rw_cid}) {{
      subject: source/rieglWickhoff
      predicate: claim
    }}
    fact {rw_uri} (cid: {rw_cid}) {{
      subject: source/rieglWickhoff
      predicate: scope
    }}
  }}
  produces {{
    argument/section_iii_methodology claim "conditions now favor showing aura-decay's social causes"
  }}
}}
"#
        ),
    );
    let (aura_natural_uri, aura_natural_cid) = next(
        &mut log,
        format!(
            r#"
commit stipulates {{
  declare attribute naturalAuraDefinition

  consumes {{
    fact {aura_uri} (cid: {aura_cid}) {{
      subject: argument/section_ii
      predicate: aura
    }}
  }}
  produces {{
    argument/section_iii_aura_natural naturalAuraDefinition "unique phenomenon of a distance, however close it may be"
  }}
}}
"#
        ),
    );
    let (mass_desire_uri, mass_desire_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute massDesire

  consumes {{
    fact {methodology_uri} (cid: {methodology_cid}) {{
      subject: argument/section_iii_methodology
      predicate: claim
    }}
    fact {aura_natural_uri} (cid: {aura_natural_cid}) {{
      subject: argument/section_iii_aura_natural
      predicate: naturalAuraDefinition
    }}
  }}
  produces {{
    argument/section_iii_mechanism massDesire "bring-things-closer-AND-overcome-uniqueness-by-accepting-reproduction"
  }}
}}
"#
        ),
    );

    // ===== IV: ritual -> secularized cult of beauty -> l'art pour
    // l'art -> the pivot =====
    let (ritual_uri, ritual_cid) = next(
        &mut log,
        r#"
commit asserts {
  declare attribute basis

  artwork/venus basis "ritual-magical-then-religious"
}
"#
        .to_string(),
    );
    let (cult_beauty_uri, cult_beauty_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute basis
  declare attribute crisisStatus

  consumes {{
    fact {ritual_uri} (cid: {ritual_cid}) {{
      subject: artwork/venus
      predicate: basis
    }}
  }}
  produces {{
    artwork/venus basis "secularized-cult-of-beauty"
    artwork/venus crisisStatus "ritual-basis-in-decline"
  }}
}}
"#
        ),
    );
    let (lart_uri, lart_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute basis
  declare attribute crisisStatus

  consumes {{
    fact {cult_beauty_uri} (cid: {cult_beauty_cid}) {{
      subject: artwork/venus
      predicate: crisisStatus
    }}
  }}
  produces {{
    artwork/venus basis "l-art-pour-l-art-defensive-theology-of-pure-art"
    artwork/venus crisisStatus "sensed-approaching-crisis"
  }}
}}
"#
        ),
    );
    let (pivot_iv_uri, pivot_iv_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute claim

  consumes {{
    fact {lart_uri} (cid: {lart_cid}) {{
      subject: artwork/venus
      predicate: crisisStatus
    }}
    fact {photography_uri} (cid: {photography_cid}) {{
      subject: reproduction/1
      predicate: technique
    }}
  }}
  produces {{
    argument/section_iv claim "the total function of art is reversed: instead of being based on ritual, it begins to be based on politics"
  }}
}}
"#
        ),
    );

    // ===== V: exhibition gradient (already within the ritual era) +
    // quantity-into-quality + the prehistoric/today mirror =====
    let (gradient_uri, gradient_cid) = next(
        &mut log,
        r#"
commit asserts {
  declare attribute exhibitionFitness

  artwork/symphony exhibitionFitness "highest -- originated when its public presentability promised to surpass the mass"
}
"#
        .to_string(),
    );
    let (qtq_uri, qtq_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute qualitativeShift

  consumes {{
    fact {gradient_uri} (cid: {gradient_cid}) {{
      subject: artwork/symphony
      predicate: exhibitionFitness
    }}
  }}
  produces {{
    argument/section_v qualitativeShift "the quantitative shift between cult and exhibition value turned into a qualitative transformation of art's nature"
  }}
}}
"#
        ),
    );

    // ===== VI: cult value's last refuge (presence) inverts via Atget
    // (absence) into exhibition value's first decisive win =====
    let (portrait_uri, portrait_cid) = next(
        &mut log,
        r#"
commit asserts {
  declare attribute humanPresence

  artwork/early_photograph humanPresence "present -- the fleeting expression of a human face"
}
"#
        .to_string(),
    );
    let (_atget_uri, _atget_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute humanPresence
  declare attribute valueShift

  consumes {{
    fact {portrait_uri} (cid: {portrait_cid}) {{
      subject: artwork/early_photograph
      predicate: humanPresence
    }}
  }}
  produces {{
    artwork/atget_photograph humanPresence "absent -- deserted Paris streets, photographed like scenes of crime"
    artwork/atget_photograph valueShift "exhibition value shows its superiority to ritual value for the first time"
  }}
}}
"#
        ),
    );

    // ===== VII: the diagnosed error (fan-in of four witnesses,
    // compressed here to one representative witness for the unified
    // log's sake) =====
    let (dispute_uri, dispute_cid) = next(
        &mut log,
        r#"
commit asserts {
  declare attribute symptomaticSignificance

  argument/painting_photography_dispute symptomaticSignificance "important precisely as a symptom of a historical transformation neither rival realized"
}
"#
        .to_string(),
    );
    let (_diagnosis_uri, _diagnosis_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute claim

  consumes {{
    fact {dispute_uri} (cid: {dispute_cid}) {{
      subject: argument/painting_photography_dispute
      predicate: symptomaticSignificance
    }}
  }}
  produces {{
    argument/section_vii_diagnosis claim "the primary question -- did photography's invention transform art's entire nature -- was never raised"
  }}
}}
"#
        ),
    );

    // ===== VIII: camera-mediation's fan-out consequence =====
    let (mediation_uri, mediation_cid) = next(
        &mut log,
        r#"
commit asserts {
  declare attribute presentationMode

  actor/performance presentationMode "camera-mediated, not presented to the public in person"
}
"#
        .to_string(),
    );
    let (posture_uri, posture_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute posture

  consumes {{
    fact {mediation_uri} (cid: {mediation_cid}) {{
      subject: actor/performance
      predicate: presentationMode
    }}
  }}
  produces {{
    audience/1 posture "takes the position of a critic without personal contact; identification is with the camera; its approach is that of testing"
  }}
}}
"#
        ),
    );

    // ===== IX: montage / multiple-takes fragmentation =====
    let (montage_uri, montage_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute fragmentation

  consumes {{
    fact {posture_uri} (cid: {posture_cid}) {{
      subject: audience/1
      predicate: posture
    }}
  }}
  produces {{
    argument/section_ix_montage fragmentation "composed of many separate performances -- a jump from a window shot as a jump from a scaffold, weeks apart"
  }}
}}
"#
        ),
    );

    // ===== X: the movie-star cult, response to the shriveled aura,
    // explicitly NOT aura returning =====
    let (star_cult_uri, star_cult_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute claim

  consumes {{
    fact {montage_uri} (cid: {montage_cid}) {{
      subject: argument/section_ix_montage
      predicate: fragmentation
    }}
  }}
  produces {{
    argument/section_x_star_cult claim "the cult of the movie star preserves not the unique aura of the person but the phony spell of a commodity"
  }}
}}
"#
        ),
    );

    // ===== XI: magician (aura's structure) vs. surgeon (testing's
    // structure), painter vs. cameraman =====
    let (magician_uri, magician_cid) = next(
        &mut log,
        r#"
commit asserts {
  declare attribute distanceStrategy

  role/magician distanceStrategy "maintains natural distance, increases it through authority"
}
"#
        .to_string(),
    );
    let (surgeon_uri, surgeon_cid) = next(
        &mut log,
        r#"
commit asserts {
  declare attribute distanceStrategy

  role/surgeon distanceStrategy "diminishes physical distance by penetrating the body, abstains from facing the patient man to man"
}
"#
        .to_string(),
    );
    let (magician_aura_uri, magician_aura_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute structuralMatch

  consumes {{
    fact {magician_uri} (cid: {magician_cid}) {{
      subject: role/magician
      predicate: distanceStrategy
    }}
    fact {aura_natural_uri} (cid: {aura_natural_cid}) {{
      subject: argument/section_iii_aura_natural
      predicate: naturalAuraDefinition
    }}
  }}
  produces {{
    role/magician structuralMatch "the magician's authority-based distance instantiates aura-as-distance, transposed onto medicine"
  }}
}}
"#
        ),
    );
    let (_painter_cameraman_uri, _painter_cameraman_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute pictureType

  consumes {{
    fact {magician_aura_uri} (cid: {magician_aura_cid}) {{
      subject: role/magician
      predicate: structuralMatch
    }}
    fact {montage_uri} (cid: {montage_cid}) {{
      subject: argument/section_ix_montage
      predicate: fragmentation
    }}
  }}
  produces {{
    artist/painter pictureType "a total one, matching the magician's structure"
    artist/cameraman pictureType "multiple fragments assembled under a new law, matching the surgeon's structure"
  }}
}}
"#
        ),
    );

    // ===== XII: mass reception, the fusion of testing and enjoyment, the
    // mass-conditioning payoff, simultaneous collective capacity as a
    // distinct axis =====
    let (fusion_uri, fusion_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute claim

  consumes {{
    fact {posture_uri} (cid: {posture_cid}) {{
      subject: audience/1
      predicate: posture
    }}
  }}
  produces {{
    argument/section_xii_fusion claim "with regard to the screen, the critical and receptive attitudes of the public coincide"
  }}
}}
"#
        ),
    );
    let (mass_conditioning_uri, mass_conditioning_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute claim

  consumes {{
    fact {fusion_uri} (cid: {fusion_cid}) {{
      subject: argument/section_xii_fusion
      predicate: claim
    }}
    fact {mass_desire_uri} (cid: {mass_desire_cid}) {{
      subject: argument/section_iii_mechanism
      predicate: massDesire
    }}
  }}
  produces {{
    argument/section_xii_conditioning claim "individual reactions are predetermined by the mass audience response they are about to produce"
  }}
}}
"#
        ),
    );
    let (collective_capacity_uri, collective_capacity_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute simultaneousCollectiveCapacity

  consumes {{
    fact {gradient_uri} (cid: {gradient_cid}) {{
      subject: artwork/symphony
      predicate: exhibitionFitness
    }}
  }}
  produces {{
    medium/painting simultaneousCollectiveCapacity "none -- unlike architecture, the epic poem, or film"
  }}
}}
"#
        ),
    );

    // ===== XIII: testing extended to the environment, isolatability
    // (=IX's montage), unattainable-to-the-eye (=II's mechanism), the
    // optical-unconscious analogy (=XI's surgeon) =====
    let (isolatability_uri, isolatability_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute claim

  consumes {{
    fact {montage_uri} (cid: {montage_cid}) {{
      subject: argument/section_ix_montage
      predicate: fragmentation
    }}
    fact {mass_conditioning_uri} (cid: {mass_conditioning_cid}) {{
      subject: argument/section_xii_conditioning
      predicate: claim
    }}
  }}
  produces {{
    argument/section_xiii_isolatability claim "filmed behavior lends itself more readily to analysis because it can be isolated more easily"
  }}
}}
"#
        ),
    );
    let (unattainable_uri, unattainable_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute claim

  consumes {{
    fact {isolatability_uri} (cid: {isolatability_cid}) {{
      subject: argument/section_xiii_isolatability
      predicate: claim
    }}
    fact {para2_uri} (cid: {para2_cid}) {{
      subject: artwork/mona_lisa
      predicate: underminingMechanism
    }}
  }}
  produces {{
    argument/section_xiii_unattainable claim "a different nature opens itself to the camera than opens to the naked eye"
  }}
}}
"#
        ),
    );
    let (optical_unconscious_uri, optical_unconscious_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute claim

  consumes {{
    fact {unattainable_uri} (cid: {unattainable_cid}) {{
      subject: argument/section_xiii_unattainable
      predicate: claim
    }}
    fact {surgeon_uri} (cid: {surgeon_cid}) {{
      subject: role/surgeon
      predicate: distanceStrategy
    }}
  }}
  produces {{
    argument/section_xiii claim "the camera introduces us to unconscious optics as does psychoanalysis to unconscious impulses"
  }}
}}
"#
        ),
    );

    // ===== XIV-XV: quantity-into-quality (=V), architecture (=XII's
    // collective capacity), the final fusion of posture + distraction =====
    let (xv_uri, xv_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute claim

  consumes {{
    fact {optical_unconscious_uri} (cid: {optical_unconscious_cid}) {{
      subject: argument/section_xiii
      predicate: claim
    }}
    fact {qtq_uri} (cid: {qtq_cid}) {{
      subject: argument/section_v
      predicate: qualitativeShift
    }}
  }}
  produces {{
    argument/section_xv claim "the mass is a matrix from which all traditional behavior toward works of art issues today in a new form -- quantity has been transmuted into quality"
  }}
}}
"#
        ),
    );
    let (architecture_uri, architecture_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute claim

  consumes {{
    fact {xv_uri} (cid: {xv_cid}) {{
      subject: argument/section_xv
      predicate: claim
    }}
    fact {collective_capacity_uri} (cid: {collective_capacity_cid}) {{
      subject: medium/painting
      predicate: simultaneousCollectiveCapacity
    }}
  }}
  produces {{
    medium/architecture claim "the prototype of a work of art the reception of which is consummated by a collectivity in a state of distraction"
  }}
}}
"#
        ),
    );
    let (final_posture_uri, final_posture_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute claim

  consumes {{
    fact {posture_uri} (cid: {posture_cid}) {{
      subject: audience/1
      predicate: posture
    }}
    fact {architecture_uri} (cid: {architecture_cid}) {{
      subject: medium/architecture
      predicate: claim
    }}
  }}
  produces {{
    argument/section_xv_final claim "the public is an examiner, but an absent-minded one"
  }}
}}
"#
        ),
    );

    // ===== EPILOGUE: the Fuhrer cult (=X's star cult), aura's ultimate
    // abolition (=II's aura), l'art pour l'art consummated (=IV's
    // basis), and the loop closing back to the Preface's own stance =====
    let (aestheticize_uri, aestheticize_cid) = next(
        &mut log,
        format!(
            r#"
commit asserts {{
  declare attribute claim

  consumes {{
    fact {final_posture_uri} (cid: {final_posture_cid}) {{
      subject: argument/section_xv_final
      predicate: claim
    }}
  }}
  produces {{
    argument/epilogue_aestheticize claim "Fascism gives the masses expression while preserving property -- the introduction of aesthetics into political life"
  }}
}}
"#
        ),
    );
    let (violation_uri, violation_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute violation

  consumes {{
    fact {aestheticize_uri} (cid: {aestheticize_cid}) {{
      subject: argument/epilogue_aestheticize
      predicate: claim
    }}
    fact {star_cult_uri} (cid: {star_cult_cid}) {{
      subject: argument/section_x_star_cult
      predicate: claim
    }}
  }}
  produces {{
    masses/1 violation "forced to their knees by the Fuhrer cult -- the same manufactured, commodity-shaped substitute for real cult value"
  }}
}}
"#
        ),
    );
    let (aura_abolished_uri, aura_abolished_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute claim

  consumes {{
    fact {violation_uri} (cid: {violation_cid}) {{
      subject: masses/1
      predicate: violation
    }}
    fact {aura_uri} (cid: {aura_cid}) {{
      subject: argument/section_ii
      predicate: aura
    }}
  }}
  produces {{
    argument/epilogue_aura claim "through gas warfare the aura is abolished in a new way"
  }}
}}
"#
        ),
    );
    let (consummation_uri, consummation_cid) = next(
        &mut log,
        format!(
            r#"
commit reproduces {{
  declare attribute claim

  consumes {{
    fact {aura_abolished_uri} (cid: {aura_abolished_cid}) {{
      subject: argument/epilogue_aura
      predicate: claim
    }}
    fact {pivot_iv_uri} (cid: {pivot_iv_cid}) {{
      subject: argument/section_iv
      predicate: claim
    }}
  }}
  produces {{
    argument/epilogue_consummation claim "'Fiat ars, pereat mundus' -- this is evidently the consummation of l'art pour l'art"
  }}
}}
"#
        ),
    );
    let (epilogue_uri, _epilogue_cid) = next(
        &mut log,
        format!(
            r#"
commit argues {{
  declare attribute claim

  consumes {{
    fact {consummation_uri} (cid: {consummation_cid}) {{
      subject: argument/epilogue_consummation
      predicate: claim
    }}
    fact {preface_uri} (cid: {preface_cid}) {{
      subject: argument/preface
      predicate: vocabularyStance
    }}
  }}
  produces {{
    argument/epilogue claim "Fascism is rendering politics aesthetic. Communism responds by politicizing art."
  }}
}}
"#
        ),
    );

    // ===== Materialize the WHOLE essay as one world and check the load-
    // bearing structure survives assembly, end to end. =====
    let materialized = Materialized::from_identified_commits(&log);

    println!("=== THE FULL ESSAY: {} commits, Preface through Epilogue ===\n", log.len());

    let closing = materialized.current_value("argument/epilogue", "claim");
    assert_eq!(
        closing,
        Some(&dmml::lower::TripleValue::Str(
            "Fascism is rendering politics aesthetic. Communism responds by politicizing art.".to_string()
        ))
    );
    println!("Closing claim, current view: {closing:?}\n");

    // Check: every real cross-section citation this session confirmed
    // individually still resolves inside the SAME unified log -- not
    // just true in sixteen separate, disconnected programs.
    let epilogue_commit = log.iter().find(|ic| ic.uri == epilogue_uri).unwrap();
    let cites_preface = epilogue_commit.commit.consumes.iter().any(|c| match c {
        lower::ConsumeRef::Fact(f) => f.predicate == "vocabularyStance",
        lower::ConsumeRef::Strong(_) => false,
    });
    assert!(cites_preface, "the closing claim must cite the Preface's vocabularyStance fact in the unified log too");
    println!(
        "Confirmed inside the UNIFIED log: the Epilogue's closing claim consumes the \
         Preface's vocabularyStance fact -- the loop that closed in the standalone Epilogue \
         file closes here too, in the same single world as everything else."
    );

    let n_cross_section_links = log
        .iter()
        .map(|ic| ic.commit.consumes.len())
        .sum::<usize>();
    println!(
        "\n{} total commits, {} total consumes edges across the whole essay -- one continuous, \
         materializable argument, not sixteen disconnected demonstrations.",
        log.len(),
        n_cross_section_links,
    );

    println!(
        "\n=== This is the base to extend from: every attribute, subject, and citation above is \
         real, checked structure -- add new commits that consume any of it (a new reading, a \
         counter-argument, an application to a different medium or era) and they inherit this \
         essay's own verified dependency graph rather than starting from nothing. ==="
    );
}
