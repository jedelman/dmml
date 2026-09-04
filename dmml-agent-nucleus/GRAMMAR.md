# DMML in one page

This is not the spec. It's the part of the spec you need to mint and cite
real commits. **Source of truth is `dmml-hs` in the sibling `dmml`
repo** — `src/DMML/Json.hs`/`src/DMML/FromJson.hs` (the JSON `*Input`
types and the `*FromInput`/`*FromJson` functions that build the real AST
from them) for JSON authoring, and `src/DMML/Surface.hs` (grammar
documented informally in `dmml-hs/SURFACE.md`) for DMML's OTHER real
authoring surface, a text grammar — this is not the older hand-written
text surface `written-world`'s `SPEC.md` §10 describes and retired; that
one is still gone. `DMML.Surface` is a new, different text grammar,
targeting the exact same validated AST the JSON front-end builds ("one
AST, two front-ends" — the two are checked to actually agree, not
assumed to). If this page and the code ever disagree, the code wins;
open an issue against this page rather than trusting it.

**RETIRED, 2026-09-04**: the Rust `dmml`/`dmml-runtime` crates (also in
this repo) are no longer canonical. `dmml-hs` diverged from them for
real, useful reasons this same day (generalized effects, a chained
retract, real execution/firing semantics neither implementation had
before) and nothing forced the two back into agreement — rather than
port the divergence backward, the call was to make the diverged,
further-along implementation the real one. The Rust crates still exist
in this repo for now; treat them as historical, not as something a new
fork should build against. (`written-world`'s `server`/`client`/`cli`/
`appview` packages still have live git dependencies on them as of this
writing — that's a real, separate migration, not yet done; see
`dev-journal/2026-09-04-dmml-hs-canonical.md`.)

## Authoring is JSON or text in, AST out — nothing else

JSON: three single-item entry points, one batching entry point
(`DMML.FromJson`):

```haskell
commitFromJson    :: Text -> Either FromJsonError CommitStmt
machineFromJson   :: Text -> Either FromJsonError MachineStmt
referenceFromJson :: Text -> Either FromJsonError ReferenceStmt
updateFromJson    :: Text -> Either UpdateFromJsonError Update   -- batches; see below
```

Text (`DMML.Surface`), the same target AST, a different front door:

```haskell
parseCommitSurface  :: Text -> Either (ParseErrorBundle Text Void) CommitStmt
parseMachineSurface :: Text -> Either (ParseErrorBundle Text Void) MachineStmt
```

Either way you get back either the AST node or a real error naming
exactly what was wrong and where — a JSON Pointer (RFC 6901, e.g.
`/facts/2/predicate`) for the JSON front-end, a line/column for the text
one.

## A commit is `verb` + `declares` + `facts` + `consumes` + `refs`

```json
{
  "verb": "mints",
  "declares": [{"kind": "relation", "name": "opensTo"}],
  "facts": [
    {"subject": "room/42", "predicate": "a", "object": {"kind": "node", "value": "Room"}},
    {"subject": "room/42", "predicate": "opensTo", "object": {"kind": "node", "value": "room/43"}},
    {"subject": "room/42", "predicate": "dampness", "object": {"kind": "number", "value": "0.4"}}
  ],
  "consumes": [],
  "refs": {}
}
```

- **`verb`** — required, an open vocabulary (`mints`, `argues`, `ruptures`,
  whatever fits). Just a bare identifier: letter-led, then
  letters/digits/underscore. Never validated against a closed enum.
- **`declares`** — `{"kind": "relation" | "attribute", "name": "..."}`.
  Declares a predicate before facts in this batch use it (see
  self-declaration ordering below).
- **`facts`** — `{"subject": "...", "predicate": "...", "object": {...}}`.
  `subject` and `predicate` are plain strings, not decomposed structs.
  `predicate` is either `"a"` (Turtle-style sugar for `rdf:type`) or a bare
  identifier. This is the *only* JSON shape — there's no separate
  `produces { }` block form; a bare fact in `facts` is exactly the same
  thing an explicit block would have meant.
- **`object`** is tagged by `kind`, one discriminant, always:
  - `{"kind": "node", "value": "room/43"}` — a node reference
  - `{"kind": "str", "value": "..."}` — a string literal
  - `{"kind": "number", "value": "0.4"}` — a number literal, **as a string**
  - `{"kind": "boolean", "value": true}` — a real JSON boolean
- **`consumes`** — see below. Empty for a mint.
- **`refs`** — role-tagged commit-level references, replacing what used to
  be separate `via`/`responds_to` fields: `{"via": [strongRef, ...],
  "respondsTo": [strongRef, ...], "requires": [...]}`. Every role is a
  list, keyed by an open role name — a new role needs no schema change.
  Omit the whole field, or a given role, for none.

A commit needs at least one of `facts`, `consumes`, or a non-empty `refs`
— an empty commit is rejected.

## Node references and identifiers

`subject`, `object.value` (when `kind: "node"`), and `MachineInput.node`
all share one grammar: `segment ( "/" segment )*`, where `segment =
seg_piece ( "." seg_piece )*` and `seg_piece` is either a bare identifier
or a digit-only run. So `room/42`, `key/7`, and `room/42.reach` are all
valid; a leading digit is fine in a segment, a leading `-` or decimal
point is not (that's a number literal, not a node ref).

Bare identifiers (`verb`, `declares[].name`, non-`"a"` predicates, guard
hop predicates, machine state/transition/param idents) are letter-led,
then letters/digits/underscore only.

## `declares` ordering — you don't have to declare-before-assert

The validator (`DMML.SelfDeclaration.undeclaredPredicates`) is two-pass:
it collects every `declares` entry (and every `rdf:type` fact) across
the whole batch first,
*then* checks every other predicate against that full set. So order within
a commit's own JSON doesn't matter — declare-after-use in the same commit
is fine, same batch is fine. What isn't fine is using a predicate nobody
in the batch ever declares.

## `consumes` — two reference shapes, one discriminant (`kind`)

**`{"kind": "strong", "uri": "...", "cid": "..."}`** — cites a whole other
commit by its real, resolved address. No `at://` scheme is assumed or
required by the parser itself — `uri` just has to be non-empty; whatever
scheme your substrate actually resolves is fine.

**`{"kind": "fact", "commit": {"uri": "...", "cid": "..."}, "subject": "...", "predicate": "...", "object": {...} }`**
— cites one specific triple inside another commit, for finer-grained
retraction or supersession. `object` is **optional** — omit it entirely
(never send `null`) for the wildcard: every triple that commit asserted
for `(subject, predicate)`. Sending `object` is genuinely load-bearing
(fixed 2026-09-04): only the one live value matching it gets removed,
every OTHER independently-asserted value for the same `(subject,
predicate)` survives untouched — omitting `object` still removes
everything for that pair, the original wildcard behavior, unchanged.

## `refs` values are the same `{uri, cid}` shape as `consumes`'s strong form

```json
"refs": {"via": [{"uri": "...", "cid": "..."}]}
```

Deliberately absent from every JSON shape above: a `created_at` field.
There is no DMML syntax for "the time this compiles" — the authoring tool
stamps that at commit time, an author never writes it.

## Machines — states, transitions, guards, effects

```json
{
  "node": "player",
  "states": [{"ident": "locked"}, {"ident": "unlocked"}],
  "transitions": [{
    "ident": "unlock",
    "params": ["key"],
    "from": "locked",
    "to": "unlocked",
    "guards": [{
      "negated": false,
      "exists": {
        "anchor": {"kind": "self"},
        "hops": [{"predicate": "holds", "term": {"kind": "param", "value": "key"}}]
      }
    }],
    "effects": [{"kind": "assert", "ident": "unlocked"}]
  }]
}
```

- A `pattern` term (`anchor`, and each hop's `term`) is tagged by `kind`:
  `"self"` (no payload — always means the machine's own node), `"param"`,
  `"var"`, or `"node"` (the last three carry `value`).
- A guard is `{"negated": bool, "exists": {"anchor": ..., "hops": [...]}}`
  — `exists` needs at least one hop. There's exactly one guard primitive,
  `EXISTS(pattern)` plus `negated`; there's no bare-predicate-call guard
  form.
- A transition needs at least one of: a guard, a `from`+`to` pair, or an
  effect.
- Full grammar, both fronts: `dmml-hs/SURFACE.md` (the text surface's own
  informal grammar, worked examples, and everything below in more
  detail).

### Effects — generalized 2026-09-03/04, this is the part that actually changed

The bare `{"kind": "assert" | "retract", "ident": "..."}` shape from
before still works exactly as it always did — sugar for
`(self, "state", <ident>)`, unchanged, use it for a transition that only
changes its own state. But an effect is no longer LIMITED to that:

```json
{"kind": "assert", "subject": {"kind": "param", "value": "name"},
 "predicate": "title", "value": {"kind": "str", "value": "A freshly forged key"}}
```

- **General assert**: `{"kind": "assert", "subject": <term>, "predicate": "...", "value": <value>}`.
  `subject` can be `self`, `$param`, or a literal node — an effect can now
  assert (or retract) a fact about ANY node, not just the machine's own
  state. `value` is the same shape a fact's `object` already is.
- **This is also how a transition mints a brand-new node**: DMML is
  open-world — a node exists the instant any fact mentions it, no
  separate registry — so an effect whose `subject` is `$name` and whose
  `predicate`/`value` assert real content brings a brand-new node into
  existence the moment the transition fires with a real binding for
  `$name`. No separate "mint" primitive.
- **General retract**: `{"kind": "retract", "subject": <term>, "hops": [<hop>, ...], "predicate": "...", "value": <value>}`
  (`hops` and `value` both optional, default `[]`/absent). Zero hops is
  the ordinary single-fact case. One or more `hops` (same shape as a
  guard's own — `{"predicate": "...", "term": <term>}`) makes it a
  CHAINED retract, mirroring a guard's own multi-hop pattern: walks
  `subject` through each hop, retracting every real edge it walks, then
  retracts the terminal `predicate`/`value` from wherever the walk
  landed. Real motivating case: a transition undoing exactly the
  multi-hop fact its own guard just checked.
- **`value` is genuinely load-bearing for a retract**, not cosmetic: with
  a value, only the ONE live alternative matching it gets removed —
  every other value asserted for the same (subject, predicate) survives.
  Without one, the old wildcard behavior applies (every live alternative
  removed) and REFUSES if there's more than one, since there's no
  principled way to pick just one without a value to match against.

### Firing — new capability, neither implementation had this before today

Everything above is still just AUTHORING a machine — declaring what
COULD happen. Given a machine, a transition name, real parameter
bindings, and the current materialized world, `DMML.Fire.fireTransition`
actually resolves a legal transition's effects to concrete facts and
renders them as a real, ordinary commit — the same real
`validate-commit`/`check-declared` pipeline any hand-authored commit
goes through applies it, nothing is mutated silently. A retract effect's
citation needs real commit provenance (a `uri`/`cid` the world was
materialized with, not just a branch label) to know what it's actually
consuming; the CLI, `fire-transition`, computes this automatically for
every world file it's given. Firing also gates the WHOLE resulting
change (not just the transition's own guards) against every OTHER
guard in a known machine set — a firing that would silently strand some
unrelated transition's guard elsewhere refuses instead. See
`dmml-hs/SURFACE.md` and `dmml-hs/examples/*-demo/` for real, worked,
verified runs of all of this.

## Batching many commits/machines in one call — `updateFromJson`

```json
{"update": [
  {"commits": [ /* CommitInput, ... */ ], "machines": [ /* MachineInput, ... */ ]},
  {"commits": [ /* another batch */ ]}
]}
```

`update` is an ordered list of **batches**. This is the one place order
*does* matter, and it's the whole point of the shape:

- Commits **within** one batch are simultaneous — a duplicate
  `(subject, predicate)` across two commits in the same batch is
  rejected, same as a self-collision inside one commit. This is a real
  bug the shape exists to catch: a model once split `player/1 holds
  key/1` into one commit and `player/1 holds key/2` into a sibling commit
  in the same batch, silently dropping `key/1` at materialization.
- Commits **across** separate batches are sequential — a later batch's
  commit legitimately overwriting an earlier batch's fact for the same
  `(subject, predicate)` is correct append-only-log behavior, not an
  error.

A single commit is just a batch of one, in a sequence of one:
`{"update": [{"commits": [c]}]}`.

Unlike the single-item `*FromJson` functions, `updateFromJson` never
fails fast — every commit and machine in every batch is built
independently and every failure collected, with each error's pointer
rebased onto the whole update (`/update/1/commits/3/facts/1/subject`).
This governs validation only, not application — `DMML.Materialize.
applyCommit`/`applyIdentifiedCommit` still take one commit at a time, in
caller-supplied order; concurrent application of a batch isn't
implemented.

## What a citation actually guarantees, and what it doesn't

A `consumes` entry is a claim of dependency, not a claim of truth, and
right now — checked directly against `dmml-hs` while writing this, not
carried over from the retired Rust crate — it's a WEAKER claim than this
page used to say: `DMML.Materialize.applyConsume` never checks that the
`uri`/`cid` you're citing was previously recorded as observed at all. It
matches purely on `(subject, predicate[, object])` and removes whatever
it finds; a citation naming a `cid` nobody has ever actually seen is
currently accepted exactly the same as a real one. (The retired Rust
crate's `graph.rs` did check the cid half of a `consumes` citation
against what it had actually recorded — that check did not come along
in the port to `dmml-hs`; tracked as jedelman/dmml#6.)
Citing a `cid` you never checked was always substantively dishonest
regardless of what the runtime enforces; it's now ALSO not something to
rely on the runtime catching for you. Every real checkpoint script in
`dmml-substrate-kit/` spot-verifies at least one citation against the
live PDS before treating a run as done. Do that.

## The one thing this grammar will never do for you

DMML has no privileged notion of "how commits are meant to be dispatched,
ordered, or ratified." That's not a missing feature. A commit's facts can
assert anything, including a claim that describes a protocol for reading
other commits — but that claim is exactly as authoritative as any other
claim in the graph: none. If a harness wants to *try* honoring what some
commit says about how to interpret other commits, that's the harness's own
convention, applied at its own discretion, never something
`DMML.Materialize.applyCommit` enforces. See `README.md`'s point about self-assembly, and `FORKING.md` for
how actual executable governance logic (which can't and shouldn't live in
this grammar) gets referenced instead.
