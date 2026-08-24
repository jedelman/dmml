# DMML `machine` grammar and evaluation spec (issue #50 Tier 2)

What replaces `SPEC.md` SS10's "reserved, not specified" `machine_stmt`
production, and what `dmml::ast::MachineStmt`/a future `dmml::machine`
module implement. Grounded in the three decisions locked in
`dev-journal/2026-08-17-machine-primitive-guard-design.md` (commit
`630b48f`) and issue #50's own comment thread:

1. **One guard primitive, `EXISTS(pattern)`** — no boolean-expression
   grammar. A state check and a graph-traversal check are the same
   operation over patterns of different complexity, not two mechanisms.
2. **States and transitions are open vocabulary**, self-declared like
   predicates, addressed via `TripleRef` — no new naming scheme.
3. **Effects need no further design** — POSIWID. Firing a transition IS
   the already-proven atomic retract/assert pair
   (`retract_assert_atomicity.th`), never a separate "description" of
   what the machine does.

This spec adds one refinement beyond those three, decided while drafting
this file: **no OR/ONE-OF combinator either.** See "No disjunction"
below.

## The grammar

```ebnf
machine_stmt    = "machine" , node_ref , "{" ,
                     state_decl* ,
                     transition_decl* ,
                   "}" ;

state_decl      = "state" , ident ;
                   (* declares a state name usable by this machine's own
                      transitions -- self-declared, same convention as
                      `declare relation`/`declare attribute` (SPEC.md
                      SS3): naming a state here does not assert it; a
                      machine only enters a state when some commit's
                      `produces` block asserts (machine_node, "state",
                      <ident>) -- ordinarily as a transition's own effect
                      (see "Firing a transition" below), but nothing
                      stops an ordinary commit from asserting a machine's
                      first state directly at mint time, same as any
                      other triple. *)

transition_decl = "transition" , ident , [ params ] , "{" ,
                     [ "from" , ":" , ident ] ,
                     [ "to" , ":" , ident ] ,
                     guard_clause* ,
                     [ "effect" , ":" , effect_list ] ,
                   "}" ;
                   (* `from`/`to` are sugar (see "Firing a transition");
                      at least one of {a guard_clause, from+to, an
                      explicit effect_list} must be present -- a
                      transition with none of those would be an
                      unconditional no-op declaration, rejected at
                      validation time. A transition MAY have guards and
                      no effects at all -- a pure gate, whose only
                      "effect" is that the commit firing it exists (see
                      the `traverse` worked example below, which gates
                      movement without itself changing `edge/12`'s own
                      state). *)

params          = "(" , ident , { "," , ident } , ")" ;
                   (* transition parameters, e.g. `transition move(dest)`
                      -- bound at fire time by the commit that fires the
                      transition, referenced in guards/effects as `$dest` *)

guard_clause    = "guard" , ":" , [ "not" ] , exists_expr ;
                   (* the optional leading "not" is the only negation
                      DMML has -- a single boolean flag on one EXISTS
                      atom, not a general expression tree. See
                      "Negation" below. *)

exists_expr     = "EXISTS" , "(" , pattern , ")" ;

pattern         = pattern_term , path_hop , { path_hop } ;
                   (* always at least one hop -- a "pattern" is never a
                      bare term, it is always (anchor, predicate, term).
                      One hop is the single-triple case (Decision 1's
                      "boolean machine" degenerate case); more than one
                      hop is the traversal case. Same production either
                      way -- pattern complexity is a hop *count*, not a
                      different grammar rule. NO commas anywhere inside a
                      pattern -- terms and hops are whitespace-separated
                      only, same as DMML's own fact_stmt convention
                      ("room/42 opensTo room/43", no commas) and
                      identical to the multi-hop chain syntax below --
                      `EXISTS(player holds key/7)` is the single-hop
                      form of the exact same production as
                      `EXISTS(?room hasEdge self connectsTo $dest)`, not
                      a different, comma-delimited "triple" syntax. *)

path_hop        = ident , pattern_term ;
                   (* (predicate, term) -- chains onto the previous
                      term, which becomes the next hop's implicit
                      subject *)

pattern_term    = "self"           (* this machine's own node_ref *)
                | "$" , ident       (* a transition parameter *)
                | "?" , ident       (* an existentially-bound variable,
                                       scoped to this one EXISTS -- see
                                       "Multi-hop patterns and ?vars" *)
                | node_ref ;        (* a concrete, literal node *)

effect_list     = effect , { "," , effect } ;
effect          = ( "retract" | "assert" ) , ident ;
                   (* ident is a state declared via state_decl in the
                      same machine; "retract locked, assert unlocked"
                      retracts (self, "state", locked) and asserts
                      (self, "state", unlocked) -- the exact pair
                      `retract_assert_atomicity.th` already proves atomic.
                      An effect list may reference states from *this*
                      machine only -- a transition never reaches into
                      another machine's state; if a scenario needs that,
                      it needs its own transition on that other machine
                      (composed by a single commit firing both, not by
                      one transition mutating two machines). *)
```

`"state"` is a reserved predicate name here — the same way `rdf:type`/`a`
is reserved in `SPEC.md` SS3's two-tier rule. A machine's current state
is always the triple `(machine_node, "state", <declared-state-ident>)`;
nothing else is allowed to assert or retract that predicate on a node
that has a `machine_stmt` declaring it, outside of a transition's own
effects (enforced at validation time, not by the grammar).

## Firing a transition

Firing transition `T` with argument bindings `$p1=v1, ...` on machine
node `self`:

1. **Guard evaluation.** Build the guard list: if `from` is present,
   prepend the implicit (non-negated) guard `EXISTS(self, "state",
   from)` — `from` is pure sugar for this, not a separate mechanism
   (Decision 1's "no grammar-level AND/OR" already implies `from` can't
   be anything other than one more `EXISTS`). Then evaluate every
   guard's `pattern` against the current graph, substituting
   `$paramName` with its bound argument and `self` with the machine's
   own node; a guard written `not EXISTS(pattern)` passes exactly when
   the pattern does **not** resolve. **All guards are an implicit AND**
   — the transition fires only if every one resolves (accounting for its
   own `not`, if present). This is the whole "compound condition" story;
   there is no conjunction/disjunction operator in the grammar because a
   guard *list* already is conjunction, structurally, and each guard is
   now itself either an atom or its negation.
2. **Effects.** Build the effect list: if `to` is present, and `from`
   is also present, append the implicit effects `retract from, assert
   to` (again sugar, not a new mechanism). Then apply the full effect
   list — every `retract`/`assert` pair in it — as **one atomic
   retract/assert operation**, exactly the primitive
   `retract_assert_atomicity.th` proved this session: either the whole
   set of retractions+assertions lands, or none of it does. A
   transition with more than one retract/assert pair (e.g. `to` sugar
   plus an explicit extra effect) is still one atomic operation, not a
   sequence of smaller atomic steps — the contract's atomicity is over
   the whole effect list, not per-pair.
3. Firing a transition is authored the same way any other content-
   causing act is: an ordinary `commit`, whose `produces` triples are
   exactly the transition's resolved effect list. There is no separate
   "machine interpreter" write path — `dmml::interpret`'s existing
   produces-fold (`Materialized::from_commits`) is already the whole
   evaluator; a transition firing is just a commit whose author chose
   triples matching a declared transition's effects. (Validating that a
   given commit's triples actually satisfy some declared transition's
   guards, rather than asserting arbitrary state, is resolver-level
   policy — out of scope for this spec, same boundary
   `VALIDATION_SPEC.md`/`resolver.rs` already draw between "what a
   commit contains" and "whether a repo accepts it.")

## No disjunction

The naive extra primitive after `EXISTS` would be `EXISTS(A) OR
EXISTS(B)` — e.g. "unlock if the player holds the key OR knows the
code." Rejected, same as the boolean-expression language itself: instead
of one transition with a disjunctive guard, declare **separate
transitions**, one per alternative, each with its own single-`EXISTS`
(or single-AND-list) guard, both producing the same effect:

```dmml
machine door/9 {
  state locked
  state unlocked

  transition unlockWithKey {
    from: locked
    to: unlocked
    guard: EXISTS(player holds key/7)
  }

  transition unlockWithCode {
    from: locked
    to: unlocked
    guard: EXISTS(player knows code/9)
  }
}
```

This is strictly more expressive than an OR combinator would have been,
not a workaround: with `EXISTS(A) OR EXISTS(B)` as one guard, a fired
transition's own record can't say *which* disjunct actually held — the
commit just says "unlock fired." With two named transitions, the fired
transition's own identity (and its `TripleRef`, per Decision 2) already
answers "was it the key or the code" for free, as provenance, without
needing to inspect the guard evaluation that produced it. Multiple
"versions" of a machine's own transition set is the disjunction
mechanism — no new grammar needed, and it's a strict provenance
improvement over a compound boolean would have been.

## Negation

The one piece missing from "a guard list is an implicit AND of `EXISTS`
atoms": AND alone can't express "this must NOT be the case" (e.g. "the
door is unlocked" is naturally `NOT EXISTS(self state locked)` in a
design that only ever bothers declaring the `locked` state and never
bothers minting an explicit `unlocked` triple at all). This is not the
disjunction question again — negating one atom is a per-atom boolean
flip, not a second combinator that needs its own resolution:

```ebnf
guard_clause = "guard" , ":" , [ "not" ] , exists_expr ;
```

`not` attaches to exactly one `exists_expr` — there is still no
parenthesized sub-expression, precedence, or nesting; `not EXISTS(A, B,
C)` is itself one guard-list entry, evaluated as "does this pattern fail
to resolve," same shape as a bare `EXISTS(...)` guard otherwise. A guard
list of `not`-and-plain `EXISTS` entries is already full conjunction of
literals (in the propositional-logic sense, a guard list is a
conjunction of possibly-negated atoms) — everything AND+NOT can express,
which is everything this design needs: Decision 1 already covers
existence, this covers non-existence, and "No disjunction" above already
covers why OR isn't needed as a third operator (named alternative
transitions, not a combinator, remain the answer there — negation
doesn't change that: `not EXISTS(A) AND not EXISTS(B)` is De Morgan's
`NOT (EXISTS(A) OR EXISTS(B))`, itself just two negated guards in one
list, still no OR needed anywhere in the grammar).

## Multi-hop patterns and `?vars`

A pattern with more than one hop chains: each hop's `pattern_term`
becomes the *next* hop's implicit subject. `?ident` existentially binds
within one `EXISTS` only — it has no meaning outside the pattern it
appears in (no cross-guard binding; if two guards need to agree on the
same intermediate node, bind it to a transition parameter instead, since
parameters — unlike `?vars` — persist across the whole transition).

```
EXISTS(?room hasEdge self, self connectsTo $dest)
```

is not valid single-pattern syntax (two independent statements, not one
chain) — a real multi-hop pattern is written as one chain of hops off a
single anchor:

```
EXISTS(?room hasEdge self connectsTo $dest)
```

reads: find `?room` such that `?room -[hasEdge]-> self -[connectsTo]->
$dest` — i.e. `path_hop`s are `hasEdge self` then `connectsTo $dest`
chained onto the anchor `?room`.

## Target shape

```rust
pub struct MachineStmt {
    pub node: ast::NodeRef,
    pub states: Vec<StateDecl>,
    pub transitions: Vec<TransitionDecl>,
    pub span: ast::Span,
}

pub struct StateDecl {
    pub ident: String,
    pub span: ast::Span,
}

pub struct TransitionDecl {
    pub ident: String,
    pub params: Vec<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub guards: Vec<GuardClause>,
    pub effects: Vec<Effect>,
    pub span: ast::Span,
}

pub struct GuardClause {
    pub negated: bool,
    pub exists: ExistsExpr,
    pub span: ast::Span,
}

pub struct ExistsExpr {
    pub pattern: Pattern,
    pub span: ast::Span,
}

pub struct Pattern {
    pub anchor: PatternTerm,
    pub hops: Vec<PatternHop>,
}

pub struct PatternHop {
    pub predicate: String,
    pub term: PatternTerm,
}

pub enum PatternTerm {
    SelfRef,
    Param(String),
    Var(String),
    Node(ast::NodeRef),
}

pub enum Effect {
    Retract(String),
    Assert(String),
}
```

`resolve_transition(decl: &TransitionDecl) -> (Vec<GuardClause>,
Vec<Effect>)` desugars `from`/`to` into the full guard/effect lists per
"Firing a transition" steps 1–2 above — the single function every other
piece of tooling (a linter checking effect idents against `states`, a
future evaluator) should call rather than re-deriving the sugar
independently.

## Worked example: `Game::go`'s movement gate (issue #50's own named Tier 2 site)

`engine/src/game.rs:803-830`, today: `if locked { block } else { commit
arrival }`. As DMML content:

```dmml
machine edge/12 {
  state locked
  state unlocked

  transition unlock {
    from: locked
    to: unlocked
    guard: EXISTS(player holds key/7)
  }

  transition traverse(dest) {
    guard: EXISTS(self state unlocked)
    guard: EXISTS(?room hasEdge self connectsTo $dest)
  }
}
```

`traverse` exercises both pattern complexities the test scenario was
chosen for, in one transition's guard list, and has no `effect_list` of
its own — it is a pure gate:

- `guard: EXISTS(self state unlocked)` — a single-triple pattern (one
  hop off `self`), the degenerate/"boolean machine" case.
- `guard: EXISTS(?room hasEdge self connectsTo $dest)` — a two-hop
  pattern, the traversal case: find a room with an edge (this edge) that
  connects to the destination.

`resolve_transition(traverse)` desugars to (`from`/`to` absent, so no
sugar to expand):

- guards: exactly the two `ExistsExpr`s written above, unchanged.
- effects: `[]` — valid per the relaxed transition rule above (guards
  alone are sufficient to make this transition well-formed).

Firing `traverse(dest: room/43)` is a commit whose `produces` is
whatever the *actual* effect of moving is — the player's own location
changing — which is not a state of `edge/12`'s machine at all and so is
correctly outside this spec's `effect_list` grammar. `edge/12`'s machine
only ever answers "is this move allowed," which is exactly what its
guards check; a commit is free to `produces` triples belonging to other
nodes (the player's) alongside firing a gate it doesn't own the state
of, same as any ordinary commit always could. This is not a gap: Decision
3 (effects need no further design) is precisely why a transition isn't
required to assert/retract anything of its own — POSIWID means a
transition's definition IS whatever its guards+effects actually are, and
here that's "guards only, by design," not an omission needing a further
mechanism.

## Worked example: negation — a door that's open unless a guard is posted

```dmml
machine door/9 {
  state guarded

  transition enter {
    guard: not EXISTS(guardPost/3 occupiedBy ?guard)
  }
}
```

`door/9` never declares an `unlocked`/`open` state at all — there's
nothing to assert, because "open" here just means "not guarded," and
`not EXISTS` says that directly rather than requiring an explicit
`open` triple to be minted and kept in sync with `guardPost/3`'s own
occupancy. `resolve_transition(enter)`:

- guards: exactly `[GuardClause { negated: true, exists: ExistsExpr {
  pattern: Pattern { anchor: Node(guardPost/3), hops: [PatternHop {
  predicate: "occupiedBy", term: Var("guard") }] } } }]` — one negated
  guard, unchanged from what was written (no `from` to prepend).
- effects: `[]` — a pure gate, same shape as `traverse` above.

`enter` fires (the guard passes) exactly when `EXISTS(guardPost/3
occupiedBy ?guard)` does **not** resolve for any binding of `?guard` —
i.e. no occupant triple exists at all.

## Not fully worked, stated only in prose above — testing generalization, not example-matching

- A transition with `from`/`to` present but zero explicit `guard_clause`s
  (e.g. `unlock { from: locked, to: unlocked }`, no `guard:` line at all)
  should still be well-formed: `resolve_transition` desugars its guard
  list to exactly one implicit guard (`GuardClause { negated: false,
  exists: EXISTS(self, "state", locked) }`, from the `from` field) and
  its effect list to exactly `[Retract("locked"), Assert("unlocked")]`
  (from `to`) — the sugar alone is sufficient to make it non-trivial,
  without needing any author-written `guard:`/`effect:` line.
- A guard list mixing negated and non-negated entries in the same
  transition (e.g. `guard: EXISTS(self state unlocked)` alongside
  `guard: not EXISTS(guardPost/3 occupiedBy ?guard)`, combining
  `traverse`'s and `enter`'s guards on one hypothetical transition)
  should evaluate each entry independently against its own `negated`
  flag and AND the results — no special-casing for "a transition with
  at least one negated guard"; negation is purely local to the one
  `GuardClause` it's written on.

## Evaluating `EXISTS`

A requirement (a transition's full guard list) is nothing more than
**a list of paths, some negated** — that's the whole runtime story.
Evaluated against `crate::interpret::Materialized` (the existing
produces-fold: `(subject, predicate) -> TripleValue`, last-write-wins),
with a small `EvalContext` supplying the two things a pattern can
reference besides literal nodes: `self` (the machine's own node) and
`$param` bindings (from the commit firing the transition).

**Resolving a single `PatternTerm` to a concrete node string**, given
`ctx`:
- `SelfRef` → `Some(ctx.self_node)`.
- `Param(name)` → `ctx.params.get(name)`, cloned. **Missing** (a
  transition fired without binding a param its own pattern references)
  → `None` — treated as the guard simply failing, not a panic or an
  error type; the caller (whatever fires the transition) is responsible
  for supplying every declared param, same way any other malformed
  input just fails a guard rather than crashing.
- `Node(s)` → `Some(s.clone())` — a literal node reference, always
  resolves.
- `Var(name)` → `None`, always — a `?var` is never pre-bound; it's
  resolved by the walk below instead (existential search if it's the
  anchor, "accept whatever's there" if it's a later hop's term). Two
  occurrences of the same `?var` name within one pattern are **not**
  unified against each other — each occurrence is independently
  existential (deliberately simple: no cross-hop variable consistency
  checking, since no real scenario in this spec needs it, and DMML's
  own `Materialized` is a *function* of `(subject, predicate)` — at
  most one object per pair — so a later hop's `?var` never actually
  has more than one candidate value to search over anyway; only the
  *anchor* position, with no known starting subject, genuinely needs a
  search across candidates).

**Walking a pattern**, given a starting node `start` (a concrete
string) and the pattern's `hops`:

1. `current = start`.
2. For each `hop` in order: look up `Materialized::current_value(current,
   hop.predicate)`.
   - If it's `None`, or `Some` of a non-`Node` `TripleValue`
     (`Number`/`Boolean`/`Str`) — the walk fails (patterns only ever
     traverse `Node`-valued edges; a literal-valued fact can be the
     *end* of a chain conceptually but this evaluator, kept
     deliberately simple, doesn't special-case matching a hop's term
     against a non-`Node` value — every worked example's hops are
     `Node`s throughout).
   - If it's `Some(Node(actual))`: resolve `hop.term` via the rules
     above. If that resolves to `Some(expected)`, the walk fails unless
     `actual == &expected`; either way (match, or `hop.term` was a
     `Var` and resolved to `None`), `current` becomes `actual.clone()`
     for the next hop.
3. If every hop was consumed without failing, the walk **succeeds**.

**`EXISTS(pattern)` evaluation**:
- Resolve `pattern.anchor` via the term rules above.
- If it resolves to `Some(start)`: `EXISTS` holds iff walking from
  `start` succeeds.
- If it resolves to `None` (the anchor is a `?var`, or an unbound
  `$param`): `EXISTS` holds iff walking succeeds from **any** subject
  in `Materialized::subjects()` — existential search over every node
  that currently has at least one outgoing edge. (An unbound `$param`
  anchor and a `?var` anchor get the same treatment here — both are
  "no known starting point" — which is harmless: a transition fired
  without a param its own guard needs was already going to fail this
  guard one way or another, and this makes it fail via "found no
  matching subject" rather than a special missing-param path.)

**A `GuardClause` holds** iff `EXISTS(pattern)`'s result, XORed with
`negated`, is `true` — i.e. a non-negated guard passes when the pattern
resolves, a negated guard passes when it does *not*. **A guard list
(a `TransitionDecl`'s resolved `guards`, after `resolve_transition`'s
sugar) holds** iff every `GuardClause` in it holds — plain
conjunction, per "Firing a transition" above; there is no other
combinator, so evaluating the list is exactly `guards.iter().all(...)`.

## Worked examples (evaluator)

Using `Materialized` built from these three facts (as if from some
prior commit log): `edge/12 state unlocked`, `room/1 hasEdge edge/12`,
`edge/12 connectsTo room/2`.

1. `EXISTS(self state unlocked)` with `ctx.self_node = "edge/12"` →
   anchor resolves to `Some("edge/12")`; walk: `current_value("edge/12",
   "state") == Some(Node("unlocked"))`, hop term `Node("unlocked")`
   resolves to `Some("unlocked")`, matches → walk succeeds → **`true`**.
2. Same pattern with `ctx.self_node = "edge/99"` (a different, unminted
   edge) → `current_value("edge/99", "state")` is `None` → walk fails →
   **`false`**.
3. `EXISTS(?room hasEdge self connectsTo $dest)` with `ctx.self_node =
   "edge/12"`, `ctx.params = {"dest": "room/2"}` → anchor `?room`
   resolves to `None` → existential search over `Materialized::subjects()`
   (`{"edge/12", "room/1"}` here). Trying `"room/1"`: hop 1
   `current_value("room/1", "hasEdge") == Some(Node("edge/12"))`, term
   `SelfRef` resolves to `Some("edge/12")`, matches, `current =
   "edge/12"`; hop 2 `current_value("edge/12", "connectsTo") ==
   Some(Node("room/2"))`, term `Param("dest")` resolves to
   `Some("room/2")`, matches → walk succeeds for `"room/1"` → **`true`**
   (search short-circuits, doesn't need to try `"edge/12"` as a
   candidate too).
4. `not EXISTS(guardPost/3 occupiedBy ?guard)` against an EMPTY
   `Materialized` (nothing minted `guardPost/3` yet) → anchor
   `Node("guardPost/3")` resolves to `Some("guardPost/3")`; walk: hop 1
   `current_value("guardPost/3", "occupiedBy")` is `None` → walk fails
   → `EXISTS` is `false` → **`GuardClause.negated == true`, so the
   guard holds (`true`)**.

## Not fully worked, stated only in prose above — testing generalization, not example-matching

- A hop whose materialized value exists but is NOT a `Node` (e.g. a
  fact `room/1 dampness 0.4`, and a pattern tries `EXISTS(room/1
  dampness ?x)`) must fail the walk at that hop — `dampness`'s value is
  `Number`, not `Node`, and per the evaluator's own scoping note above,
  a non-`Node` value can never continue or terminate a pattern match,
  regardless of whether the hop's own term is a concrete node, a
  `Var`, or anything else.

## Wiring into the toolchain

Two small integration points connect `crate::machine` to the rest of
`dmml`, closing the loop from "a `Document` was parsed" to "can this
named transition fire right now":

1. **Finding every declared machine in a document.** `ast::Document`
   may contain zero or more `TopLevelItem::Machine(MachineStmt)` items,
   each an opaque `{ node: NodeRef, body: String, span }` (per
   `ast.rs`'s own doc comment — `machine` is grammar-reserved but its
   body was never structurally parsed until this crate's `machine`
   module existed). `parse_all_machines` runs `parse_machine_body` over
   every one of them, keyed by the machine's own node, joined the exact
   same way `lower::lower_reference` already joins a `NodeRef`'s
   segments (`node_ref.segments.join("/")` — e.g. `edge/12`, matching
   this spec's own worked examples' node naming throughout). Stops at
   the first malformed machine body, in document order (this crate's
   existing convention throughout — `lower_commit`/`validate_declarations`
   don't try to recover-and-continue past a single bad input either).

   ```rust
   pub fn parse_all_machines(
       doc: &ast::Document,
   ) -> Result<std::collections::HashMap<String, MachineBody>, (String, MachineParseError)>
   ```

   `Err((node, err))` — `node` is that machine's own joined node string
   (so a caller can report exactly *which* declared machine failed to
   parse), `err` is the underlying `MachineParseError`.

2. **Asking whether a named transition may fire right now.**
   `may_fire` looks up `ident` among `body.transitions` (by
   `TransitionDecl.ident`, exact match, first one found — transition
   idents are assumed unique within one machine body, same assumption
   `MACHINE_SPEC.md` has made throughout; nothing currently enforces
   uniqueness, that's a linter question for later, not this function's
   job), desugars it via `resolve_transition` (so `from`/`to` sugar is
   included in the guard check for free), and evaluates the resolved
   guard list via `eval_guards`. Returns `None` if no transition with
   that ident exists in `body` — not an error, since "does this
   machine even have a transition named X" is a distinct question from
   "can it fire" and callers need to tell the two apart (a `bool` alone
   can't distinguish "declared but blocked" from "never declared").

   ```rust
   pub fn may_fire(
       body: &MachineBody,
       ident: &str,
       ctx: &EvalContext,
       world: &crate::interpret::Materialized,
   ) -> Option<bool>
   ```

**Deliberately not yet wired**: actually validating that a *specific
candidate commit's* `produces` triples match a transition's resolved
`effects` (so a resolver could reject a commit that claims to fire
`unlock` but produces the wrong triples). This needs retraction-aware
materialization first — `crate::interpret::Materialized`'s own module
doc already flags cross-commit `consumes`-driven retraction as "NOT
wired in here... a real follow-up, not attempted in this pass," and an
effect's `retract` half has no representation in a produces-only fold
to check against. `may_fire` answers "is this transition currently
permitted to fire" (a guard question, fully answerable today); "did
this specific commit fire it correctly" (an effects-matching question)
stays blocked on that same, already-flagged retraction gap — not a new
gap this section is introducing.

## Worked examples (wiring)

Using the same three-fact `world()` as the evaluator examples above
(`edge/12 state unlocked`, `room/1 hasEdge edge/12`, `edge/12
connectsTo room/2`), and a document containing:

```dmml
machine edge/12 {
  state locked
  state unlocked

  transition unlock {
    from: locked
    to: unlocked
    guard: EXISTS(player holds key/7)
  }
}
```

1. `parse_all_machines(doc)` → `Ok(map)` where `map.len() == 1` and
   `map["edge/12"]` is the parsed `MachineBody` (one state pair, one
   transition) — the key is `"edge/12"`, not `"edge"` or `"12"` (the
   `NodeRef`'s segments `["edge", "12"]` joined with `/`).
2. `may_fire(&map["edge/12"], "unlock", &ctx, &world)` where
   `ctx.self_node == "edge/12"` and the world has NOT asserted `player
   holds key/7` → `Some(false)` — `resolve_transition` prepends the
   implicit `EXISTS(self state locked)` guard (from `unlock`'s `from:
   locked`), which itself is `false` here (`edge/12`'s materialized
   state is `unlocked`, not `locked`) — fails on the very first guard,
   never even needs to check the `holds key/7` guard's own truth to
   already know the answer is `false`.
3. `may_fire(&map["edge/12"], "openSesame", &ctx, &world)` (a
   never-declared transition ident) → `None`.
