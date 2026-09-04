# DMML Surface — a new text authoring syntax (spike, not shipped)

**Status: spike, started 2026-08-31, machine authoring added
2026-09-01. Commits and machines — still no `reference` or batching.**
Not a revival of the retired text grammar (`SPEC.md` §10's
old EBNF) — that one is gone and stays gone; this is a new design,
parsed by a new parser (`DMML.Surface`, `dmml-hs`), targeting the same
validated `DMML.Ast.CommitStmt` the JSON front-end (`DMML.FromJson`)
already builds. **One AST, two front-ends** — `DMML.Surface`'s whole job
is text → the exact same `CommitStmt`/`FactStmt`/`Value`/etc. that
`commitStmtFromInput` builds from JSON, so every existing validator
(`isValidIdent`, `isValidNodeRef`, the duplicate-fact check) is reused
verbatim, not reimplemented.

**Why try text again after JSON tested well**: not settled by the
2026-08-31 JSON compliance checkpoint (15/15) — that checkpoint says
JSON-via-`GRAMMAR.md` works, it doesn't say a *better-designed* text
surface wouldn't also work, or work better. The original retired text
grammar was an ad hoc block syntax nobody deliberately optimized for
what a tool-calling model already writes fluently; real, heavily-trained
Haskell syntax is a different bet — worth its own checkpoint, not
assumed either way. That checkpoint hasn't been run yet; see "Not done
yet" below.

## Design principles

- **Lean on real Haskell conventions a model has actually seen a lot
  of**, not an invented-from-scratch syntax: infix backtick application,
  `::` for type-of (a direct, honest parallel to DMML's own `a`/`rdf:type`
  sugar — "`room/1 :: a Room`" reads exactly as it means), record-dot
  field access for plain facts.
- **Let lexical shape carry the tag** that JSON's `{"kind": ...}` had to
  spell out explicitly: a bare, unquoted, slash-shaped token is a node
  reference; a quoted string is a string literal; a bare numeral is a
  number literal; `true`/`false` are booleans. No explicit discriminant
  needed in the surface syntax — the grammar itself disambiguates,
  same information, less to write.
- **No general offside-rule layout algorithm (yet).** Real Haskell's
  layout rule is a real parser project on its own; this spike uses a
  single fixed indentation level per block instead of full recursive
  layout, to stay a spike. If nested constructs need real layout later
  (a machine's guards/effects, most likely), that's separate follow-up,
  not assumed solved here.

## Grammar (informal, commits only)

```
commit <verb>
  declare relation <ident>
  declare attribute <ident>

  <node_ref> :: a <node_ref>
  <node_ref> `<ident>` <value>
  <node_ref> . <ident> = <value>

  consumes
    strong <uri> # <cid>
    fact <uri> # <cid>
      <node_ref> . <ident>
      <node_ref> . <ident> = <value>

  refs
    <role_ident> <uri> # <cid>
```

- `<verb>`, `<ident>`, `<role_ident>` — same lexical class as JSON's
  bare identifiers (`isValidIdent`, reused directly).
- `<node_ref>` — same `segment(/segment)*` shape as JSON's node
  references (`isValidNodeRef`, reused directly).
- `<value>` — a `<node_ref>` (bareword), a quoted string (`"..."`), a
  bare numeral (`0.6`, `3`), or `true`/`false`.
- `<node_ref> :: a <node_ref>` — sugar for the `"a"`/`rdf:type` fact,
  matching JSON's own `predicate_ref = "a" | ident` distinction.
- `` <node_ref> `<ident>` <value> `` — infix backtick application: a
  plain relation/attribute fact, subject-predicate-object in reading
  order (`room/1 \`opensTo\` room/2`).
- `<node_ref> . <ident> = <value>` — dot-field-assignment form of the
  same fact shape; both forms lower to the identical `FactStmt`, purely
  a style choice for whichever reads better for a given predicate (infix
  verbs like `opensTo`; dot-assignment for attribute-like predicates
  like `dampness`).
- `consumes`/`fact ... # ...` sub-block's dot line with no `=` is the
  wildcard-object omission (same semantics as JSON's omitted `object`
  field); with `=` it's an explicit object, same as JSON's present
  `object` field.
- `#` separates a `<uri>` from its `<cid>` in both `consumes` and
  `refs` — evokes a fragment/anchor, not reused from anywhere else in
  this grammar.

## Example

```
commit mints
  declare relation opensTo
  declare attribute dampness

  room/1 :: a Room
  room/2 :: a Room
  room/1 `opensTo` room/2
  room/1 . dampness = 0.6
```

Parses to the identical `CommitStmt` the JSON authoring example in
`GRAMMAR.md` produces for the same content.

## Grammar (informal, machines)

```
machine <node_ref>
  states
    <ident>
    <ident>

  transition <ident>(<ident>, <ident>, ...)
    <ident> -> <ident>
    guard [not] <term> `<ident>` <term> (`<ident>` <term>)*
    assert <ident>
    retract <ident>
    assert <term> `<ident>` <value>
    retract <term> (`<ident>` <term>)* `<ident>` [<value>]
```

- `machine <node_ref>` — the node the machine is attached to, e.g.
  `machine door/12`.
- `states` — one or more bare state identifiers, one per line.
- `transition <ident>(<params>)` — a transition's name plus its
  parameter list (parens always present, may be empty: `transition
  reset()`). Body lines, any number, in any order: at most one `from ->
  to` pair, zero or more `guard` lines, zero or more `assert`/`retract`
  effect lines. Same rule as JSON's `TransitionInput`: needs at least
  one of a guard, a from+to pair, or an effect — an empty transition
  body is rejected.
- `guard [not] <pattern>` — `EXISTS(pattern)`, optionally negated.
  `<pattern>` is `anchor` `` `predicate` `` `term` repeated one or more
  times (at least one hop, same rule as JSON's `ExistsInput`) — the same
  infix-backtick idiom a plain fact's predicate application uses, e.g.
  `` self `opensWith` $key `` or a real multi-hop chain:
  `` self `adjacent` room/2 `opensTo` $torch ``.
- A pattern **term** is one of: `self` (the machine's own node, no
  payload); `$name` (a transition parameter reference); a **multi-
  segment** node reference like `key/13` (a literal node — must contain
  a `/`); or a bare, slash-free identifier, read as a pattern variable.
  **Real, named limitation**: a single-segment literal node term (e.g.
  a node genuinely named just `Room` with no further segment) is
  lexically indistinguishable from a pattern variable in this surface
  and is always read as a variable — write a real multi-segment
  reference if a literal single-word node is what's meant. JSON doesn't
  have this gap (`kind: "node"` vs `kind: "var"` is explicit there).
- `assert <ident>` / `retract <ident>` — the OLD, still-fully-supported
  sugar, always implicitly `(self, "state", <ident>)`. Use this when a
  transition is ONLY changing the machine's own state and nothing else.
- **`assert <term> \`<ident>\` <value>` / `retract <term> \`<ident>\` [<value>]`
  — the GENERAL form, added 2026-09-03.** An effect is no longer
  limited to the machine's own `self . state` — it can assert or
  retract ANY fact, on ANY subject term (`self`, `$param`, or a literal
  multi-segment node), the same infix-backtick shape a plain fact uses.
  `<value>` is whatever an ordinary fact's value can be: `self`/`$param`
  (a node, resolved at fire time), a literal multi-segment node, a
  quoted string, a bare number, or `true`/`false`.
  **This is also how a transition mints a brand-new node**: DMML is
  open-world (a node exists the instant any fact mentions it, no
  separate registry) — so an effect whose subject is `$name` and whose
  predicate/value assert real content brings a brand-new node into
  existence the moment that transition fires with a concrete binding
  for `$name`. A transition doesn't need a special "mint" keyword; an
  ordinary general-form assert on a fresh `$param` binding already does
  it. Real, worked example:
  ```
  transition forge(name, material)
    idle -> forged
    guard self `stocked` $material
    assert $name `title` "A freshly forged key"
    assert $name `madeFrom` $material
    assert forged
  ```
  Firing this with `$name` bound to a node nobody has ever mentioned
  before (e.g. `key/rusty42`) mints it for real — see
  `examples/fire-demo/` for the actual verified run.
  A retract in the general form still needs the SAME real-provenance
  discipline any retraction needs (see `app/FireTransition.hs`/
  `DMML.Fire`) — that's a firing-time concern for whoever runs
  `fire-transition`, not something an author needs to think about while
  just writing the machine text.
  **Retract's trailing `<value>` is optional** (added 2026-09-04, after
  a real eval found three independent free models writing one anyway by
  analogy with assert — `dev-journal/2026-09-04-complex-machine-eval.md`).
  It's accepted and carried through: when a fired retract has one, it
  renders into the produced `consumes`/`fact` citation's own
  pre-existing optional object position (the same `subject.predicate
  [= value]` shape a `consumes` block already has). **Made load-bearing
  the same day**: `DMML.Materialize`'s `consumes` application now
  removes ONLY the alternative matching the cited value, leaving every
  other live alternative for that key untouched — a value-less retract
  keeps the original wildcard behavior (removes every live alternative)
  and still refuses when there's more than one, since there's no
  principled way to pick just one without a value to match against.
  **Retract can also be CHAINED** (added 2026-09-04, jedelman/dmml#5):
  `retract self \`witnessedBy\` self \`at\` $eruption` mirrors a guard's
  own multi-hop pattern — real output a free model wrote unprompted for
  exactly this reason (same eval). Every hop gets independently walked
  and independently retracted at fire time (each becomes its own real
  `consumes`/`fact` entry, its own provenance, its own ambiguity check —
  see `DMML.Fire.resolveRetractHops`), so this is genuinely "undo the
  whole pattern I just checked," not just a longer single retraction.
  Firing ALSO now gates the whole resolved effect set (assert and
  retract alike) against every guard in the known machine set —
  `DMML.Retroconsistency.gateConsistentTree`, generalized the same day
  to check both guard polarities in one pass — so a chained retract that
  would silently strand some OTHER transition's guard elsewhere refuses
  instead of firing. See `examples/chained-retract-demo/` for a real,
  worked, verified run of both the chain itself and the gate catching a
  real cross-machine break.

## Example (machine)

```
machine door/12
  states
    locked
    unlocked

  transition unlock(key)
    locked -> unlocked
    guard self `opensWith` $key
    assert unlocked
```

Parses to the identical `MachineStmt` the equivalent
`MachineInput`-shaped JSON produces — verified directly, not assumed,
on this exact door/12 example (the same one `GRAMMAR.md` and
`MACHINE_SPEC.md` use).

A second, richer hand-authored example — `examples/shrine.dmml`, three
states, three transitions, a negated guard, a guard anchored on a
transition parameter rather than `self`, a two-hop chain with literal
node terms — exercises the parts of the grammar door/12 doesn't touch
at all. Written and parse-verified before any model saw the machine
grammar, specifically so the compliance checkpoint below wasn't testing
against the only example a model could have copied.

A third example, `examples/complex-demo/master.dmml`, exercises the
general effect form specifically: two transitions, two params each, a
guard chain spanning both `self` and a param-anchored subject, a
negated guard, general-form asserts on non-`self` subjects (minting a
brand-new node, `item/thorncrown`, purely by naming it as a param at
fire time), a literal string value, and two real `from -> to`
retractions in sequence. Fired live end to end — both commits it
produces pass real `validate-commit`/`check-declared` — not just
parsed and set aside.

## What's implemented

`src/DMML/Surface.hs` — a `megaparsec` parser, exposing both
`parseCommitSurface :: Text -> Either (ParseErrorBundle Text Void)
CommitStmt` and (added 2026-09-01) `parseMachineSurface :: Text ->
Either (ParseErrorBundle Text Void) MachineStmt`. Both verified by
parsing the worked examples above and diffing the result (spans
stripped) against what `DMML.FromJson.commitFromJson`/`machineFromJson`
build from the equivalent JSON — same `Ast.CommitStmt`/`MachineStmt`
value, two different front-ends, checked to actually agree rather than
assumed to. The machine parser's guard/pattern handling was also
checked against a second, harder example (a negated guard, a two-hop
pattern, a literal multi-segment node term, both `assert` and `retract`
in one transition) beyond the single worked example shown above.

## What's NOT done yet — real gaps, not implementation debt to ignore

- ~~No compliance checkpoint against light models yet.~~ **Done,
  2026-09-01: 15/15 accepted** (`dmml/compliance-surface/`, same three
  models and the same discipline as the JSON checkpoint — a real oracle,
  `ComplianceCheckSurface.hs`, calling `parseCommitSurface` directly).
  Matches the JSON checkpoint's own 15/15 exactly. Doesn't settle
  "Haskell-styled text is *better* than JSON for agents" (nothing here
  compares the two head-to-head on the same failure modes) — only that
  it's not *worse*, on this scenario set, against these three models.
  One real, checkpoint-scoped-only finding worth carrying forward: since
  this parser checks shape only (see below), a reply that uses a
  predicate without declaring it (observed once, Kimi) still scores
  "accepted" here — that gap won't be visible until self-declaration
  semantics exist to check against.
- ~~No `machine`, `reference`, or batching (`update`) support.~~
  **Machine authoring added 2026-09-01** — see the machine grammar
  section above. Still no `reference` or batching (`update`) support;
  JSON's `ReferenceInput`/`UpdateInput` have no surface-syntax
  counterpart.
- ~~No compliance checkpoint against light models for machine authoring
  yet.~~ **Done, 2026-09-01: 9/9 accepted**
  (`dmml/compliance-surface-machines/`, same three models). Scenarios
  deliberately exercised parts `examples/shrine.dmml` covers but
  door/12 doesn't — a negated guard, a guard anchored on a transition
  parameter instead of `self`, a two-hop chain — so this wasn't testing
  recall of the one example every model saw. Spot-checked substantively:
  all three models converged on identical, correct output for the
  non-`self`-anchor scenario, and `glm-5.3-flash` correctly explained
  *why* the literal node terms in its own reply weren't read as pattern
  variables (citing the `/`-disambiguation rule above) — real evidence
  it understood the rule, not just pattern-matched an example. Same
  "checked shape only, not self-declaration" caveat as the commit
  checkpoints applies here too.
- **No error-recovery/reporting design.** `megaparsec`'s default error
  messages are used as-is — JSON Pointer-style localized errors
  (`from_json.rs`'s whole design point) don't have an equivalent here.
- **No layout algorithm** — see "Design principles" above.
