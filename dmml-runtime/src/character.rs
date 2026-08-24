//! Room character: mapping the graded ground facts (dampness, decay,
//! light) to a single dominant trait and its wording, shared between
//! `demiurge.rs` (which needs an adjective at generation time, baked into
//! the room's proper name) and `render.rs` (which needs a clause at every
//! render, composing the description). One shared vocabulary reading the
//! same facts, not two independently rolled ones -- previously the name's
//! adjective and the description's flavor detail were drawn from separate
//! word banks with nothing keeping them consistent (an "ash-grey" room
//! could still get "water drips" as its detail sentence). Deriving both
//! from the same stored facts makes that incoherence structurally
//! impossible rather than something to remember to avoid.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trait {
    Plain,
    Damp,
    Decayed,
    Dark,
}

/// Whichever of dampness/decay/darkness is most pronounced, provided it
/// clears the "worth mentioning" threshold. Most rooms should read as
/// unremarkable, not maximally eventful on every axis at once -- a room
/// only gets a dominant character when one fact actually stands out.
pub fn dominant_trait(dampness: f32, decay: f32, light: f32) -> (Trait, f32) {
    let dark = 1.0 - light;
    let candidates = [
        (Trait::Damp, dampness),
        (Trait::Decayed, decay),
        (Trait::Dark, dark),
    ];
    let (t, v) =
        candidates.into_iter().fold(
            (Trait::Plain, 0.0f32),
            |acc, c| if c.1 > acc.1 { c } else { acc },
        );
    if v > 0.34 {
        (t, v)
    } else {
        (Trait::Plain, v)
    }
}

/// A single word, for baking into a generated room's proper name at
/// creation time (`demiurge.rs`).
pub fn adjective(t: Trait, intensity: f32) -> &'static str {
    let strong = intensity > 0.67;
    match (t, strong) {
        (Trait::Plain, _) => "bare",
        (Trait::Damp, true) => "dripping",
        (Trait::Damp, false) => "moss-slick",
        (Trait::Decayed, true) => "root-cracked",
        (Trait::Decayed, false) => "sootstained",
        (Trait::Dark, true) => "shadowed",
        (Trait::Dark, false) => "dim",
    }
}

/// A full sentence, for composing a room's description fresh on every
/// render (`render.rs`) rather than storing one.
pub fn clause(t: Trait, intensity: f32) -> &'static str {
    let strong = intensity > 0.67;
    match (t, strong) {
        (Trait::Plain, _) => "The stone here is unremarkable, holding steady.",
        (Trait::Damp, true) => {
            "Water drips somewhere close, and everything feels damp to the touch."
        }
        (Trait::Damp, false) => "A faint dampness clings to the air.",
        (Trait::Decayed, true) => {
            "The stone is crumbling here, cracked through by roots and years."
        }
        (Trait::Decayed, false) => "Old cracks run through one wall, too regular to be accidental.",
        (Trait::Dark, true) => {
            "It is close to entirely dark; you feel your way more than you see it."
        }
        (Trait::Dark, false) => "Shadows crowd the corners, thicker than they should be.",
    }
}
