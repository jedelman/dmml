# Informed-vs-blind authoring checkpoint

Tests the actual claim behind "hand agents a materialized subset of the
world": does real world-state context prevent **predicate-name drift**
— two different agents independently naming the same relationship two
different ways, a coherence failure no syntax validator catches, since
both names parse fine on their own.

## Method

`dmml-hs/src/DMML/Materialize.hs` — a minimal materializer (real, but
deliberately not a port of `interpret::Materialized`: no cid/uri
provenance, no `reachable_from` graph-scoping, `consumes` handled
operationally rather than resolved against a real citation graph). It
applies a sequence of `CommitStmt`s and renders "declared predicates" +
"current facts, latest value per (subject, predicate)" as plain text.

`examples/shrine-genesis.dmml` (hand-authored, parse-verified) seeds a
small world: the shrine accepts incense via a relation named `accepts`,
among other declared predicates and facts.

The same task — author a commit recording that the shrine accepts
incense — was dispatched twice per model: once **blind** (no world
context at all, has to invent a predicate name), once **informed** (the
real rendered snapshot in the system prompt, told to reuse an existing
predicate if one already covers the relationship).

## Result

| condition | distinct predicate names chosen | matched existing `accepts` |
|---|---|---|
| blind | 2 (`accepts`, `acceptsOffering`) | 1/3 by chance, 2/3 drifted |
| informed | 1 (`accepts`) | 2/2 that produced content |

**Real drift confirmed in the blind condition** — gemini-3.7-flash and
kimi-k2.5 both independently invented `acceptsOffering` instead of
`accepts`; only glm-5.3-flash happened to land on the same name a
different agent had already used. **Zero drift in the informed
condition** — both replies that produced content matched the existing
name exactly.

## The finding this checkpoint almost missed by scoring too narrowly

Kimi's *informed* reply wasn't malformed DMML — it was **no DMML at
all**: *"The relation `accepts` already exists in the world state, and
the fact `shrine/threshold . accepts = offering/incense` is already
present. No new commit is needed."* This is correct. The scenario task
was accidentally already fully satisfied by `shrine-genesis.dmml`'s own
seed content — a real design flaw in this checkpoint's scenario, not a
model failure. Kimi read the snapshot, recognized the task was already
done, and declined to author a redundant duplicate — arguably the
*most* semantically correct response of the six dispatches. The
Haskell oracle here only knows how to score "did a fenced DMML block
parse," so it recorded this as `rejected`. It shouldn't read as a
failure in the report table without this context attached, and a
better-designed follow-up scenario would ask for something the seed
genuinely doesn't already contain.

## What this does and doesn't show

Small sample (6 dispatches, one relationship, one seed world) — this is
a real, checked data point that materialized context measurably reduces
predicate-name drift, not a general proof it eliminates the class of
problem. It also surfaced, for free, that an informed agent can produce
a *correct non-answer* a narrow parse-only oracle can't credit — worth
remembering before trusting any future "N/M accepted" number from a
checkpoint like this one without reading what the rejections actually
say.
