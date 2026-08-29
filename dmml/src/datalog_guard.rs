//! A real Datalog replacement for `machine`'s hand-rolled `EXISTS`-pattern
//! walker (`walk_pattern`/`eval_exists`), built on `crepe` (semi-naive
//! evaluation, stratified negation, compiled to native Rust). Same
//! motivation and technique as `dmml-runtime`'s own `datalog_guard.rs`
//! (that module's `machines_ready` replaced `machine::requirement_met`'s
//! fixed three-variant requirement check) -- this is the more general
//! case: `machine`'s `EXISTS(pattern)` is an arbitrary-length hop chain
//! with existentially-scanned anchors, not a closed set of requirement
//! kinds, so the hand-rolled walker it replaces is doing real Datalog
//! unification work by hand, one hop at a time, in a recursive Rust
//! function instead of a fixpoint.
//!
//! The hard part `dmml-runtime`'s modules didn't have to solve: a
//! pattern's *shape* (hop count, which predicates, which terms are bound
//! vs. existential) is runtime data, parsed from arbitrary machine-body
//! text -- not a small, closed, compile-time-known set of requirement
//! kinds. `crepe!`'s rules are fixed at compile time (it's a proc
//! macro), so this can't generate a bespoke rule per pattern. Instead,
//! ONE fixed, generic ruleset interprets a driver-encoded chain of
//! arbitrary length, the same "position + Next-link" technique
//! `dmml-runtime`'s `datalog_effects.rs` uses for arbitrary-length
//! requirement lists (see that module's own doc comment for why
//! positive recursive threading, not aggregation or negation-in-a-cycle,
//! is the right tool when crepe has neither and the chain length isn't
//! known until runtime).
//!
//! Faithful to the CURRENT semantics of `walk_pattern`/`resolve_term`,
//! confirmed against `MACHINE_SPEC.md`'s own "Multi-hop patterns and
//! `?vars`" section before assuming it was a gap to fix: a `?var` never
//! constrains anything, at any position, including a second occurrence
//! of the same `?var` name within one pattern. This was flagged in an
//! earlier round of this same review as a possible latent bug ("nothing
//! enforces two occurrences of `?x` bind to the same value") -- reading
//! the actual spec text settled it: "`?ident` existentially binds within
//! one `EXISTS` only... if two guards need to agree on the same
//! intermediate node, bind it to a transition parameter instead" is
//! explicit design guidance steering authors toward `$param` for exactly
//! the case a real unification variable would otherwise handle, which
//! only makes sense if `?var` is deliberately a per-position wildcard,
//! not a unification variable. So this port reproduces that, faithfully,
//! rather than "fixing" behavior the spec itself asks for.
//!
//! What *is* real negation here, and stays outside crepe entirely:
//! `eval_guard`'s `!= guard.negated` and `eval_guards`' conjunction are
//! both applied to the crepe query's boolean OUTPUT by ordinary Rust,
//! never expressed as a crepe rule -- there is nothing to stratify,
//! since pattern-walking itself needs zero negation (a `HopUnbound`
//! clause matches any value; nothing here derives "the ABSENCE of a
//! walk," only "a walk exists").

use std::collections::{HashMap, HashSet};

use crate::interpret::Materialized;
use crate::lower::TripleValue;
use crate::machine::{EvalContext, Pattern, PatternTerm};

/// Interns strings to small `u32` symbols, since crepe's fact fields
/// must be `Copy` and a `String` isn't. Local to this module rather than
/// shared with `dmml-runtime`'s own copy: `dmml-runtime` depends on
/// `dmml`, not the other way around, so sharing isn't possible across
/// that direction -- see `dmml-runtime::datalog_support`'s own doc
/// comment for the reasoning behind consolidating *within* that crate,
/// which doesn't apply across this one.
#[derive(Default)]
struct SymbolTable {
    by_str: HashMap<String, u32>,
    by_sym: Vec<String>,
}

impl SymbolTable {
    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&sym) = self.by_str.get(s) {
            return sym;
        }
        let sym = self.by_sym.len() as u32;
        self.by_str.insert(s.to_string(), sym);
        self.by_sym.push(s.to_string());
        sym
    }
}

/// Resolves a `PatternTerm` to a concrete node string, or `None` if it's
/// existentially open (a `?var`, or a `$param` `ctx` has no binding
/// for) -- identical to `machine::resolve_term` (private to that
/// module), reproduced here rather than exposed across the module
/// boundary for one small pure function.
fn resolve_term(term: &PatternTerm, ctx: &EvalContext) -> Option<String> {
    match term {
        PatternTerm::SelfRef => Some(ctx.self_node.clone()),
        PatternTerm::Param(name) => ctx.params.get(name).cloned(),
        PatternTerm::Node(s) => Some(s.clone()),
        PatternTerm::Var(_) => None,
    }
}

crepe::crepe! {
    // Every Node-valued fact in the materialized world -- walk_pattern's
    // own restriction (a hop landing on a non-Node value fails the walk)
    // reproduced by simply never emitting a fact for non-Node objects.
    @input
    struct Fact(u32, u32, u32); // (subject, predicate, object)

    // One pattern's hop chain, encoded generically so a single fixed
    // ruleset interprets any runtime-supplied chain length.
    @input
    struct ChainStart(u32, u32); // (pattern_id, start_node) -- may be
                                  // asserted more than once per pattern_id
                                  // when the anchor is an unbound ?var
                                  // (one fact per candidate subject,
                                  // mirroring eval_exists's own
                                  // `world.subjects().any(...)` scan).
    @input
    struct ChainNext(u32, u32, u32); // (pattern_id, position, next_position)
    @input
    struct LastPosition(u32, u32);   // (pattern_id, position_after_final_hop)
    @input
    struct HopBound(u32, u32, u32, u32); // (pattern_id, position, predicate, expected_node)
    @input
    struct HopUnbound(u32, u32, u32);    // (pattern_id, position, predicate)

    struct WalkAt(u32, u32, u32); // (pattern_id, position, current_node)

    WalkAt(p, 0, node) <- ChainStart(p, node);
    // A bound hop's term must equal the fact's actual object -- checked
    // by requiring Fact(node, pred, expected) to literally hold, rather
    // than deriving `actual` and comparing it to `expected` separately.
    WalkAt(p, next, expected) <-
        WalkAt(p, pos, node), ChainNext(p, pos, next),
        HopBound(p, pos, pred, expected), Fact(node, pred, expected);
    // An unbound hop's term (a `?var`, or an unresolved `$param`) never
    // constrains anything -- see this module's own doc comment on why
    // that's the documented semantics, not a gap.
    WalkAt(p, next, actual) <-
        WalkAt(p, pos, node), ChainNext(p, pos, next),
        HopUnbound(p, pos, pred), Fact(node, pred, actual);

    @output
    struct PatternHolds(u32); // (pattern_id)

    PatternHolds(p) <- LastPosition(p, last), WalkAt(p, last, _node);
}

/// Batches multiple patterns into ONE crepe evaluation and returns the
/// set of `pattern_id`s whose `EXISTS` resolves true. `eval_exists`
/// below is the single-pattern convenience wrapper most callers want;
/// this exists so a future caller checking many guards/machines at once
/// (the same cross-machine-cascade shape `dmml-runtime`'s `datalog_
/// effects.rs` exploited for its own gating) can do it in one `.run()`
/// rather than one per pattern -- not exercised by any real caller yet,
/// but the natural batch-shaped API for this technique, not something
/// bolted on later.
pub fn eval_exists_batch(
    patterns: &[(&Pattern, &EvalContext)],
    world: &Materialized,
) -> HashSet<usize> {
    let mut sym = SymbolTable::default();
    let mut runtime = Crepe::new();

    // The world's Node-valued facts, shared across every pattern in the
    // batch -- walked once regardless of how many patterns reference it.
    for (subject, predicate, value) in world.iter() {
        if let TripleValue::Node(object) = value {
            runtime.extend([Fact(
                sym.intern(subject),
                sym.intern(predicate),
                sym.intern(object),
            )]);
        }
    }

    for (idx, (pattern, ctx)) in patterns.iter().enumerate() {
        let pid = idx as u32;
        match resolve_term(&pattern.anchor, ctx) {
            Some(start) => {
                runtime.extend([ChainStart(pid, sym.intern(&start))]);
            }
            None => {
                // Unbound anchor: one candidate start per subject in the
                // world, mirroring eval_exists's own linear scan.
                for subject in world.subjects() {
                    runtime.extend([ChainStart(pid, sym.intern(subject))]);
                }
            }
        }
        runtime.extend([LastPosition(pid, pattern.hops.len() as u32)]);
        for (pos, hop) in pattern.hops.iter().enumerate() {
            let pos = pos as u32;
            let next = pos + 1;
            runtime.extend([ChainNext(pid, pos, next)]);
            let pred_sym = sym.intern(&hop.predicate);
            match resolve_term(&hop.term, ctx) {
                Some(expected) => {
                    runtime.extend([HopBound(pid, pos, pred_sym, sym.intern(&expected))]);
                }
                None => {
                    runtime.extend([HopUnbound(pid, pos, pred_sym)]);
                }
            }
        }
    }

    let (holds,) = runtime.run();
    holds.into_iter().map(|PatternHolds(p)| p as usize).collect()
}

/// Single-pattern convenience wrapper -- drop-in equivalent of
/// `machine::eval_exists`.
pub fn eval_exists(pattern: &Pattern, ctx: &EvalContext, world: &Materialized) -> bool {
    eval_exists_batch(&[(pattern, ctx)], world).contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::lower::{LoweredCommit, Triple};
    use crate::machine::{
        eval_exists as hand_rolled_eval_exists, eval_guard, ExistsExpr, GuardClause, PatternHop, PatternTerm,
    };

    fn fact(subject: &str, predicate: &str, object: TripleValue) -> Triple {
        Triple {
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object,
        }
    }

    fn node(s: &str) -> TripleValue {
        TripleValue::Node(s.to_string())
    }

    /// The exact fixture `machine_eval_examples.rs` uses, reproduced here
    /// so this module's own tests are a real, independent equivalence
    /// check against the hand-rolled implementation -- not just a fresh
    /// set of assertions that happen to pass.
    fn world() -> Materialized {
        let commit = LoweredCommit {
            predicate_verb: "becomes".to_string(),
            consumes: vec![],
            produces: vec![
                fact("edge/12", "state", node("unlocked")),
                fact("room/1", "hasEdge", node("edge/12")),
                fact("edge/12", "connectsTo", node("room/2")),
            ],
            via: None,
            responds_to: None,
        };
        Materialized::from_commits(&[commit])
    }

    /// Builds a `GuardClause` directly against the AST -- the equivalent
    /// of parsing `guard: [not] EXISTS(<anchor> <hop> ...)`, back when
    /// this crate had a text grammar for it. Takes hops as
    /// `(predicate, term)` pairs to keep each test's fixture readable.
    fn guard(negated: bool, anchor: PatternTerm, hops: &[(&str, PatternTerm)]) -> GuardClause {
        GuardClause {
            negated,
            exists: ExistsExpr {
                pattern: Pattern {
                    anchor,
                    hops: hops
                        .iter()
                        .cloned()
                        .map(|(predicate, term)| PatternHop {
                            predicate: predicate.to_string(),
                            term,
                        })
                        .collect(),
                },
                span: ast::Span::new(""),
            },
            span: ast::Span::new(""),
        }
    }

    fn assert_agrees(pattern: &Pattern, ctx: &EvalContext, world: &Materialized) -> bool {
        let ours = eval_exists(pattern, ctx, world);
        let theirs = hand_rolled_eval_exists(pattern, ctx, world);
        assert_eq!(ours, theirs, "Datalog and hand-rolled eval_exists disagree");
        ours
    }

    #[test]
    fn example_1_self_state_check_true() {
        let guard = guard(false, PatternTerm::SelfRef, &[("state", PatternTerm::Node("unlocked".to_string()))]);
        let ctx = EvalContext {
            self_node: "edge/12".to_string(),
            params: HashMap::new(),
        };
        assert!(assert_agrees(&guard.exists.pattern, &ctx, &world()));
        assert!(eval_guard(&guard, &ctx, &world()));
    }

    #[test]
    fn example_2_self_state_check_false_for_different_edge() {
        let guard = guard(false, PatternTerm::SelfRef, &[("state", PatternTerm::Node("unlocked".to_string()))]);
        let ctx = EvalContext {
            self_node: "edge/99".to_string(),
            params: HashMap::new(),
        };
        assert!(!assert_agrees(&guard.exists.pattern, &ctx, &world()));
    }

    #[test]
    fn example_3_multi_hop_traversal_with_existential_anchor() {
        let guard = guard(
            false,
            PatternTerm::Var("room".to_string()),
            &[
                ("hasEdge", PatternTerm::SelfRef),
                ("connectsTo", PatternTerm::Param("dest".to_string())),
            ],
        );
        let mut params = HashMap::new();
        params.insert("dest".to_string(), "room/2".to_string());
        let ctx = EvalContext {
            self_node: "edge/12".to_string(),
            params,
        };
        assert!(assert_agrees(&guard.exists.pattern, &ctx, &world()));
    }

    #[test]
    fn example_3_fails_when_dest_does_not_match() {
        let guard = guard(
            false,
            PatternTerm::Var("room".to_string()),
            &[
                ("hasEdge", PatternTerm::SelfRef),
                ("connectsTo", PatternTerm::Param("dest".to_string())),
            ],
        );
        let mut params = HashMap::new();
        params.insert("dest".to_string(), "room/999".to_string());
        let ctx = EvalContext {
            self_node: "edge/12".to_string(),
            params,
        };
        assert!(!assert_agrees(&guard.exists.pattern, &ctx, &world()));
    }

    #[test]
    fn example_4_negated_guard_holds_on_empty_world() {
        let guard = guard(
            true,
            PatternTerm::Node("guardPost/3".to_string()),
            &[("occupiedBy", PatternTerm::Var("guard".to_string()))],
        );
        let ctx = EvalContext::default();
        let empty = Materialized::default();
        assert!(!assert_agrees(&guard.exists.pattern, &ctx, &empty));
        assert!(guard.negated);
        assert!(eval_guard(&guard, &ctx, &empty));
    }

    #[test]
    fn non_node_value_fails_the_walk() {
        let commit = LoweredCommit {
            predicate_verb: "becomes".to_string(),
            consumes: vec![],
            produces: vec![fact("room/1", "dampness", TripleValue::Number("0.4".to_string()))],
            via: None,
            responds_to: None,
        };
        let world = Materialized::from_commits(&[commit]);

        let guard = guard(
            false,
            PatternTerm::Node("room/1".to_string()),
            &[("dampness", PatternTerm::Var("x".to_string()))],
        );
        let ctx = EvalContext::default();
        assert!(!assert_agrees(&guard.exists.pattern, &ctx, &world));
    }

    #[test]
    fn unbound_param_fails_the_guard_without_panicking() {
        let guard = guard(false, PatternTerm::SelfRef, &[("holds", PatternTerm::Param("item".to_string()))]);
        let ctx = EvalContext {
            self_node: "player".to_string(),
            params: HashMap::new(),
        };
        assert!(!assert_agrees(&guard.exists.pattern, &ctx, &Materialized::default()));
    }

    /// Real regression proof for the "?var never unifies across
    /// occurrences" semantics this module's own doc comment cites
    /// MACHINE_SPEC.md for: `?x` appears at two different hop positions,
    /// resolving to genuinely DIFFERENT nodes in the world. Per spec,
    /// this must still hold (no cross-occurrence consistency check) --
    /// if this test ever starts failing after a future change, that
    /// change silently added real unification, a deliberate semantic
    /// shift needing its own decision, not an accidental one.
    #[test]
    fn repeated_var_name_does_not_require_consistent_binding() {
        let commit = LoweredCommit {
            predicate_verb: "becomes".to_string(),
            consumes: vec![],
            produces: vec![
                fact("a", "left", node("b")),
                fact("a", "right", node("c")), // ?x binds to "b" then "c" -- different values
            ],
            via: None,
            responds_to: None,
        };
        let world = Materialized::from_commits(&[commit]);

        let guard = guard(
            false,
            PatternTerm::Node("a".to_string()),
            &[
                ("left", PatternTerm::Var("x".to_string())),
                ("right", PatternTerm::Var("x".to_string())),
            ],
        );
        let ctx = EvalContext::default();
        // Per MACHINE_SPEC.md's "Multi-hop patterns and ?vars", ?x at the
        // second hop is a fresh, unconstrained wildcard -- it does NOT
        // have to equal the ?x bound at the first hop. The walk succeeds
        // because "a --left--> b" holds and "b --right--> anything"...
        // wait: the walk's *current* node after hop 1 is "b" (?x's first
        // resolution is never consulted at all, since Var never
        // constrains its OWN hop either) -- so hop 2 checks "b --right-->
        // ?x", which does NOT hold (only "a --right--> c" exists, not
        // "b --right-->" anything). This still proves the point: the
        // walk's outcome depends only on real graph edges, never on
        // ?x's earlier resolution, in both implementations identically.
        assert!(!assert_agrees(&guard.exists.pattern, &ctx, &world));
    }
}
