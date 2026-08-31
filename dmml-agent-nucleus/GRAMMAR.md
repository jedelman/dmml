# DMML in one page

This is not the spec. It's the part of the spec you need to mint and cite
real commits. Source of truth is `dmml/src/from_json.rs` in the sibling
`dmml` repo — the JSON `*Input` structs and the `*_stmt_from_input`
functions that build the real AST from them. `written-world`'s `SPEC.md`
§10 has design history and rationale, but its own EBNF there describes an
older hand-written text surface that was retired — DMML has no text
grammar and no lexer anymore. **JSON is the only authoring surface.** If
this page and the code ever disagree, the code wins; open an issue against
this page rather than trusting it.

## Authoring is JSON in, AST out — nothing else

Three single-item entry points, one batching entry point:

```
commit_from_json(json: &str)    -> CommitStmt
machine_from_json(json: &str)   -> MachineStmt
reference_from_json(json: &str) -> ReferenceStmt
update_from_json(json: &str)    -> Update   (batches of the above; see below)
```

Each takes a raw JSON string (not a pre-parsed value) and returns either
the AST node or a `FromJsonError` naming exactly which JSON Pointer
(RFC 6901, e.g. `/facts/2/predicate`) was wrong and why.

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

The validator (`validate_self_declared`) is two-pass: it collects every
`declares` entry (and every `rdf:type` fact) across the whole batch first,
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
for `(subject, predicate)`.

## `refs` values are the same `{uri, cid}` shape as `consumes`'s strong form

```json
"refs": {"via": [{"uri": "...", "cid": "..."}]}
```

Deliberately absent from every JSON shape above: a `created_at` field.
There is no DMML syntax for "the time this compiles" — the authoring tool
stamps that at commit time, an author never writes it.

## Machines — states, transitions, guards, effects: also just JSON now

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
- An effect is `{"kind": "assert" | "retract", "ident": "..."}` — always
  implicitly `(self, "state", <ident>)`.
- A transition needs at least one of: a guard, a `from`+`to` pair, or an
  effect.
- Full semantics: `dmml/MACHINE_SPEC.md` in the `dmml` repo.

## Batching many commits/machines in one call — `update_from_json`

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

Unlike the single-item `*_from_json` functions, `update_from_json` never
fails fast — every commit and machine in every batch is built
independently and every failure collected, with each error's pointer
rebased onto the whole update (`/update/1/commits/3/facts/1/subject`).
This governs validation only, not application — `WorldGraph::apply_commit`
still takes one commit at a time, in caller-supplied order; concurrent
application of a batch isn't implemented.

## What a citation actually guarantees, and what it doesn't

A `consumes` entry is a claim of dependency, not a claim of truth. The
runtime checks that the cid you're citing was **previously recorded as
observed** — it does not re-fetch, re-verify, or execute anything the
citation points at. Citing a real cid you never checked is technically
valid and substantively dishonest. Every real checkpoint script in
`dmml-substrate-kit/` spot-verifies at least one citation against the live
PDS before treating a run as done. Do that too.

## The one thing this grammar will never do for you

DMML has no privileged notion of "how commits are meant to be dispatched,
ordered, or ratified." That's not a missing feature. A commit's facts can
assert anything, including a claim that describes a protocol for reading
other commits — but that claim is exactly as authoritative as any other
claim in the graph: none. If a harness wants to *try* honoring what some
commit says about how to interpret other commits, that's the harness's own
convention, applied at its own discretion, never something `apply_commit`
enforces. See `README.md`'s point about self-assembly, and `FORKING.md` for
how actual executable governance logic (which can't and shouldn't live in
this grammar) gets referenced instead.
