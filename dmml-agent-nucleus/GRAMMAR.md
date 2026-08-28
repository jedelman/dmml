# DMML in one page

This is not the spec. It's the part of the spec you need to mint and cite
real commits. Source of truth is `dmml-runtime/src/graph.rs` in the sibling
`dmml` repo (the `Commit`, `ConsumeRef`, `StrongRef`, `FactRef` types) and
`written-world`'s `SPEC.md`, if you want the full thing.

## A commit is five fields

```
consumes:     [ConsumeRef]   -- what this commit depends on. Empty for a mint.
produces:     string          -- N-Quads text: the subgraph this commit asserts.
predicate:    string          -- an open verb naming the operation. "mints",
                                  "argues", "ruptures", whatever fits. Not a
                                  closed enum -- the vocabulary is deliberately
                                  open. Make one up if none fits.
via:          ConsumeRef?     -- optional: what authorized this commit, if
                                  anything did.
responds_to:  ConsumeRef?     -- set only when this commit is the accepting
                                  half of an exchange with another commit.
created_at:   string          -- ISO 8601 timestamp.
```

## `produces` is N-Quads, not JSON

Each line is one triple, dot-terminated:

```
_:some_subject <https://written-world.example/predicate/claim> "the actual content" .
```

The subject slug and predicate IRI are yours to choose. Multiple lines in
one `produces` field assert multiple triples in one commit.

## `consumes` is a list of two possible reference shapes

**`StrongRef`** — cites a whole other record by its real, resolved address:

```json
{"uri": "at://did:plc:.../collection.nsid/rkey", "cid": "bafyrei..."}
```

**`FactRef`** — cites one specific triple inside another commit's
`produces`, for finer-grained retraction or supersession:

```json
{"commit": {"uri": "...", "cid": "..."}, "subject": "...", "predicate": "...", "object": "..." or null}
```

`object: null` means "every triple this commit asserted for that
subject+predicate" — a wildcard, not "no object."

## What a citation actually guarantees, and what it doesn't

A `consumes` entry is a claim of dependency, not a claim of truth. The
runtime (`WorldGraph::apply_commit`) checks that the cid you're citing was
**previously recorded as observed** — it does not re-fetch, re-verify, or
execute anything the citation points at. Citing a real cid you never
checked is technically valid and substantively dishonest. Every real
checkpoint script in `dmml-substrate-kit/` spot-verifies at least one
citation against the live PDS before treating a run as done. Do that too.

## The one thing this grammar will never do for you

DMML has no privileged notion of "how commits are meant to be dispatched,
ordered, or ratified." That's not a missing feature. A `produces` field can
assert anything, including a claim that describes a protocol for reading
other commits — but that claim is exactly as authoritative as any other
claim in the graph: none. If a harness wants to *try* honoring what some
commit says about how to interpret other commits, that's the harness's own
convention, applied at its own discretion, never something `apply_commit`
enforces. See `README.md`'s point about self-assembly, and `FORKING.md` for
how actual executable governance logic (which can't and shouldn't live in
this grammar) gets referenced instead.
