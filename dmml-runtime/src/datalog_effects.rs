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
//! 1. **Multi-requirement support -- genuinely unbounded, not capped at
//!    2.** The spike's `EffectMachine` carried `requirement:
//!    Option<Requirement>` -- at most one. An earlier revision of this
//!    module bounded it at 2 instead, justified as "the real, verified
//!    maximum across every `build_action_machine` call site in this
//!    workspace" -- that was a real mistake, corrected after Jason caught
//!    it (see written-world's `CLAUDE.md`, "Code is never ground truth
//!    for a domain invariant"): a grep over today's call sites answers
//!    what existing code happens to do, never what the domain actually
//!    allows. `requires: &[Requirement]` (`machine.rs`) is an unbounded
//!    slice, nothing in `vocab.rs`/`validate.rs` caps `has_requirement`
//!    triple count, and this is a sovereign, atproto-backed content
//!    system where a future `demiurge` generator or a hand-authored
//!    commit could equip any number of requirements without touching a
//!    line of Rust a cap would live in. Worse, the cap lived in
//!    `EffectMachine::new`, called directly from `fire_object_verb`'s
//!    live dispatch -- it would have panicked and crashed a real
//!    player's turn the day content needed a third requirement, directly
//!    contradicting this same file's own `GenerateFrontier` handling
//!    ("a future content bug... should degrade quietly, not crash a
//!    player's turn"). Replaced with a driver-built chain of arbitrary
//!    length (`ReqChainStart`/`ReqChainNext`/`ReqChainLast`,
//!    `NoRequirements` for the empty case) -- unification over an
//!    explicit position index, per Jason's own proposed design, adapted
//!    for the one real constraint that design needs (see point 2).
//! 2. **Why this still isn't negation, and why that matters.** Jason's
//!    original sketch for unbounded requirements used double negation
//!    ("no requirement this machine's type requires is unfulfilled") --
//!    exactly `datalog_guard.rs`'s own `UnmetRequirement`/
//!    `AllRequirementsMet` shape, which is completely correct THERE
//!    because gating never feeds into effect derivation in that module,
//!    so there's no cycle for the negation to sit inside. Here there is
//!    one: `AllReqsMet` feeds `NewAttrValue`/`NewEdgeLocked` feeds
//!    `AttrNow`/`EdgeNow` feeds the requirement check itself. This isn't
//!    a soft crepe limitation -- verified directly in crepe 0.2.0's own
//!    source (`strata.rs` computes strongly-connected components via
//!    Kosaraju's algorithm; `lib.rs` then walks every rule and calls
//!    `abort!` -- a hard compile error -- the instant a negated relation
//!    shares a stratum with its own rule's goal relation). So the chain
//!    design keeps Jason's actual insight (unification/joins generalize
//!    over an explicit index without needing enumeration or aggregation,
//!    which crepe has none of) while staying strictly positive: `ReqSatAt`
//!    (one clause per requirement kind, keyed by position instead of a
//!    fixed slot) and `AllSatUpTo` (recursive threading through the
//!    driver-built chain, one link at a time) never use `!` anywhere in
//!    the cycle. Positive recursion through the cycle is exactly caveat 2
//!    below and is fine; negation through it is not, and never appears.
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

use crepe::crepe;
use oxigraph::model::{NamedNode, Term};

use crate::datalog_support::{dequantize, quantize, SymbolTable};
use crate::graph::{as_bool, as_float, graded_range, WorldGraph};
use crate::machine::Effect;
use crate::vocab;

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
/// requirement gating it -- genuinely unbounded (see module doc point 1
/// for why an earlier version of this struct capped it at 2, and why
/// that was wrong: nothing in this repo's own schema/protocol caps how
/// many requirements a machine can carry, so neither should this).
#[derive(Debug, Clone)]
pub struct EffectMachine {
    pub id: NamedNode,
    pub effect: Effect,
    pub requirements: Vec<Requirement>,
}

impl EffectMachine {
    pub fn new(id: NamedNode, effect: Effect, requirements: Vec<Requirement>) -> Self {
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

    // Requirements, threaded through a driver-built chain of arbitrary
    // length -- see module doc point 1 for why a fixed slot count was
    // wrong and this replaces it. `position` is a plain 0-based index,
    // reused freely across different machines (every rule below always
    // joins it together with `m`, so it never needs to be globally
    // unique).
    @input
    struct ReqAttrAtLeastAt(u32, u32, u32, u32, i64); // (machine, position, node, attr, min_fp)
    @input
    struct ReqEdgeLockedIsAt(u32, u32, u32, bool);    // (machine, position, edge, want)
    @input
    struct ReqPlayerInRoomAt(u32, u32, u32);          // (machine, position, room)
    @input
    struct ReqChainStart(u32, u32);                   // (machine, first position)
    @input
    struct ReqChainNext(u32, u32, u32);                // (machine, position, next position)
    @input
    struct ReqChainLast(u32, u32);                    // (machine, last position)
    @input
    struct NoRequirements(u32);                       // (machine) -- the empty-chain case

    // ---- internal derived relations ----------------------------------
    struct BaseAttr(u32, u32, i64);       // pre-effect attr values
    struct HasBaseAttr(u32, u32);
    struct AttrNow(u32, u32, i64);        // base OR post-effect values
    struct EdgeNow(u32, bool);
    struct ReqSatAt(u32, u32);            // (machine, position): that one requirement holds
    struct AllSatUpTo(u32, u32);          // (machine, position): every requirement from the
                                          // chain's start through this position holds
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

    // Each chain position is satisfied if the requirement it names holds.
    // Purely positive -- see module doc point 2.
    ReqSatAt(m, pos) <- ReqAttrAtLeastAt(m, pos, node, attr, min), AttrNow(node, attr, v), (v >= min);
    ReqSatAt(m, pos) <- ReqEdgeLockedIsAt(m, pos, edge, want), EdgeNow(edge, want);
    ReqSatAt(m, pos) <- ReqPlayerInRoomAt(m, pos, room), PlayerRoom(room);

    // Positive recursive threading, not aggregation (crepe has none) and
    // not negation (would create a cycle with AttrNow/EdgeNow -- see
    // module doc point 2): "every requirement from the chain's start
    // through `pos` holds" is built up one link at a time. Terminates
    // because the driver-built chain is a finite, acyclic linked list.
    AllSatUpTo(m, pos) <- ReqChainStart(m, pos), ReqSatAt(m, pos);
    AllSatUpTo(m, next) <- AllSatUpTo(m, pos), ReqChainNext(m, pos, next), ReqSatAt(m, next);

    // A machine may fire iff it has an effect and either has no
    // requirements at all, or every requirement in its (arbitrary-length)
    // chain is satisfied through the last position.
    AllReqsMet(m) <- NoRequirements(m), HasIncrementAttr(m, _, _, _);
    AllReqsMet(m) <- NoRequirements(m), HasSetEdgeLocked(m, _, _);
    AllReqsMet(m) <- ReqChainLast(m, last), AllSatUpTo(m, last), HasIncrementAttr(m, _, _, _);
    AllReqsMet(m) <- ReqChainLast(m, last), AllSatUpTo(m, last), HasSetEdgeLocked(m, _, _);

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
    let mut req_attr_at = Vec::new();
    let mut req_edge_at = Vec::new();
    let mut req_room_at = Vec::new();
    let mut chain_start = Vec::new();
    let mut chain_next = Vec::new();
    let mut chain_last = Vec::new();
    let mut no_requirements = Vec::new();

    let player_room_sym = syms.intern(player_room.as_str());

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
        let ns = syms.intern(node.as_str());
        let as_ = syms.intern(attr.as_str());
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
        let es = syms.intern(edge.as_str());
        let locked = graph
            .object(edge, &vocab::locked())
            .and_then(|t: Term| as_bool(&t))
            .unwrap_or(false);
        edge_states.push(EdgeLockedState(es, locked));
    }

    // Effects and requirement slots.
    for m in machines {
        let ms = syms.intern(m.id.as_str());
        match &m.effect {
            Effect::IncrementAttr { node, attr, step } => {
                has_inc.push(HasIncrementAttr(
                    ms,
                    syms.intern(node.as_str()),
                    syms.intern(attr.as_str()),
                    quantize(*step),
                ));
            }
            Effect::SetEdgeLocked { edge, value } => {
                has_lock.push(HasSetEdgeLocked(ms, syms.intern(edge.as_str()), *value));
            }
            Effect::GenerateFrontier { .. } => {} // out of scope, never fires here
        }

        // Requirements, threaded as a chain of arbitrary length -- see
        // module doc point 1. `position` is just the 0-based index into
        // `m.requirements`; safe to reuse across machines since every
        // rule always joins it together with `ms`.
        if m.requirements.is_empty() {
            no_requirements.push(NoRequirements(ms));
        } else {
            chain_start.push(ReqChainStart(ms, 0));
            chain_last.push(ReqChainLast(ms, (m.requirements.len() - 1) as u32));
            for (pos, req) in m.requirements.iter().enumerate() {
                let pos = pos as u32;
                if let Some(next) = pos.checked_add(1) {
                    if (next as usize) < m.requirements.len() {
                        chain_next.push(ReqChainNext(ms, pos, next));
                    }
                }
                match req {
                    Requirement::AttrAtLeast { node, attr, min } => {
                        req_attr_at.push(ReqAttrAtLeastAt(
                            ms,
                            pos,
                            syms.intern(node.as_str()),
                            syms.intern(attr.as_str()),
                            quantize(*min),
                        ));
                    }
                    Requirement::EdgeLockedIs { edge, value } => {
                        req_edge_at.push(ReqEdgeLockedIsAt(ms, pos, syms.intern(edge.as_str()), *value));
                    }
                    Requirement::PlayerInRoom { room } => {
                        req_room_at.push(ReqPlayerInRoomAt(ms, pos, syms.intern(room.as_str())));
                    }
                }
            }
        }
    }

    let mut program = Crepe::new();
    program.extend([PlayerRoom(player_room_sym)]);
    program.extend(attr_values);
    program.extend(edge_states);
    program.extend(ranges);
    program.extend(has_inc);
    program.extend(has_lock);
    program.extend(req_attr_at);
    program.extend(req_edge_at);
    program.extend(req_room_at);
    program.extend(chain_start);
    program.extend(chain_next);
    program.extend(chain_last);
    program.extend(no_requirements);

    let (new_attr, new_edge, fired) = program.run();

    let mut out = EffectFixpoint::default();
    for NewAttrValue(m, node, attr, v) in new_attr {
        out.attr_deltas.push((
            syms.resolve_node(m),
            syms.resolve_node(node),
            syms.resolve_node(attr),
            dequantize(v),
        ));
    }
    for NewEdgeLocked(m, edge, value) in new_edge {
        out.edge_locks.push((syms.resolve_node(m), syms.resolve_node(edge), value));
    }
    for MachineFired(m) in fired {
        out.fired_machines.push(syms.resolve_node(m));
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

    /// Genuinely unbounded, not just "more than 2": four requirements, all
    /// four kinds represented across positions in an arbitrary order, all
    /// satisfied -- and one flipped false to prove the whole chain still
    /// correctly rejects on a single unmet link, wherever it sits (first,
    /// middle, or last position, not just the boundary a 2-slot design
    /// would have had to special-case). Real regression test for the
    /// mistake this module's own doc comment (point 1) records: there is
    /// no real maximum, and this proves the mechanism actually reflects
    /// that, not just says so.
    #[test]
    fn four_requirements_all_kinds_arbitrary_order() {
        let mut graph = empty_graph();
        let room = node("urn:dmml:room:r");
        let other_room = node("urn:dmml:room:other");
        let edge_a = node("urn:dmml:edge:a");
        let edge_b = node("urn:dmml:edge:b");
        let x = node("urn:dmml:node:x");

        graph
            .commit(
                "test",
                Delta::new()
                    .assert(edge_a.clone(), vocab::locked(), crate::graph::lit_bool(true))
                    .assert(edge_b.clone(), vocab::locked(), crate::graph::lit_bool(false))
                    .assert(x.clone(), vocab::wear(), crate::graph::lit_float(1.5)),
            )
            .expect("fixture delta is always valid");

        let requirements = vec![
            Requirement::EdgeLockedIs { edge: edge_a.clone(), value: true }, // holds
            Requirement::PlayerInRoom { room: room.clone() },                // holds
            Requirement::AttrAtLeast { node: x.clone(), attr: wear(), min: 1.0 }, // holds
            Requirement::EdgeLockedIs { edge: edge_b.clone(), value: true }, // does NOT hold (edge_b is false)
        ];

        let m = EffectMachine::new(
            node("urn:dmml:machine:four"),
            Effect::SetEdgeLocked { edge: edge_a.clone(), value: false },
            requirements.clone(),
        );
        let out = resolve_effects(&graph, &room, &[m]);
        assert!(
            !out.fired_machines.contains(&node("urn:dmml:machine:four")),
            "the 4th requirement (edge_b locked=true) does not hold, so the whole chain must reject"
        );

        // Flip the 4th requirement to what the graph actually says, and
        // wrong-room the PlayerInRoom one (2nd position) instead --
        // proves rejection isn't hardcoded to "the last position", it's
        // wherever in the chain the unmet link actually is.
        let mut requirements_wrong_middle = requirements.clone();
        requirements_wrong_middle[3] = Requirement::EdgeLockedIs { edge: edge_b.clone(), value: false };
        requirements_wrong_middle[1] = Requirement::PlayerInRoom { room: other_room };
        let m2 = EffectMachine::new(
            node("urn:dmml:machine:four"),
            Effect::SetEdgeLocked { edge: edge_a.clone(), value: false },
            requirements_wrong_middle,
        );
        let out2 = resolve_effects(&graph, &room, &[m2]);
        assert!(
            !out2.fired_machines.contains(&node("urn:dmml:machine:four")),
            "a mismatch at the 2nd position (not the last) must still reject the whole chain"
        );

        // Now all four genuinely hold.
        let mut requirements_all_hold = requirements;
        requirements_all_hold[3] = Requirement::EdgeLockedIs { edge: edge_b.clone(), value: false };
        let m3 = EffectMachine::new(
            node("urn:dmml:machine:four"),
            Effect::SetEdgeLocked { edge: edge_a, value: false },
            requirements_all_hold,
        );
        let out3 = resolve_effects(&graph, &room, &[m3]);
        assert!(
            out3.fired_machines.contains(&node("urn:dmml:machine:four")),
            "all four requirements genuinely hold, across all four kinds and positions -- must fire"
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
