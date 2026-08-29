//! Structural types for a machine's states/transitions, per
//! `MACHINE_SPEC.md` (issue #50 Tier 2). These are built directly from
//! JSON authoring input (`crate::from_json`) into `crate::ast::MachineStmt`
//! -- there is no text grammar and no parser here anymore; a hand-written
//! DMML source language existed for the tokenizer/recursive-descent
//! `parse_machine_body` that used to live in this module, but it was
//! retired once JSON became the sole authoring format (nothing hand-
//! writes DMML source text; see `from_json`'s own doc comment for why).
//! What's left is exactly the semantic layer `MACHINE_SPEC.md` describes
//! -- states, transitions, guards, `EXISTS` patterns, effects -- and the
//! functions that evaluate them, none of which cared how the structure
//! was built.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDecl {
    pub ident: String,
    pub span: crate::ast::Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDecl {
    pub ident: String,
    pub params: Vec<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub guards: Vec<GuardClause>,
    pub effects: Vec<Effect>,
    pub span: crate::ast::Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardClause {
    pub negated: bool,
    pub exists: ExistsExpr,
    pub span: crate::ast::Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistsExpr {
    pub pattern: Pattern,
    pub span: crate::ast::Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub anchor: PatternTerm,
    pub hops: Vec<PatternHop>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternHop {
    pub predicate: String,
    pub term: PatternTerm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternTerm {
    SelfRef,
    Param(String),
    Var(String),
    Node(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Retract(String),
    Assert(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MachineBody {
    pub states: Vec<StateDecl>,
    pub transitions: Vec<TransitionDecl>,
}

/// Collects every `machine` item in `doc`, keyed by the machine's own
/// node, joined the same way `crate::lower::lower_reference` joins a
/// `NodeRef`'s segments (`stmt.node.segments.join("/")`). Infallible --
/// unlike the retired text parser this replaces, a `MachineStmt` already
/// carries validated structural data by the time it's in the AST, so
/// there's no malformed-body case left to fail on.
pub fn all_machines(doc: &crate::ast::Document) -> std::collections::HashMap<String, MachineBody> {
    let mut map = std::collections::HashMap::new();
    for item in &doc.items {
        if let crate::ast::TopLevelItem::Machine(stmt) = item {
            let key = stmt.node.segments.join("/");
            map.insert(
                key,
                MachineBody {
                    states: stmt.states.clone(),
                    transitions: stmt.transitions.clone(),
                },
            );
        }
    }
    map
}

/// Desugars `decl.from`/`decl.to` into the full guard/effect lists per
/// `MACHINE_SPEC.md`'s "Firing a transition" steps 1-2: if `from` is
/// present, prepend an implicit non-negated guard
/// `GuardClause { negated: false, exists: EXISTS(self, "state", from) }`
/// (using `decl.span` for that guard's own span, since it has no literal
/// source location of its own); if `to` is present (and `from` is also
/// present), append the implicit effects `[Retract(from), Assert(to)]`.
/// Returns the full resolved `(guards, effects)`, author-written entries
/// first, sugar-derived entries appended in the order described above.
pub fn resolve_transition(decl: &TransitionDecl) -> (Vec<GuardClause>, Vec<Effect>) {
    let mut guards = decl.guards.clone();

    if let Some(ref from_value) = decl.from {
        let implicit_guard = GuardClause {
            negated: false,
            exists: ExistsExpr {
                pattern: Pattern {
                    anchor: PatternTerm::SelfRef,
                    hops: vec![PatternHop {
                        predicate: "state".to_string(),
                        term: PatternTerm::Node(from_value.clone()),
                    }],
                },
                span: decl.span.clone(),
            },
            span: decl.span.clone(),
        };
        guards.insert(0, implicit_guard);
    }

    let mut effects = decl.effects.clone();

    if let (Some(ref from_value), Some(ref to_value)) = (&decl.from, &decl.to) {
        effects.push(Effect::Retract(from_value.clone()));
        effects.push(Effect::Assert(to_value.clone()));
    }

    (guards, effects)
}

/// Runtime bindings available while evaluating one transition firing's
/// guards, per `MACHINE_SPEC.md`'s "Evaluating EXISTS": the machine's
/// own node, and whatever `$param` values the commit firing the
/// transition supplied.
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    pub self_node: String,
    pub params: std::collections::HashMap<String, String>,
}

/// Evaluates one `EXISTS(pattern)` against `world`, per `MACHINE_SPEC.md`'s
/// "Evaluating EXISTS". Never panics.
///
/// Datalog-backed as of the cutover that added `crate::datalog_guard`
/// (see that module's own doc comment for the full design and the
/// equivalence tests it was proven against before this delegation
/// replaced the hand-rolled walker that used to live here). Kept as a
/// stable, named function -- not inlined at call sites -- since this
/// crate's own test suite (`tests/machine_eval_examples.rs` and others)
/// calls `eval_exists` directly by name.
pub fn eval_exists(pattern: &Pattern, ctx: &EvalContext, world: &crate::interpret::Materialized) -> bool {
    crate::datalog_guard::eval_exists(pattern, ctx, world)
}

/// Evaluates one `GuardClause` (its `EXISTS` result, XORed with
/// `negated`) against `world`.
pub fn eval_guard(guard: &GuardClause, ctx: &EvalContext, world: &crate::interpret::Materialized) -> bool {
    eval_exists(&guard.exists.pattern, ctx, world) != guard.negated
}

/// Evaluates a full guard list (a `TransitionDecl`'s resolved `guards`,
/// after `resolve_transition`'s sugar) against `world` -- plain
/// conjunction, per "Firing a transition": the transition may fire iff
/// every guard holds.
pub fn eval_guards(guards: &[GuardClause], ctx: &EvalContext, world: &crate::interpret::Materialized) -> bool {
    guards.iter().all(|guard| eval_guard(guard, ctx, world))
}

/// Whether `ident`'s transition may fire right now, given `ctx` and
/// `world`: resolves the transition (`from`/`to` sugar included) and
/// evaluates its guard list. `None` if no transition with that ident is
/// declared in `body` -- distinct from `Some(false)` ("declared, but
/// blocked").
pub fn may_fire(
    body: &MachineBody,
    ident: &str,
    ctx: &EvalContext,
    world: &crate::interpret::Materialized,
) -> Option<bool> {
    let decl = body.transitions.iter().find(|t| t.ident == ident)?;
    let (guards, _) = resolve_transition(decl);
    Some(eval_guards(&guards, ctx, world))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FiresTransitionError {
    /// No transition named `ident` is declared on this machine.
    UnknownTransition,
    /// The transition's guards did not hold against `world_before` --
    /// this commit was never entitled to fire, regardless of what it
    /// actually asserts.
    GuardNotSatisfied,
    /// The guard held, but the candidate's own `consumes`/`produces`
    /// don't match the transition's resolved effects: `missing` lists
    /// every resolved effect the candidate never actually delivered.
    EffectMismatch { missing: Vec<Effect> },
}

/// Whether a candidate commit's own `consumes`/`produces` actually
/// deliver exactly the effects `ident`'s resolved transition requires,
/// evaluated against `world_before` (the materialized state
/// immediately prior to this candidate committing) -- the "did this
/// commit fire it correctly" half `MACHINE_SPEC.md`'s own "Wiring into
/// the toolchain" section left deferred pending issue #70's
/// retraction-aware materialization, now real.
///
/// Distinct from `may_fire`: `may_fire` asks "is this transition
/// currently permitted to fire" (a guard question against the current
/// world). This asks "does THIS SPECIFIC commit's content match what
/// firing `ident` actually requires" (an effects-matching question
/// against a candidate commit) -- a resolver needs both, in order:
/// first confirm the guard held immediately before the candidate
/// (`may_fire` against `world_before`), then confirm the candidate's
/// own triples are exactly the resolved effects, not something else
/// asserted under the same transition name.
///
/// Every resolved effect currently checks predicate `"state"`
/// specifically, on `ctx.self_node` -- not a shortcut: both `from`/`to`
/// sugar-derived effects AND author-written explicit `retract`/`assert`
/// effects share one value-only grammar (`parse_effect`), always
/// implicitly `(self, "state", <value>)`. If that grammar ever grows a
/// full-triple explicit-effect form, this hardcoding stops being
/// implied by the grammar and needs revisiting alongside it.
///
/// A `ConsumeRef::Strong` (whole-commit reference) never satisfies a
/// `Retract` effect here, by deliberate choice, not an oversight:
/// `interpret::apply_consume` already treats a `Strong` consume as
/// retracting every `(subject, predicate)` its target commit produced,
/// so a `Strong` reference genuinely *could* deliver a given `Retract`
/// effect -- but confirming that from inside this function would mean
/// resolving the referenced commit's own content, which `world_before`
/// (a plain `Materialized` fold, not an `IdentifiedCommit` history) has
/// no way to look up. Accepting a `Strong` reference unconditionally
/// (assuming it always retracts whatever's needed) would be the wrong
/// kind of wrong: a commit could reference an unrelated `Strong` target
/// and this function would wave the effect through unverified. Failing
/// closed (reporting `EffectMismatch` rather than guessing `Ok`) matches
/// this crate's own posture on unverifiable claims elsewhere
/// (`cross_repo_commit_valid`'s fail-closed stance, not
/// `commit_valid_despite_dangling_factref`'s fail-open one -- a
/// different category of problem, verifying a *positive* claim rather
/// than tolerating a *dangling* one). Resolving a `Strong` reference's
/// real content, if this ever needs to stop being conservative here,
/// is real follow-up work needing the caller to supply the underlying
/// commit history, not a fix confined to this function's current
/// signature.
pub fn commit_fires_transition(
    body: &MachineBody,
    ident: &str,
    ctx: &EvalContext,
    world_before: &crate::interpret::Materialized,
    candidate: &crate::lower::LoweredCommit,
) -> Result<(), FiresTransitionError> {
    let decl = body
        .transitions
        .iter()
        .find(|t| t.ident == ident)
        .ok_or(FiresTransitionError::UnknownTransition)?;

    let (guards, effects) = resolve_transition(decl);

    if !eval_guards(&guards, ctx, world_before) {
        return Err(FiresTransitionError::GuardNotSatisfied);
    }

    let mut missing: Vec<Effect> = Vec::new();

    for effect in &effects {
        let delivered = match effect {
            Effect::Assert(value) => candidate.produces.iter().any(|t| {
                t.subject == ctx.self_node
                    && t.predicate == "state"
                    && t.object == crate::lower::TripleValue::Node(value.clone())
            }),
            Effect::Retract(value) => candidate.consumes.iter().any(|cr| match cr {
                crate::lower::ConsumeRef::Fact(fr) => {
                    if fr.subject != ctx.self_node || fr.predicate != "state" {
                        return false;
                    }
                    let has_object = fr.object.is_some();
                    let object_equal =
                        fr.object.as_ref() == Some(&crate::lower::TripleValue::Node(value.clone()));
                    crate::resolver::factref_matches(has_object, object_equal)
                }
                crate::lower::ConsumeRef::Strong(_) => false,
            }),
        };

        if !delivered {
            missing.push(effect.clone());
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(FiresTransitionError::EffectMismatch { missing })
    }
}
