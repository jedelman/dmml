//! Wired into `Game::fire_object_verb` (`game.rs`): the real, live
//! replacement for `build_effect_delta`'s per-machine imperative dispatch
//! AND `datalog_guard::machines_ready`'s gating, unified into ONE crepe
//! fixpoint for exactly the machines a single `fire_object_verb` call
//! considers. `verbs_available_on` (read-only, fires nothing) is
//! deliberately untouched -- it still uses `datalog_guard::machines_ready`
//! directly, which remains the authoritative gate for that path.
//!
//! First drafted by `z-ai/glm-5.3-flash` (dev-tooling dispatch pipeline,
//! see written-world's CLAUDE.md) as a standalone, not-wired-in spike;
//! extended by hand for the real cutover -- see "Extended for the real
//! cutover" below for exactly what changed and why, on top of the
//! original spike's own fixes (documented in its first commit).
//!
//! THE ACTUAL POINT, proven by `cascade_three_machines`: a machine's
//! fired effect can satisfy ANOTHER machine's requirement inside a single
//! `.run()`, with no imperative per-machine loop and no notion of a
//! "turn" as a primitive in the causality model. Concretely, the rule
//! chain is:
//!
//!   BaseAttr/AttrValue → AttrNow → ReqSatAttr → AllReqsMet(M2)
//!     ← NewAttrValue(M1) ← AllReqsMet(M1) ← NoRequirement(M1)
//!   AllReqsMet(M2) → NewEdgeLocked(M2) → EdgeNow → ReqSatEdge(M3)
//!     → AllReqsMet(M3) → NewAttrValue(M3)
//!
//! All of this is monotone Datalog (no negation anywhere in the cycle --
//! see "why multi-requirement isn't more negation" below), so crepe's
//! semi-naive evaluation derives the whole M1 → M2 → M3 chain in one
//! native `.run()`.
//!
//! Extended for the real cutover (none of this was in the original
//! spike, all found necessary by actually trying to wire it into
//! `fire_object_verb` rather than assumed):
//!
//! 1. **Multi-requirement support.** The spike's `EffectMachine` carried
//!    `requirement: Option<Requirement>` -- at most one. Real content
//!    needs two: `demiurge.rs`'s lever `threshold_delta` machine has
//!    `[EdgeLocked{equals:true}, AttrAtLeast{wear,threshold}]`. The
//!    spike's own doc comment already named the fix ("provenance-tagged
//!    satisfaction facts, not more negation") without building it, since
//!    it wasn't needed to prove the cascade. Built here as two fixed
//!    requirement *slots* (`ReqSlot1*`/`ReqSlot2*`, `NoReqSlot1`/
//!    `NoReqSlot2` for absent slots) -- bounded at 2 because that's the
//!    real, verified maximum across every `build_action_machine` call
//!    site in this workspace (0 for the frontier generator, 1 for the
//!    lever's drift machine, 2 for its threshold machine; grep confirms
//!    no third). `EffectMachine::new` panics if handed more than 2 --
//!    an explicit, loud stop if content ever needs a third slot, not a
//!    silently-dropped requirement. This is exactly the "smallest
//!    generic extension the failure actually demands" this workspace's
//!    own `SPEC.md` §18 razor asks for, not a general N-requirement
//!    system nothing yet needs.
//! 2. **Why multi-requirement isn't more negation.** The spike's doc
//!    comment was right that a *derived* "all requirements met" via
//!    `!UnmetReq(m)` can't work here: `AllReqsMet` feeds `NewAttrValue`/
//!    `NewEdgeLocked` feeds `AttrNow`/`EdgeNow` feeds the requirement
//!    check itself, so a negated relation would sit inside a genuine
//!    recursive cycle and crepe would (correctly) reject it as
//!    unstratifiable. The slot design sidesteps this the same way the
//!    original single-requirement version did: enumerate every
//!    combination explicitly (`Slot1Sat(m) <- NoReqSlot1(m)` /
//!    `<- ReqSlot1AttrAtLeast(...)` / `<- ReqSlot1EdgeLockedIs(...)` /
//!    `<- ReqSlot1PlayerInRoom(...)`, same for slot 2, then
//!    `AllReqsMet(m) <- Slot1Sat(m), Slot2Sat(m), Has*(m, ...)`) -- pure
//!    positive joins, no `!` anywhere in the cycle. Positive recursion
//!    through the cycle is exactly caveat 2 below and is fine; negation
//!    through it is not, and never appears.
//! 3. **`PlayerInRoom` support added.** The spike's `Requirement` enum
//!    only had `AttrAtLeast`/`EdgeLockedIs` -- two of `machine::
//!    Requirement`'s three variants. Since this module's gating now
//!    fully replaces `datalog_guard::machines_ready` for
//!    `fire_object_verb` specifically (not just supplements it), leaving
//!    a requirement kind unsupported would silently narrow what that
//!    call site can gate on, versus `datalog_guard.rs`'s exhaustive
//!    coverage. No real content currently equips a `PlayerInRoom`-gated
//!    object machine, but nothing rules one out, and `verbs_available_on`
//!    already accepts it as a general requirement kind -- so it's here
//!    for parity, not because a specific piece of content demanded it.
//! 4. **Real `vocab::locked()`, not a spike-local placeholder.** The
//!    original spike used `const LOCKED_PREDICATE: &str =
//!    "urn:dmml:pred:locked"` -- a plausible-looking IRI that is NOT
//!    what `apply_commit`/`render.rs`/everything else in this crate
//!    actually reads and writes for edge-lock state. Wiring it in with
//!    that placeholder unchanged would have silently read/written the
//!    wrong predicate against the real graph -- caught only by checking
//!    against `vocab.rs`, not by any test the spike itself ran (its own
//!    tests never touched a real graph's `locked` triples).
//! 5. **Per-machine attribution kept in the output.** The spike's
//!    `EffectFixpoint.attr_deltas`/`.edge_locks` discarded which machine
//!    produced each value (`for NewAttrValue(_m, node, attr, v) in
//!    new_attr`). `fire_object_verb` needs to commit one delta per
//!    firing machine (matching the pre-cutover behavior's commit
//!    granularity exactly, so transcript/commit-log structure doesn't
//!    silently change shape) and render one `describe_effect_outcome`
//!    message per machine in creation order. Both need the machine id
//!    back, not an aggregated (node, attr, value)/(edge, value) list.
//!
//! Known, accepted behavioral difference from the pre-cutover imperative
//! loop, flagged rather than hidden: if two machines fire in the SAME
//! `fire_object_verb` call and both `IncrementAttr` the exact same
//! `(node, attr)` pair, the old sequential loop would compound (each
//! machine's commit visible to the next machine's read), while this
//! fixpoint computes both from the SAME pre-turn `BaseAttr` (each
//! `NewAttrValue` is keyed by machine, so both values are real and both
//! get committed -- they just don't chain off each other). No real
//! content does this today (`demiurge.rs`'s lever: the drift machine
//! increments `wear`, the threshold machine only ever sets the edge
//! lock, never touches an attr) -- if content ever needs same-tick,
//! same-attr compounding across two distinct machines, that's a new,
//! deliberate design decision to make then, not a silent regression to
//! discover later.
//!
//! Other honest caveats, carried from the original spike unchanged:
//!
//! - `graded_range` is called in the DRIVER, not in a rule body: crepe
//!   fact fields must be `Copy`, so a `NamedNode` cannot appear in a
//!   fact and a rule body cannot recover the IRI to call
//!   `crate::graph::graded_range(attr)` the way `datalog_guard` calls
//!   `as_float` on whole `Term`s. The driver pre-quantizes each
//!   referenced attr's range into a `RangeOf(attr, lo_fp, hi_fp)` input
//!   fact; the clamp arithmetic itself lives in the rule head.
//! - Arithmetic is fixed-point (`i64`, scale 1e6) to keep fact fields
//!   `Copy`, matching `datalog_guard`'s `FIXED_POINT_SCALE`. This can
//!   differ from the f32 arithmetic it replaces by at most one
//!   quantization ULP at the boundaries. `quantize(f32::MIN/MAX)`
//!   saturates to `i64::MIN/MAX` (Rust float→int casts saturate), which
//!   is exactly the behavior wanted for the "no graded range" default.
//! - `GenerateFrontier` is out of scope and skipped: machines carrying
//!   it get no effect fact and therefore never fire here. It was never
//!   dispatched through `fire_object_verb` anyway (see `game.rs`'s own
//!   `build_effect_delta` doc comment: a `GenerateFrontier` machine is
//!   equipped to a room, not an item/npc, so this dispatch path can
//!   never find one to fire).
//! - Self-justification: a machine whose own effect feeds its own
//!   requirement forms a *positive* recursion (crepe accepts and
//!   terminates it -- `NewAttrValue` is computed from `BaseAttr`, not
//!   from `AttrNow`, so it derives exactly once). Semantically such a
//!   machine self-justifies its firing; flagging it so nobody mistakes
//!   it for guarded sequencing.

use std::collections::HashMap;

use crepe::crepe;
use oxigraph::model::{NamedNode, Term};

use crate::graph::{as_bool, as_float, graded_range, WorldGraph};
use crate::machine::Effect;
use crate::vocab;

/// Same convention as `datalog_guard`: IRI -> u32 so crepe fact fields stay `Copy`.
#[derive(Default)]
struct SymbolTable {
    by_str: HashMap<String, u32>,
    by_sym: Vec<NamedNode>,
}

impl SymbolTable {
    fn intern(&mut self, node: &NamedNode) -> u32 {
        if let Some(&s) = self.by_str.get(node.as_str()) {
            return s;
        }
        let s = self.by_sym.len() as u32;
        self.by_str.insert(node.as_str().to_string(), s);
        self.by_sym.push(node.clone());
        s
    }

    fn resolve(&self, s: u32) -> NamedNode {
        self.by_sym[s as usize].clone()
    }
}

const FIXED_POINT_SCALE: f64 = 1_000_000.0;

fn quantize(v: f32) -> i64 {
    (v as f64 * FIXED_POINT_SCALE).round() as i64
}

fn dequantize(v: i64) -> f32 {
    (v as f64 / FIXED_POINT_SCALE) as f32
}

/// `(current + step).clamp(lo, hi)` in fixed-point. Note `quantize(f32::MIN)`
/// and `quantize(f32::MAX)` saturate to `i64::MIN`/`i64::MAX`, which is exactly
/// the unbounded-clamp default we want for attrs with no graded range.
fn clamp_fp(cur: i64, step: i64, lo: i64, hi: i64) -> i64 {
    let raw = cur + step;
    if raw < lo {
        lo
    } else if raw > hi {
        hi
    } else {
        raw
    }
}

/// A machine's firing requirement -- the full vocabulary
/// `machine::Requirement` supports (see module doc point 3).
#[derive(Debug, Clone)]
pub enum Requirement {
    PlayerInRoom { room: NamedNode },
    EdgeLockedIs { edge: NamedNode, value: bool },
    AttrAtLeast { node: NamedNode, attr: NamedNode, min: f32 },
}

/// One machine's dispatch-relevant description: its effect plus every
/// requirement gating it. Bounded to 2 requirements -- see module doc
/// point 1 for why that's the real, verified maximum, not an arbitrary
/// limit.
#[derive(Debug, Clone)]
pub struct EffectMachine {
    pub id: NamedNode,
    pub effect: Effect,
    pub requirements: Vec<Requirement>,
}

impl EffectMachine {
    /// Panics if `requirements.len() > 2` -- see this module's doc
    /// comment for why 2 is real content's verified maximum, not a
    /// guess, and why silently dropping a third requirement would be
    /// the wrong failure mode.
    pub fn new(id: NamedNode, effect: Effect, requirements: Vec<Requirement>) -> Self {
        assert!(
            requirements.len() <= 2,
            "datalog_effects supports at most 2 requirements per machine \
             (real content's verified maximum); machine {id} has {}",
            requirements.len()
        );
        Self { id, effect, requirements }
    }
}

/// Everything the single fixpoint derived, ready to be committed as
/// deltas -- one entry per FIRING machine, not aggregated by (node, attr)
/// or (edge), so the caller can commit and render per machine in its own
/// chosen order (see module doc point 5).
#[derive(Debug, Default)]
pub struct EffectFixpoint {
    /// (machine, node, attr, new value) -- the old (node, attr, _) triple
    /// is retracted by whoever commits this, exactly as
    /// `build_effect_delta` would.
    pub attr_deltas: Vec<(NamedNode, NamedNode, NamedNode, f32)>,
    /// (machine, edge, new locked value).
    pub edge_locks: Vec<(NamedNode, NamedNode, bool)>,
    /// Every machine whose requirements were satisfied and whose effect
    /// fired -- a strict superset-by-construction of the machines named
    /// in `attr_deltas`/`edge_locks` (also includes ones whose effect was
    /// `GenerateFrontier`, out of scope, which never fires here in
    /// practice since `fire_object_verb` never finds one to dispatch).
    pub fired_machines: Vec<NamedNode>,
}

crepe! {
    // ---- input facts -------------------------------------------------
    // Existing world state (same shape as datalog_guard's inputs).
    @input
    struct AttrValue(u32, u32, i64);      // (node, attr, current value_fp)
    @input
    struct EdgeLockedState(u32, bool);    // (edge, locked)
    @input
    struct PlayerRoom(u32);               // singleton: the current room
    // Per-attr graded range, precomputed by the driver (see doc: fact
    // fields must be Copy, so `graded_range(&NamedNode)` is called there,
    // not here). Attrs with no graded range get (i64::MIN, i64::MAX).
    @input
    struct RangeOf(u32, i64, i64);        // (attr, lo_fp, hi_fp)
    // Machines and their effects. Kind is encoded by which relation the
    // driver emits, rather than a string tag: crepe facts are typed, and
    // a tag column would just be joined away again.
    @input
    struct HasIncrementAttr(u32, u32, u32, i64); // (machine, node, attr, step_fp)
    @input
    struct HasSetEdgeLocked(u32, u32, bool);     // (machine, edge, value)

    // Requirements, in two fixed slots (see module doc points 1-2 for
    // why this shape and not a general N-requirement/negation design).
    @input
    struct ReqSlot1AttrAtLeast(u32, u32, u32, i64);
    @input
    struct ReqSlot1EdgeLockedIs(u32, u32, bool);
    @input
    struct ReqSlot1PlayerInRoom(u32, u32);
    @input
    struct NoReqSlot1(u32);
    @input
    struct ReqSlot2AttrAtLeast(u32, u32, u32, i64);
    @input
    struct ReqSlot2EdgeLockedIs(u32, u32, bool);
    @input
    struct ReqSlot2PlayerInRoom(u32, u32);
    @input
    struct NoReqSlot2(u32);

    // ---- internal derived relations ----------------------------------
    struct BaseAttr(u32, u32, i64);       // pre-effect attr values
    struct HasBaseAttr(u32, u32);
    struct AttrNow(u32, u32, i64);        // base OR post-effect values
    struct EdgeNow(u32, bool);
    struct Slot1Sat(u32);
    struct Slot2Sat(u32);
    struct AllReqsMet(u32);

    // ---- outputs ------------------------------------------------------
    @output
    struct NewAttrValue(u32, u32, u32, i64); // (machine, node, attr, new value_fp)
    @output
    struct NewEdgeLocked(u32, u32, bool);    // (machine, edge, value)
    @output
    struct MachineFired(u32);                // (machine)

    // Base values: what the graph already says, defaulting to 0.0 for any
    // attr some machine wants to increment but that has no triple yet
    // (stratified: `HasBaseAttr` depends only on inputs).
    BaseAttr(node, attr, v) <- AttrValue(node, attr, v);
    HasBaseAttr(node, attr) <- AttrValue(node, attr, _);
    BaseAttr(node, attr, 0) <- HasIncrementAttr(_, node, attr, _),
                               !HasBaseAttr(node, attr);

    // "Current" state as visible to requirement checks: base values plus
    // every effect-derived value from ANY machine (including, self-
    // justification caveat above, potentially the checking machine
    // itself -- positive recursion only).
    AttrNow(node, attr, v) <- BaseAttr(node, attr, v);
    AttrNow(node, attr, v) <- NewAttrValue(_, node, attr, v);
    EdgeNow(edge, v) <- EdgeLockedState(edge, v);
    EdgeNow(edge, v) <- NewEdgeLocked(_, edge, v);

    // Each slot is satisfied if the machine has no requirement in that
    // slot, or the requirement it does have (whichever kind) holds.
    // Purely positive -- see module doc point 2.
    Slot1Sat(m) <- NoReqSlot1(m);
    Slot1Sat(m) <- ReqSlot1AttrAtLeast(m, node, attr, min), AttrNow(node, attr, v), (v >= min);
    Slot1Sat(m) <- ReqSlot1EdgeLockedIs(m, edge, want), EdgeNow(edge, want);
    Slot1Sat(m) <- ReqSlot1PlayerInRoom(m, room), PlayerRoom(room);

    Slot2Sat(m) <- NoReqSlot2(m);
    Slot2Sat(m) <- ReqSlot2AttrAtLeast(m, node, attr, min), AttrNow(node, attr, v), (v >= min);
    Slot2Sat(m) <- ReqSlot2EdgeLockedIs(m, edge, want), EdgeNow(edge, want);
    Slot2Sat(m) <- ReqSlot2PlayerInRoom(m, room), PlayerRoom(room);

    // A machine may fire iff it has an effect and every requirement slot
    // it has is satisfied.
    AllReqsMet(m) <- Slot1Sat(m), Slot2Sat(m), HasIncrementAttr(m, _, _, _);
    AllReqsMet(m) <- Slot1Sat(m), Slot2Sat(m), HasSetEdgeLocked(m, _, _);

    // Effect application. IncrementAttr reproduces build_effect_delta:
    // new = (current + step).clamp(graded_range(attr) or unbounded).
    // Head expression is fine here; crepe lowers it to Rust directly.
    NewAttrValue(m, node, attr, clamp_fp(cur, step, lo, hi)) <-
        HasIncrementAttr(m, node, attr, step),
        BaseAttr(node, attr, cur),
        RangeOf(attr, lo, hi),
        AllReqsMet(m);

    // SetEdgeLocked: retract-old/assert-new is the committer's job; here we
    // just derive the new assertion.
    NewEdgeLocked(m, edge, value) <-
        HasSetEdgeLocked(m, edge, value),
        AllReqsMet(m);

    MachineFired(m) <- AllReqsMet(m);
}

/// Runs the whole effect/requirement fixpoint in ONE `crepe .run()`.
///
/// The caller hands in the machines and the player's current room; this
/// function never loops over fixpoint iterations and never needs to be
/// re-invoked to make an effect satisfy another machine's requirement --
/// the cascade is derived inside the single evaluation (see module docs +
/// `cascade_three_machines`/`real_lever_mechanic_two_requirement_machine`
/// tests).
pub fn resolve_effects(
    graph: &WorldGraph,
    player_room: &NamedNode,
    machines: &[EffectMachine],
) -> EffectFixpoint {
    let mut syms = SymbolTable::default();
    let mut attr_values = Vec::new();
    let mut edge_states = Vec::new();
    let mut ranges = Vec::new();
    let mut has_inc = Vec::new();
    let mut has_lock = Vec::new();
    let mut req_slot1_attr = Vec::new();
    let mut req_slot1_edge = Vec::new();
    let mut req_slot1_room = Vec::new();
    let mut no_req_slot1 = Vec::new();
    let mut req_slot2_attr = Vec::new();
    let mut req_slot2_edge = Vec::new();
    let mut req_slot2_room = Vec::new();
    let mut no_req_slot2 = Vec::new();

    let player_room_sym = syms.intern(player_room);

    // Attrs/edges referenced anywhere (effects or requirements), so the
    // fixpoint sees the existing state it needs. Base-0 for missing attr
    // triples is handled by the `BaseAttr` default rule, not here.
    let mut referenced_attrs: Vec<(NamedNode, NamedNode)> = Vec::new();
    let mut referenced_edges: Vec<NamedNode> = Vec::new();

    for m in machines {
        match &m.effect {
            Effect::IncrementAttr { node, attr, .. } => {
                referenced_attrs.push((node.clone(), attr.clone()));
            }
            Effect::SetEdgeLocked { edge, .. } => {
                referenced_edges.push(edge.clone());
            }
            Effect::GenerateFrontier { .. } => {} // out of scope, skipped
        }
        for req in &m.requirements {
            match req {
                Requirement::AttrAtLeast { node, attr, .. } => {
                    referenced_attrs.push((node.clone(), attr.clone()));
                }
                Requirement::EdgeLockedIs { edge, .. } => {
                    referenced_edges.push(edge.clone());
                }
                Requirement::PlayerInRoom { .. } => {}
            }
        }
    }

    // Existing attr values + graded ranges.
    for (node, attr) in &referenced_attrs {
        let ns = syms.intern(node);
        let as_ = syms.intern(attr);
        let current = graph
            .object(node, attr)
            .and_then(|t: Term| as_float(&t))
            .unwrap_or(0.0);
        attr_values.push(AttrValue(ns, as_, quantize(current)));
        let (lo, hi) = graded_range(attr).unwrap_or((f32::MIN, f32::MAX));
        ranges.push(RangeOf(as_, quantize(lo), quantize(hi)));
    }

    // Existing edge lock states -- vocab::locked(), the real predicate
    // apply_commit/render.rs/everything else in this crate reads and
    // writes (see module doc point 4).
    for edge in &referenced_edges {
        let es = syms.intern(edge);
        let locked = graph
            .object(edge, &vocab::locked())
            .and_then(|t: Term| as_bool(&t))
            .unwrap_or(false);
        edge_states.push(EdgeLockedState(es, locked));
    }

    // Effects and requirement slots.
    for m in machines {
        let ms = syms.intern(&m.id);
        match &m.effect {
            Effect::IncrementAttr { node, attr, step } => {
                has_inc.push(HasIncrementAttr(
                    ms,
                    syms.intern(node),
                    syms.intern(attr),
                    quantize(*step),
                ));
            }
            Effect::SetEdgeLocked { edge, value } => {
                has_lock.push(HasSetEdgeLocked(ms, syms.intern(edge), *value));
            }
            Effect::GenerateFrontier { .. } => {} // out of scope, never fires here
        }

        match m.requirements.first() {
            Some(Requirement::AttrAtLeast { node, attr, min }) => {
                req_slot1_attr.push(ReqSlot1AttrAtLeast(
                    ms,
                    syms.intern(node),
                    syms.intern(attr),
                    quantize(*min),
                ));
            }
            Some(Requirement::EdgeLockedIs { edge, value }) => {
                req_slot1_edge.push(ReqSlot1EdgeLockedIs(ms, syms.intern(edge), *value));
            }
            Some(Requirement::PlayerInRoom { room }) => {
                req_slot1_room.push(ReqSlot1PlayerInRoom(ms, syms.intern(room)));
            }
            None => no_req_slot1.push(NoReqSlot1(ms)),
        }

        match m.requirements.get(1) {
            Some(Requirement::AttrAtLeast { node, attr, min }) => {
                req_slot2_attr.push(ReqSlot2AttrAtLeast(
                    ms,
                    syms.intern(node),
                    syms.intern(attr),
                    quantize(*min),
                ));
            }
            Some(Requirement::EdgeLockedIs { edge, value }) => {
                req_slot2_edge.push(ReqSlot2EdgeLockedIs(ms, syms.intern(edge), *value));
            }
            Some(Requirement::PlayerInRoom { room }) => {
                req_slot2_room.push(ReqSlot2PlayerInRoom(ms, syms.intern(room)));
            }
            None => no_req_slot2.push(NoReqSlot2(ms)),
        }
    }

    let mut program = Crepe::new();
    program.extend([PlayerRoom(player_room_sym)]);
    program.extend(attr_values);
    program.extend(edge_states);
    program.extend(ranges);
    program.extend(has_inc);
    program.extend(has_lock);
    program.extend(req_slot1_attr);
    program.extend(req_slot1_edge);
    program.extend(req_slot1_room);
    program.extend(no_req_slot1);
    program.extend(req_slot2_attr);
    program.extend(req_slot2_edge);
    program.extend(req_slot2_room);
    program.extend(no_req_slot2);

    let (new_attr, new_edge, fired) = program.run();

    let mut out = EffectFixpoint::default();
    for NewAttrValue(m, node, attr, v) in new_attr {
        out.attr_deltas.push((
            syms.resolve(m),
            syms.resolve(node),
            syms.resolve(attr),
            dequantize(v),
        ));
    }
    for NewEdgeLocked(m, edge, value) in new_edge {
        out.edge_locks.push((syms.resolve(m), syms.resolve(edge), value));
    }
    for MachineFired(m) in fired {
        out.fired_machines.push(syms.resolve(m));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Delta, WorldGraph as Graph};
    use crate::machine::{build_action_machine, Requirement as MachineRequirement};

    fn node(iri: &str) -> NamedNode {
        NamedNode::new(iri).unwrap()
    }

    fn wear() -> NamedNode {
        // The real graded attribute (`GRADED_ATTRS` in `graph.rs`), not a
        // fabricated IRI -- the first draft used `node("urn:dmml:attr:wear")`,
        // a plausible-looking string that doesn't match `graded_range`'s
        // actual table entry (keyed on `vocab::wear()`'s real IRI), so the
        // "clamped to a real bounded range" assertion below silently never
        // exercised the clamp branch at all (`graded_range` returned `None`
        // every time). Caught only by actually running the test, not by
        // reading the code -- it looked entirely reasonable.
        crate::vocab::wear()
    }

    fn ungraded_attr() -> NamedNode {
        // Deliberately not in graded_range's static table.
        node("urn:dmml:spike:definitely-ungraded-attr")
    }

    fn empty_graph() -> WorldGraph {
        WorldGraph::default()
    }

    /// Expected result of `build_effect_delta`'s arithmetic for an
    /// IncrementAttr, computed through the same fixed-point helpers this
    /// module uses (see module doc's fixed-point caveat re: ULP-level f32
    /// differences).
    fn expected_increment(current: f32, step: f32, attr: &NamedNode) -> f32 {
        let (lo, hi) = graded_range(attr).unwrap_or((f32::MIN, f32::MAX));
        dequantize(clamp_fp(quantize(current), quantize(step), quantize(lo), quantize(hi)))
    }

    #[test]
    fn single_machine_increment_attr_matches_build_effect_delta() {
        let graph = empty_graph();
        let room = node("urn:dmml:room:r");

        // (1) Attr WITH a graded range: verify clamp-to-range matches the
        // build_effect_delta formula. Step is huge so the bounded graded
        // range forces the clamp branch.
        let m1 = EffectMachine::new(
            node("urn:dmml:machine:m1"),
            Effect::IncrementAttr {
                node: node("urn:dmml:node:x"),
                attr: wear(),
                step: 1.0e9,
            },
            vec![],
        );
        let out = resolve_effects(&graph, &room, &[m1]);

        assert_eq!(out.attr_deltas.len(), 1);
        let (m, n, a, v) = &out.attr_deltas[0];
        assert_eq!(m, &node("urn:dmml:machine:m1"));
        assert_eq!(n, &node("urn:dmml:node:x"));
        assert_eq!(a, &wear());
        let (lo, hi) = graded_range(&wear()).unwrap_or((f32::MIN, f32::MAX));
        if lo.is_finite() || hi.is_finite() {
            assert!(
                *v == dequantize(quantize(lo)) || *v == dequantize(quantize(hi)),
                "expected clamped boundary value, got {v}"
            );
        }
        assert_eq!(*v, expected_increment(0.0, 1.0e9, &wear()));
        assert_eq!(out.fired_machines, vec![node("urn:dmml:machine:m1")]);

        // (2) Attr with NO graded range: defaults to (f32::MIN, f32::MAX),
        // i.e. unclamped — new value is exactly current(0) + step.
        assert!(graded_range(&ungraded_attr()).is_none());
        let m2 = EffectMachine::new(
            node("urn:dmml:machine:m2"),
            Effect::IncrementAttr {
                node: node("urn:dmml:node:x"),
                attr: ungraded_attr(),
                step: 3.5,
            },
            vec![],
        );
        let out = resolve_effects(&graph, &room, &[m2]);
        assert_eq!(out.attr_deltas.len(), 1);
        assert_eq!(out.attr_deltas[0].3, dequantize(quantize(3.5)));
    }

    #[test]
    fn single_machine_set_edge_locked() {
        let graph = empty_graph();
        let room = node("urn:dmml:room:r");
        let edge = node("urn:dmml:edge:e");
        let m = EffectMachine::new(
            node("urn:dmml:machine:m1"),
            Effect::SetEdgeLocked { edge: edge.clone(), value: false },
            vec![],
        );
        let out = resolve_effects(&graph, &room, &[m]);
        assert_eq!(
            out.edge_locks,
            vec![(node("urn:dmml:machine:m1"), edge, false)]
        );
        assert!(out.attr_deltas.is_empty());
        assert_eq!(out.fired_machines, vec![node("urn:dmml:machine:m1")]);
    }

    #[test]
    #[should_panic(expected = "at most 2 requirements")]
    fn three_requirements_panics_rather_than_silently_dropping_one() {
        EffectMachine::new(
            node("urn:dmml:machine:overloaded"),
            Effect::SetEdgeLocked { edge: node("urn:dmml:edge:e"), value: false },
            vec![
                Requirement::PlayerInRoom { room: node("urn:dmml:room:r") },
                Requirement::EdgeLockedIs { edge: node("urn:dmml:edge:e"), value: true },
                Requirement::AttrAtLeast {
                    node: node("urn:dmml:node:x"),
                    attr: wear(),
                    min: 1.0,
                },
            ],
        );
    }

    /// THE cascade test: M1 (no req, IncrementAttr X/wear +5, clamped by
    /// wear's real graded range 0.0-2.0 down to 2.0) → M2 (req
    /// AttrAtLeast(X, wear, >=2.0), SetEdgeLocked E=false) → M3 (req
    /// EdgeLockedIs(E, false), IncrementAttr Y/charge +1). All three must
    /// fire from ONE `.run()`, in one derived chain, with no manual
    /// re-invocation and no loop over machines.
    ///
    /// The threshold is 2.0, not the raw step of 5.0: `wear` is a real
    /// graded attribute (`GRADED_ATTRS`, range 0.0-2.0), so M1's clamped
    /// output is 2.0, not 5.0 -- proving the cascade correctly threads the
    /// *clamped* value into M2's requirement check, not the raw
    /// pre-clamp arithmetic.
    #[test]
    fn cascade_three_machines() {
        let graph = empty_graph();
        let room = node("urn:dmml:room:r");
        let x = node("urn:dmml:node:x");
        let y = node("urn:dmml:node:y");
        let wear_n = wear();
        let charge = node("urn:dmml:attr:charge");
        let edge_e = node("urn:dmml:edge:e");

        let machines = vec![
            EffectMachine::new(
                node("urn:dmml:machine:m1"),
                Effect::IncrementAttr { node: x.clone(), attr: wear_n.clone(), step: 5.0 },
                vec![],
            ),
            EffectMachine::new(
                node("urn:dmml:machine:m2"),
                Effect::SetEdgeLocked { edge: edge_e.clone(), value: false },
                vec![Requirement::AttrAtLeast {
                    node: x.clone(),
                    attr: wear_n.clone(),
                    min: 2.0,
                }],
            ),
            EffectMachine::new(
                node("urn:dmml:machine:m3"),
                Effect::IncrementAttr { node: y.clone(), attr: charge.clone(), step: 1.0 },
                vec![Requirement::EdgeLockedIs { edge: edge_e.clone(), value: false }],
            ),
        ];

        let out = resolve_effects(&graph, &room, &machines);

        let m1 = node("urn:dmml:machine:m1");
        let m2 = node("urn:dmml:machine:m2");
        let m3 = node("urn:dmml:machine:m3");
        for m in [&m1, &m2, &m3] {
            assert!(out.fired_machines.contains(m), "machine {m} did not fire");
        }

        let wear_delta = out
            .attr_deltas
            .iter()
            .find(|(m, n, a, _)| m == &m1 && n == &x && a == &wear_n)
            .expect("M1's IncrementAttr on X/wear missing");
        assert_eq!(wear_delta.3, expected_increment(0.0, 5.0, &wear_n));

        assert!(
            out.edge_locks.contains(&(m2.clone(), edge_e.clone(), false)),
            "M2's SetEdgeLocked missing — cascade M1→M2 broke"
        );

        let charge_delta = out
            .attr_deltas
            .iter()
            .find(|(m, n, a, _)| m == &m3 && n == &y && a == &charge)
            .expect("M3's IncrementAttr on Y/charge missing — cascade M2→M3 broke");
        assert_eq!(charge_delta.3, dequantize(quantize(1.0)));
    }

    /// Real content, not a synthetic fixture: `demiurge.rs`'s exact lever
    /// mechanic -- a drift machine (1 requirement: EdgeLocked(edge,
    /// true)) and a threshold machine (2 requirements: EdgeLocked(edge,
    /// true) AND AttrAtLeast(lever, wear, threshold)) both equipped to
    /// the same lever, both triggered by "pull". Built via the exact
    /// same `machine::build_action_machine` real content uses, then its
    /// `machine::Requirement`s are translated into this module's
    /// `Requirement` -- proving the 2-requirement slot design actually
    /// handles the one real machine that needs it, not just a
    /// hand-picked synthetic case.
    #[test]
    fn real_lever_mechanic_two_requirement_machine() {
        let mut graph = Graph::new();
        crate::demiurge::bootstrap(&mut graph);
        let edge = graph.fresh("edge/");
        let lever = graph.fresh("item/");
        let room = graph.fresh("room/");
        graph
            .commit(
                "test",
                Delta::new()
                    .assert(edge.clone(), vocab::rdf_type(), vocab::class_edge())
                    .assert(edge.clone(), vocab::locked(), crate::graph::lit_bool(true))
                    .assert(lever.clone(), vocab::rdf_type(), vocab::class_item())
                    .assert(lever.clone(), vocab::wear(), crate::graph::lit_float(0.0)),
            )
            .expect("fixture delta is always valid");

        const THRESHOLD: f32 = 1.0;
        const STEP: f32 = 0.5;

        let (drift_id, d1) = build_action_machine(
            &mut graph,
            &lever,
            "pull",
            &[MachineRequirement::EdgeLocked { edge: edge.clone(), equals: true }],
            &Effect::IncrementAttr { node: lever.clone(), attr: vocab::wear(), step: STEP },
        );
        graph.commit("test", d1).expect("drift machine is always valid");

        let (threshold_id, d2) = build_action_machine(
            &mut graph,
            &lever,
            "pull",
            &[
                MachineRequirement::EdgeLocked { edge: edge.clone(), equals: true },
                MachineRequirement::AttrAtLeast {
                    node: lever.clone(),
                    attr: vocab::wear(),
                    threshold: THRESHOLD,
                },
            ],
            &Effect::SetEdgeLocked { edge: edge.clone(), value: false },
        );
        graph.commit("test", d2).expect("threshold machine is always valid");

        let translate = |reqs: &[MachineRequirement]| -> Vec<Requirement> {
            reqs.iter()
                .map(|r| match r {
                    MachineRequirement::PlayerInRoom { room } => {
                        Requirement::PlayerInRoom { room: room.clone() }
                    }
                    MachineRequirement::EdgeLocked { edge, equals } => {
                        Requirement::EdgeLockedIs { edge: edge.clone(), value: *equals }
                    }
                    MachineRequirement::AttrAtLeast { node, attr, threshold } => {
                        Requirement::AttrAtLeast {
                            node: node.clone(),
                            attr: attr.clone(),
                            min: *threshold,
                        }
                    }
                })
                .collect()
        };

        let drift_machine = EffectMachine::new(
            drift_id.clone(),
            Effect::IncrementAttr { node: lever.clone(), attr: vocab::wear(), step: STEP },
            translate(&[MachineRequirement::EdgeLocked { edge: edge.clone(), equals: true }]),
        );
        let threshold_machine = EffectMachine::new(
            threshold_id.clone(),
            Effect::SetEdgeLocked { edge: edge.clone(), value: false },
            translate(&[
                MachineRequirement::EdgeLocked { edge: edge.clone(), equals: true },
                MachineRequirement::AttrAtLeast {
                    node: lever.clone(),
                    attr: vocab::wear(),
                    threshold: THRESHOLD,
                },
            ]),
        );

        // Turn 1: wear starts at 0.0, step 0.5 -- drift fires (its lone
        // requirement, EdgeLocked(true), holds), threshold does NOT (0.5
        // < 1.0 threshold).
        let out1 = resolve_effects(&graph, &room, &[drift_machine.clone(), threshold_machine.clone()]);
        assert!(out1.fired_machines.contains(&drift_id));
        assert!(
            !out1.fired_machines.contains(&threshold_id),
            "threshold machine must not fire before wear crosses its own threshold"
        );
        let (_, _, _, new_wear) = out1
            .attr_deltas
            .iter()
            .find(|(m, ..)| m == &drift_id)
            .expect("drift machine's IncrementAttr missing");
        assert_eq!(*new_wear, STEP);

        // Commit turn 1's result, same as fire_object_verb would.
        graph
            .commit(
                "player",
                Delta::new()
                    .retract(lever.clone(), vocab::wear(), crate::graph::lit_float(0.0))
                    .assert(lever.clone(), vocab::wear(), crate::graph::lit_float(*new_wear)),
            )
            .expect("committing turn 1's drift delta is always valid");

        // Turn 2: wear is now 0.5, another +0.5 step crosses the 1.0
        // threshold -- THE real-content cascade this whole extension
        // exists for: drift's own effect, in the SAME fixpoint, must
        // satisfy threshold's second (AttrAtLeast) requirement slot.
        let out2 = resolve_effects(&graph, &room, &[drift_machine, threshold_machine]);
        assert!(out2.fired_machines.contains(&drift_id));
        assert!(
            out2.fired_machines.contains(&threshold_id),
            "threshold machine must fire once wear reaches its threshold in the SAME fixpoint \
             as the drift machine's own increment — this is the real 2-requirement cascade"
        );
        assert!(out2
            .edge_locks
            .contains(&(threshold_id, edge.clone(), false)));
    }
}
