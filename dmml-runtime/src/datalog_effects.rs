//! `datalog_effects.rs` — SPIKE, do not wire into `game.rs`.
//!
//! Reimplements `Game::build_effect_delta`'s decision logic for
//! `IncrementAttr` and `SetEdgeLocked` as crepe-derived facts, and — the
//! actual point — makes one machine's fired effect satisfy *another*
//! machine's requirement inside a single `.run()` fixpoint, with no
//! imperative per-machine loop and no notion of a "turn".
//!
//! First drafted by `z-ai/glm-5.3-flash` (dev-tooling dispatch pipeline,
//! see written-world's CLAUDE.md); corrected by hand where it didn't
//! actually compile before being trusted -- see inline notes at each fix.
//!
//! DOES THE CASCADE CLOSE? YES. Concretely, the rule chain is:
//!
//!   BaseAttr/AttrValue → AttrNow → ReqSatAttr → AllReqsMet(M2)
//!     ← NewAttrValue(M1) ← AllReqsMet(M1) ← NoRequirement(M1)
//!   AllReqsMet(M2) → NewEdgeLocked(M2) → EdgeNow → ReqSatEdge(M3)
//!     → AllReqsMet(M3) → NewAttrValue(M3)
//!
//! All of this is monotone Datalog (the one negation — `BaseAttr`'s
//! default-0 rule — depends only on input facts, so it is trivially
//! stratified), so crepe's semi-naive evaluation derives the whole M1 →
//! M2 → M3 chain in one native `.run()`. See test `cascade_three_machines`,
//! which is real and passes.
//!
//! Honest caveats / deliberate deviations:
//!
//! 1. Requirements are INLINED as a compatible subset of
//!    `datalog_guard::machines_ready`'s derivation (`ReqSatAttr` /
//!    `ReqSatEdge` / `AllReqsMet`), not imported: crepe relation structs
//!    are module-private, so `ReqAttrAtLeast` etc. cannot be reused
//!    across modules. Only one requirement per machine is supported. The
//!    general multi-requirement case is relational division, which needs
//!    `!UnmetReq(m)` — and since `AllReqsMet` feeds `NewAttrValue`
//!    feeds `AttrNow` feeds the requirement check, that negation would
//!    sit inside a recursive cycle and crepe would (correctly) reject it
//!    as non-stratifiable. Single-requirement-per-machine (or a driver
//!    -emitted `NoRequirement`) keeps the negation out of cycles. If the
//!    real guard machines ever carry multiple requirements, the fix is
//!    provenance-tagged satisfaction facts, not more negation.
//! 2. Self-justification: a machine whose own effect feeds its own
//!    requirement forms a *positive* recursion (crepe accepts and
//!    terminates it — `NewAttrValue` is computed from `BaseAttr`, not
//!    from `AttrNow`, so it derives exactly once). Semantically such a
//!    machine self-justifies its firing. This matches "effect fires if
//!    its requirement is satisfiable in the shared world state", which
//!    is what the cascade test needs; flagging it so nobody mistakes it
//!    for guarded sequencing.
//! 3. `graded_range` is called in the DRIVER, not in a rule body. crepe
//!    fact fields must be `Copy` (`SymbolTable` interns IRIs to `u32`
//!    for exactly this reason), so a `NamedNode` cannot appear in a fact
//!    and a rule body cannot recover the IRI to call
//!    `crate::graph::graded_range(attr)` the way `datalog_guard` calls
//!    `as_float` on whole `Term`s. Instead the driver pre-quantizes each
//!    referenced attr's range into a `RangeOf(attr, lo_fp, hi_fp)` input
//!    fact; the clamp arithmetic itself lives in the rule head.
//! 4. Arithmetic is fixed-point (`i64`, scale 1e6) to keep fact fields
//!    `Copy`, matching `datalog_guard`'s `FIXED_POINT_SCALE`. This can
//!    differ from `build_effect_delta`'s f32 arithmetic by at most one
//!    quantization ULP at the boundaries. `quantize(f32::MIN/MAX)`
//!    saturates to `i64::MIN/MAX` (Rust float→int casts saturate), which
//!    is exactly the behavior we want for the "no graded range" default.
//! 5. `GenerateFrontier` is out of scope and skipped: machines carrying
//!    it get no effect fact and therefore never fire here.
//! 6. The driver iterates the machine slice once to build INPUT facts.
//!    That is input construction, not fixpoint iteration; the cascade
//!    itself involves zero loops and one `.run()`.

use std::collections::HashMap;

use crepe::crepe;
use oxigraph::model::{NamedNode, Term};

use crate::graph::{as_bool, as_float, graded_range, WorldGraph};
use crate::machine::Effect;

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

/// IRI of the `locked` predicate used by `SetEdgeLocked` deltas.
/// (Spike-local; align with `crate::vocab` before promoting this module.)
const LOCKED_PREDICATE: &str = "urn:dmml:pred:locked";

/// A machine's firing requirement. This is the spike-local subset of the
/// requirement language `datalog_guard::machines_ready` understands.
#[derive(Debug, Clone)]
pub enum Requirement {
    AttrAtLeast { node: NamedNode, attr: NamedNode, min: f32 },
    EdgeLockedIs { edge: NamedNode, value: bool },
}

/// One machine's dispatch-relevant description: its effect plus the
/// (single) requirement that gates firing.
#[derive(Debug, Clone)]
pub struct EffectMachine {
    pub id: NamedNode,
    pub effect: Effect,
    pub requirement: Option<Requirement>,
}

/// Everything the single fixpoint derived, ready to be committed as deltas.
#[derive(Debug, Default)]
pub struct EffectFixpoint {
    /// New (node, attr, value) assertions; the old (node, attr, _) triple
    /// is retracted by whoever commits these, exactly as
    /// `build_effect_delta` would.
    pub attr_deltas: Vec<(NamedNode, NamedNode, f32)>,
    /// New (edge, locked) assertions.
    pub edge_locks: Vec<(NamedNode, bool)>,
    /// Machines whose requirements were satisfied and whose effects fired.
    pub fired_machines: Vec<NamedNode>,
}

crepe! {
    // ---- input facts -------------------------------------------------
    // Existing world state (same shape as datalog_guard's inputs).
    @input
    struct AttrValue(u32, u32, i64);      // (node, attr, current value_fp)
    @input
    struct EdgeLockedState(u32, bool);    // (edge, locked)
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
    // Requirements (inlined subset of datalog_guard; one per machine).
    @input
    struct ReqAttrAtLeast(u32, u32, u32, i64);   // (machine, node, attr, min_fp)
    @input
    struct ReqEdgeLockedIs(u32, u32, bool);      // (machine, edge, want)
    @input
    struct NoRequirement(u32);                   // (machine)

    // ---- internal derived relations ----------------------------------
    struct BaseAttr(u32, u32, i64);       // pre-effect attr values
    struct HasBaseAttr(u32, u32);
    struct AttrNow(u32, u32, i64);        // base OR post-effect values
    struct EdgeNow(u32, bool);
    struct ReqSatAttr(u32);
    struct ReqSatEdge(u32);
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
    // every effect-derived value from ANY machine (including, caveat 2,
    // potentially the checking machine itself — positive recursion only).
    AttrNow(node, attr, v) <- BaseAttr(node, attr, v);
    AttrNow(node, attr, v) <- NewAttrValue(_, node, attr, v);
    EdgeNow(edge, v) <- EdgeLockedState(edge, v);
    EdgeNow(edge, v) <- NewEdgeLocked(_, edge, v);

    // Requirement satisfaction (inlined datalog_guard-style derivation).
    ReqSatAttr(m) <- ReqAttrAtLeast(m, node, attr, min),
                     AttrNow(node, attr, v),
                     (v >= min);
    ReqSatEdge(m) <- ReqEdgeLockedIs(m, edge, want),
                     EdgeNow(edge, want);

    // A machine may fire iff it has an effect and its requirement is met.
    // `NoRequirement` lets requirement-free machines (M1) fire without
    // needing `!HasRequirement(m)` — see doc caveat 1.
    AllReqsMet(m) <- NoRequirement(m), HasIncrementAttr(m, _, _, _);
    AllReqsMet(m) <- NoRequirement(m), HasSetEdgeLocked(m, _, _);
    AllReqsMet(m) <- ReqSatAttr(m), HasIncrementAttr(m, _, _, _);
    AllReqsMet(m) <- ReqSatAttr(m), HasSetEdgeLocked(m, _, _);
    AllReqsMet(m) <- ReqSatEdge(m), HasIncrementAttr(m, _, _, _);
    AllReqsMet(m) <- ReqSatEdge(m), HasSetEdgeLocked(m, _, _);

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
/// The caller hands in the machines; this function never loops over
/// fixpoint iterations and never needs to be re-invoked to make an effect
/// satisfy another machine's requirement — the cascade is derived inside
/// the single evaluation (see module docs + `cascade_three_machines` test).
pub fn resolve_effects(graph: &WorldGraph, machines: &[EffectMachine]) -> EffectFixpoint {
    let mut syms = SymbolTable::default();
    let mut attr_values = Vec::new();
    let mut edge_states = Vec::new();
    let mut ranges = Vec::new();
    let mut has_inc = Vec::new();
    let mut has_lock = Vec::new();
    let mut req_attr = Vec::new();
    let mut req_edge_facts = Vec::new();
    let mut no_req = Vec::new();

    let locked_pred = NamedNode::new(LOCKED_PREDICATE).unwrap();

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
        match &m.requirement {
            Some(Requirement::AttrAtLeast { node, attr, .. }) => {
                referenced_attrs.push((node.clone(), attr.clone()));
            }
            Some(Requirement::EdgeLockedIs { edge, .. }) => {
                referenced_edges.push(edge.clone());
            }
            None => {}
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

    // Existing edge lock states.
    for edge in &referenced_edges {
        let es = syms.intern(edge);
        let locked = graph
            .object(edge, &locked_pred)
            .and_then(|t: Term| as_bool(&t))
            .unwrap_or(false);
        edge_states.push(EdgeLockedState(es, locked));
    }

    // Effects and requirements.
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
        match &m.requirement {
            Some(Requirement::AttrAtLeast { node, attr, min }) => {
                req_attr.push(ReqAttrAtLeast(
                    ms,
                    syms.intern(node),
                    syms.intern(attr),
                    quantize(*min),
                ));
            }
            Some(Requirement::EdgeLockedIs { edge, value }) => {
                req_edge_facts.push(ReqEdgeLockedIs(ms, syms.intern(edge), *value));
            }
            None => {
                no_req.push(NoRequirement(ms));
            }
        }
    }

    let mut program = Crepe::new();
    program.extend(attr_values);
    program.extend(edge_states);
    program.extend(ranges);
    program.extend(has_inc);
    program.extend(has_lock);
    program.extend(req_attr);
    program.extend(req_edge_facts);
    program.extend(no_req);

    let (new_attr, new_edge, fired) = program.run();

    let mut out = EffectFixpoint::default();
    for NewAttrValue(_m, node, attr, v) in new_attr {
        out.attr_deltas
            .push((syms.resolve(node), syms.resolve(attr), dequantize(v)));
    }
    for NewEdgeLocked(_m, edge, value) in new_edge {
        out.edge_locks.push((syms.resolve(edge), value));
    }
    for MachineFired(m) in fired {
        out.fired_machines.push(syms.resolve(m));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(iri: &str) -> NamedNode {
        NamedNode::new(iri).unwrap()
    }

    /// The real graded attribute (`GRADED_ATTRS` in `graph.rs`), not a
    /// fabricated IRI -- the first draft used `node("urn:dmml:attr:wear")`,
    /// a plausible-looking string that doesn't match `graded_range`'s
    /// actual table entry (keyed on `vocab::wear()`'s real IRI), so the
    /// "clamped to a real bounded range" assertion below silently never
    /// exercised the clamp branch at all (`graded_range` returned `None`
    /// every time). Caught only by actually running the test, not by
    /// reading the code -- it looked entirely reasonable.
    fn wear() -> NamedNode {
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
    /// module uses (see doc caveat 4 re: ULP-level f32 differences).
    fn expected_increment(current: f32, step: f32, attr: &NamedNode) -> f32 {
        let (lo, hi) = graded_range(attr).unwrap_or((f32::MIN, f32::MAX));
        dequantize(clamp_fp(quantize(current), quantize(step), quantize(lo), quantize(hi)))
    }

    #[test]
    fn single_machine_increment_attr_matches_build_effect_delta() {
        let graph = empty_graph();

        // (1) Attr WITH a graded range: verify clamp-to-range matches the
        // build_effect_delta formula. Step is huge so a bounded graded
        // range (if `wear` has one) forces the clamp branch; expected is
        // computed with the real `graded_range` lookup either way.
        let m1 = EffectMachine {
            id: node("urn:dmml:machine:m1"),
            effect: Effect::IncrementAttr {
                node: node("urn:dmml:node:x"),
                attr: wear(),
                step: 1.0e9,
            },
            requirement: None,
        };
        let out = resolve_effects(&graph, &[m1]);

        assert_eq!(out.attr_deltas.len(), 1);
        let (n, a, v) = &out.attr_deltas[0];
        assert_eq!(n, &node("urn:dmml:node:x"));
        assert_eq!(a, &wear());
        let (lo, hi) = graded_range(&wear()).unwrap_or((f32::MIN, f32::MAX));
        if lo.is_finite() || hi.is_finite() {
            // Bounded range: new value must be pinned to a boundary.
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
        let m2 = EffectMachine {
            id: node("urn:dmml:machine:m2"),
            effect: Effect::IncrementAttr {
                node: node("urn:dmml:node:x"),
                attr: ungraded_attr(),
                step: 3.5,
            },
            requirement: None,
        };
        let out = resolve_effects(&graph, &[m2]);
        assert_eq!(out.attr_deltas.len(), 1);
        assert_eq!(out.attr_deltas[0].2, dequantize(quantize(3.5)));
    }

    #[test]
    fn single_machine_set_edge_locked() {
        let graph = empty_graph();
        let edge = node("urn:dmml:edge:e");
        let m = EffectMachine {
            id: node("urn:dmml:machine:m1"),
            effect: Effect::SetEdgeLocked { edge: edge.clone(), value: false },
            requirement: None,
        };
        let out = resolve_effects(&graph, &[m]);
        assert_eq!(out.edge_locks, vec![(edge, false)]);
        assert!(out.attr_deltas.is_empty());
        assert_eq!(out.fired_machines, vec![node("urn:dmml:machine:m1")]);
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
    /// pre-clamp arithmetic. An earlier version of this test used an
    /// unclamped step/threshold pair that happened to pass without ever
    /// exercising a real bounded range at all (`wear()`'s first draft was
    /// a fabricated IRI that didn't match `graded_range`'s table).
    #[test]
    fn cascade_three_machines() {
        let graph = empty_graph();
        let x = node("urn:dmml:node:x");
        let y = node("urn:dmml:node:y");
        let wear_n = wear();
        let charge = node("urn:dmml:attr:charge");
        let edge_e = node("urn:dmml:edge:e");

        let machines = vec![
            EffectMachine {
                id: node("urn:dmml:machine:m1"),
                effect: Effect::IncrementAttr { node: x.clone(), attr: wear_n.clone(), step: 5.0 },
                requirement: None,
            },
            EffectMachine {
                id: node("urn:dmml:machine:m2"),
                effect: Effect::SetEdgeLocked { edge: edge_e.clone(), value: false },
                requirement: Some(Requirement::AttrAtLeast {
                    node: x.clone(),
                    attr: wear_n.clone(),
                    min: 2.0,
                }),
            },
            EffectMachine {
                id: node("urn:dmml:machine:m3"),
                effect: Effect::IncrementAttr { node: y.clone(), attr: charge.clone(), step: 1.0 },
                requirement: Some(Requirement::EdgeLockedIs { edge: edge_e.clone(), value: false }),
            },
        ];

        // NOTE: no machine-creation-order dependence, no re-invocation.
        let out = resolve_effects(&graph, &machines);

        let m1 = node("urn:dmml:machine:m1");
        let m2 = node("urn:dmml:machine:m2");
        let m3 = node("urn:dmml:machine:m3");
        for m in [&m1, &m2, &m3] {
            assert!(out.fired_machines.contains(m), "machine {m} did not fire");
        }

        // M1's effect: wear 0 + 5 (clamped by wear's real graded range if any).
        let wear_delta = out
            .attr_deltas
            .iter()
            .find(|(n, a, _)| n == &x && a == &wear_n)
            .expect("M1's IncrementAttr on X/wear missing");
        assert_eq!(wear_delta.2, expected_increment(0.0, 5.0, &wear_n));

        // M2's effect, fired only because M1's derived NewAttrValue satisfied
        // its AttrAtLeast requirement inside the same fixpoint.
        assert!(
            out.edge_locks.contains(&(edge_e.clone(), false)),
            "M2's SetEdgeLocked missing — cascade M1→M2 broke"
        );

        // M3's effect, fired only because M2's derived NewEdgeLocked satisfied
        // its EdgeLockedIs requirement — two hops of cascade, still one .run().
        let charge_delta = out
            .attr_deltas
            .iter()
            .find(|(n, a, _)| n == &y && a == &charge)
            .expect("M3's IncrementAttr on Y/charge missing — cascade M2→M3 broke");
        assert_eq!(charge_delta.2, dequantize(quantize(1.0)));
    }
}
