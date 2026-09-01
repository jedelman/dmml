# World assembly — three agents, one real chain, no shared static context

The actual standing challenge this whole thread has pointed at: can
light models assemble a coherent world, not just author one valid
commit in isolation? Three different models, in sequence, each handed
the REAL materialized snapshot of everything authored so far — genesis
plus every prior agent's actually-accepted output. Not three copies of
the same static context: step 2's prompt is generated *after* step 1's
real reply is parsed, validated, and folded into the world.

## Seed world

`dmml-hs/examples/hearth-genesis.dmml` — a small commons: a hearth, a
shrine, a forge, an archive, a keeper NPC, connected by `locatedIn` and
`tends`. Hand-authored, and it caught a real bug on first materialize
(see below) before any model ever saw it.

## The chain

1. **gemini-3.7-flash**: mint a wandering scribe, locate them at the
   hearth using the existing `locatedIn` relation.
2. **glm-5.3-flash**: record that the scribe (name given nowhere in the
   task text — only in the materialized snapshot from step 1) now
   tends the forge, reusing the existing `tends` relation.
3. **kimi-k2.5**: update the forge's `state` from `cold` to `banked` —
   an ordinary sequential overwrite, no `consumes` needed.

## Result: 3/3 accepted, real coherence, not just parse validity

```
commit mints
  npc/scribe :: a Scribe
  npc/scribe `locatedIn` commons/hearth
```
```
commit scribeTendsForge

  npc/scribe . tends = forge/9
```
```
commit updates
  forge/9 . state = "banked"
```

Every claim below is checked against `results/snapshot-final.txt`, not
asserted:

- **Zero predicate-name drift.** Neither `locatedIn` nor `tends` was
  redeclared by any of the three agents — each reused the exact name
  from the snapshot it was handed.
- **Real cross-agent reference resolution.** GLM's commit references
  `npc/scribe` — a name Gemini chose, that appears nowhere in GLM's task
  text, only in the world snapshot GLM was handed. This is the one
  thing static shared context can't test: information actually flowing
  from one agent's output into the next agent's input.
- **Correct sequential-overwrite semantics.** Kimi's `state` update is a
  fresh assertion, no `consumes` block — correct, since `state` isn't
  being retracted from a specific cited commit, it's an ordinary later-
  batch overwrite, exactly the case `UpdateInput`'s own doc comment
  distinguishes from same-batch duplication.
- **Final world is fully self-consistent** — every node either seeded
  or minted is real, every relation used was declared exactly once (in
  genesis), no orphaned references.

## A real bug this run found before any model touched it

`hearth-genesis.dmml`'s first draft asserted
`` commons/hearth `opensTo` X `` three times in one commit (to the
shrine, forge, and archive). `DMML.Materialize`'s single-value-per-
(subject, predicate) model — the same rule `from_json.rs`'s own
duplicate-fact check enforces — silently kept only the last one.
**`DMML.Surface` had no equivalent check at all** and accepted the
buggy commit without complaint; the bug only surfaced once the
snapshot was rendered and `opensTo` showed one target instead of three.
Fixed two things, not one: the genesis content itself (dropped the
redundant `opensTo` lines — each place's own `locatedIn commons/hearth`
already carries the relationship from the other direction) and the real
gap in `DMML.Surface` (`checkNoDuplicateFacts`, now rejects this before
it can happen again — see the commit fixing that). A hand-authored
example file finding a real parser gap is exactly the kind of thing a
"write it yourself first" discipline is supposed to catch.

## What this does and doesn't show

Three steps, three models, one linear chain, one small world — this is
a real, checked demonstration that materialized context enables
genuine cross-agent coherence, including reference resolution neither
static context nor a blind baseline could produce. It is not: a test of
concurrent/conflicting edits (every step here was strictly sequential,
no two agents ever saw the same snapshot), not a test of the
`consumes`/retraction path in a multi-agent setting, and not a stress
test at any real scale (a handful of nodes, three turns). The peer-to-
peer git broker's own open question — what happens when two agents
authored against the *same* snapshot and now conflict — is still
exactly as open as `sync-spike/README.md` left it.
