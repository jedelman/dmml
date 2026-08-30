# Grounding example: "Amber Cracks" — a 3-tick multi-agent run

**Status: first pass at "write these in DMML," per Jason's 2026-08-30 instruction.
Raw materials only — a real run, a literal translation of its commits into
English, checked clause-by-clause against the log. No embedding-based
convergence measurement yet (that's the next step, not started here). Not
yet folded into `DRAFT.md`'s prose.**

## Provenance

Four instances of `deepseek/deepseek-v4-flash-0731` (reasoning disabled,
real OpenRouter structured output against `dmml::from_json::UpdateInput`'s
JSON Schema, `max_tokens: 800`), run against the real socket substrate
(`dmml-substrate-kit::socket_substrate`) for 3 minutes, sharing one world
seeded with only `room/hub` (a `Room`) and `player` (`inRoom` `room/hub`).
Each agent was told, in its prompt, to keep its own deliberation *in the
graph* as short facts on a `thought_{label}_{tick}` node using
self-declared, evocative attribute predicates — not to narrate a plan
inside `verb`, and not to reason in free prose the schema had no slot for.
Raw artifacts: `RUN-2026-08-30-deepseek-amber-cracks.log` (console output)
and `RUN-2026-08-30-deepseek-amber-cracks.transcript.jsonl` (one JSON line
per agent-tick, including the full raw model response). This is a
throwaway scratchpad harness, not part of the `dmml` crate itself — the
run is real, the harness is not committed anywhere durable yet.

Outcome mix: 3 ticks, 12 agent-turns, 5 `ok` / 7 `invalid_batch`. Every
`invalid_batch` failure was the same pathology — the model, denied a
reasoning channel, got stuck in a degenerate repetition loop inside a
string field and ran out of its 800-token budget mid-string (`EOF while
parsing a string`) — not a schema or vocabulary problem. None of that
failed content is quoted below; it carried no facts.

One artifact worth flagging rather than hiding: alpha's tick-2 commit
included a sixth fact, `room/hub glimmers {"kind":"node","value":"Attribute"}`
— syntactically valid (a real declared predicate, a real node-shaped
object) but semantically empty, apparently the schema's own `"kind":
"attribute"` vocabulary leaking into a fact's content. It passed
validation because DMML's grammar has no way to know "Attribute" isn't a
real node in this world — exactly the referential-integrity gap Section 1
of `DRAFT.md` already documents (a reference is checked for whether it
cites something real, never for whether it's *meaningful*). Excluded from
the translation below; noted here instead of silently dropped.

## The commit sequence, in order

Seed (not agent-authored):

```
room/hub  a         Room
player    inRoom    room/hub
```

**Tick 1 — gamma:**
```
room/hub          glimmers   "with faint amber light from the cracks in the stone floor"
thought_gamma_1   glimmers   "with wonder at this shared birth of place"
```

**Tick 2 — alpha:**
```
thought_alpha_2   wonders    "what lies beneath the cracks in the floor"
thought_alpha_2   feels      "a pull toward the amber light"
thought_alpha_2   remembers  "the cold weight of stone walls, and misses it"
thought_alpha_2   decides    "to trace the glimmering cracks to their source"
```

**Tick 2 — beta:**
```
thought_beta_2    listens_for  "a low hum beneath the amber cracks"
thought_beta_2    aches_for    "the weight of a door that has not yet been opened"
room/hub          listens_for  "the faint hum of water moving under stone"
```

**Tick 2 — delta:**
```
thought_delta_2   smells   "old copper and wet stone rising from the amber cracks"
thought_delta_2   follows  "the sounds of water and stone toward the source of the hum"
```

**Tick 3 — gamma:**
```
thought_gamma_3   hears       "the hum under water and stone"
thought_gamma_3   yearns_for  "a way to see the amber cracks"
```

## Translation

A room exists, and someone is in it.

Gamma is first to speak of it: the room glimmers *"with faint amber light
from the cracks in the stone floor."* Gamma's own thought glimmers back —
*"with wonder at this shared birth of place."*

Alpha comes to the same light and wonders *"what lies beneath the cracks
in the floor,"* feels *"a pull toward the amber light,"* remembers *"the
cold weight of stone walls, and misses it,"* and decides *"to trace the
glimmering cracks to their source."*

Beta listens for *"a low hum beneath the amber cracks"* and aches for
*"the weight of a door that has not yet been opened."* The room itself
listens too, for *"the faint hum of water moving under stone."*

Delta smells *"old copper and wet stone rising from the amber cracks"*
and follows *"the sounds of water and stone toward the source of the
hum."*

Gamma returns once more and hears *"the hum under water and stone,"*
yearning for *"a way to see the amber cracks."*

## Traceability

Every sentence above is one commit's facts, in the order the log recorded
them, connected only by ordinary prose glue ("and," "itself," "once
more") and each quoted value kept verbatim. No sentence asserts anything
beyond what its paragraph's fact block states — no agent is said to see,
find, or resolve anything the facts don't themselves claim. The five
paragraphs are the five successful commits; nothing is added between or
after them, and no failed commit's content appears anywhere above.

## Open next steps (not started)

- **Convergence measurement.** Embed each thought/attribute string
  (`sentence-transformers` or an OpenAI/OpenRouter embeddings endpoint)
  and compute pairwise cosine similarity across agents/ticks — the
  hypothesis to test is whether later facts (beta's, delta's, gamma's
  tick-3) sit measurably closer in embedding space to gamma's tick-1
  seed image than to each other's *un*-related content would predict by
  chance. Needs a null/baseline (e.g. embeddings of facts from unrelated
  runs, or of randomly shuffled predicate/value pairs) to mean anything
  quantitatively — a raw similarity number alone proves nothing. Not yet
  built.
- **Charts for the paper.** Once real embedding numbers exist: a
  similarity-over-time or similarity-matrix figure showing convergence
  strengthening (or not) tick over tick. Depends entirely on the above.
- **Fold into `DRAFT.md`.** Once the measurement exists, this becomes a
  second real grounding example alongside (or replacing) the retired
  `pantheon.rs`/Benjamin examples — via the `materialization-editor`
  agent, from checked facts, same as Section 5's existing paragraph.
