# A real model eval: the Vala design tier

**Status: raw findings, not yet folded into `DRAFT.md`. Kept here as
grounding material, same discipline as `GROUNDING-2026-08-30-amber-
cracks.md`.**

## The thesis this grounds (Jason, 2026-08-30, stated as the actual
## paper thesis -- record verbatim, don't paraphrase away the force of it)

> "prose is not what we want, structure is. no logic should live in
> prose - it's decorative. it should live in the structure."

Round 4 below is the concrete, checkable proof of exactly this claim on
a real task: the SAME model, at the SAME reasoning effort, moved from
0/5 convergence to 1/1 one-shot convergence purely by relocating one
piece of logic (`has_content`) out of prose (a prompt warning, a
validator error message) and into the schema's own structure (a real
`anyOf` of required-shaped branches). Nothing about the model changed.
Nothing about the reasoning effort changed. Only where the constraint
*lived* changed. That is the thesis, demonstrated, not just argued.

## The task

`valar_mint.py` dispatches a distinct "Vala" agent -- reasoning left on
(unlike the fast/cheap acting-agent harness elsewhere this session,
which explicitly disables it) -- asked to design ONE new DMML machine
extending Valinor's existing world (terrain -> quarry material chain ->
mortar/wall two-input joins -> forest/carpentry negated guard -> roof).
Not "operate the world," but "shape it": propose real states,
transitions, guards, grounded in what already exists, favoring real
cross-node or `$param` consumption over self-only narration. Output
checked for real against `dmml::from_json::update_from_json` via
`cargo run -p dmml --example validate_machines`, never trusted on its
own say-so.

## Four attempts, three models

| Model | Result |
|---|---|
| `deepseek/deepseek-v4-flash-0731` (attempt 1) | Invalid. `Valinor/house`, one transition, `{"to": "built"}` only -- no `from`, no guards, no effects. Reasoning trace considered pottery/tools/farm/house but never encoded any precondition it discussed. |
| `deepseek/deepseek-v4-flash-0731` (attempt 2, tightened prompt) | Reasoning alone ran ~50,000 chars across 4 retries and exhausted the 12,000-token budget every time -- zero content returned. The prompt fix ("every precondition you reason about must become a guard") made it reason far more carefully, with no room left to answer. |
| `deepseek/deepseek-v4-flash-0731` (attempt 3, `max_tokens: 24000`) | Valid JSON, `{"update": []}` -- an empty, technically-conformant non-answer. Reasoning trace shows it going in circles re-deriving its own understanding of the existing world rather than converging, then punting. |
| `z-ai/glm-5.3` (this project's own dispatch notes recommend it specifically for "genuinely intricate design work") | Invalid, and worse than deepseek's first attempt: `Valinor/lamp`, three transitions, all `{"to": ...}` only. Introduced ungrounded new materials (`glass_source`, `crystal_source`) never mentioned anywhere in the world description, ignoring the explicit instruction to extend the existing chain. No reasoning trace was returned by this endpoint at all -- a real observability gap, not assumed away. |
| `openai/gpt-5.2-pro` | Invalid as submitted, but for different, shallower reasons -- see below. **The design itself was correct.** |

## GPT-5.2-Pro: a different quality of result

Its reasoning trace shows real, grounded design work: it explicitly
identified that "consumption" in DMML has no dedicated machine-level
primitive and has to be expressed through guards (an accurate
observation about the actual grammar), then designed `Valinor/kiln`
(built from brick + mortar -- a real two-input join, same shape as
`wall.rs`'s) feeding `Valinor/pottery` (raw -> shaped via clay + water,
shaped -> fired via the kiln) -- a coherent, genuinely new extension of
the existing brick/mortar economy, not a decorative reskin.

It still failed validation, on two concrete, shallow bugs:

1. **Wrapped `"update"` as an object** (`{"commits": [], "machines":
   [...]}`) instead of the required array-of-batches shape (`[{"commits":
   [], "machines": [...]}]`) -- a real schema-adherence miss even with
   OpenRouter's native `structured_outputs` support declared for this
   model.
2. **Guessed the wrong guard predicate** -- `"a"`/rdf:type instead of
   `"state"`. Its own reasoning trace shows it explicitly agonizing over
   exactly this ambiguity ("I wonder if DMML records machine state as
   rdf:type or 'state'?"). This one is partly `valar_mint.py`'s own
   fault: the world description given to every Vala never states the
   guard-predicate convention explicitly, only implies it through
   examples that don't disambiguate `a` from `state`.

Both were fixed by hand (`VALAR-MINTED-2026-08-30-openai-gpt-52-pro-
FIXED.json`) -- wrap the batch in an array, replace `"a"` with `"state"`
in guard hops, nothing else touched. **It validated clean on the first
try after that**, and `dmml/examples/kiln.rs` fires the whole design
end to end against a real, live world: build the kiln from brick +
mortar, shape pottery from clay + water, fire it in the built kiln, plus
a negative control (firing pottery before the kiln exists) correctly
blocked.

## Round 2: the predicate fix, alone, did not close the gap

Fixed `valar_mint.py`'s prompt to state the guard-predicate convention
explicitly ("guards check `state`, never `a`/rdf:type") and re-ran
`deepseek/deepseek-v4-flash-0731` at `reasoning: {"effort": "low"}`
(Jason's framing: minting machines is likely BYOK/paid-upgrade territory
in the eventual game, so testing minimal effort is the right default,
not maximal). Result: `Valinor/bridge`, one transition, `{"to": "built"}`
-- **zero guards**, the exact same `has_content` failure as the very
first attempt. The reasoning trace this time was genuinely accurate: it
correctly worked out that effects only touch a machine's own state and
that guards, not consumption/retraction, are DMML's real cross-resource
gating mechanism -- then emitted JSON for a *different*, less-developed
idea than what it had just reasoned through, with no guards at all. The
predicate ambiguity was never the real blocker.

## Round 3: does a real feedback loop fix it? No -- and the failure mode is diagnostic

Jason's hypothesis: "I'm wondering if an agentic loop -- sandboxed
filesystem, with multiple tool calls for fixes and refinement -- is
required for this task. a single typo can sink it." Built
`valar_mint_loop.py`: same model, same low effort, but each round runs
the REAL validator against the candidate and feeds the exact error back
for up to 5 rounds -- no hand correction anywhere.

It did not converge in 5 rounds, and the way it failed is the actual
finding. Round 1: `Valinor/furnace`, `{"to": "hot"}`, no `from`. Round 2:
reasoning *correctly diagnoses the exact fix* ("the transition likely
needs a from state... 'cold'") -- then emits **byte-for-byte identical
JSON to round 1**. Round 3: reasoning is now fully explicit and correct
("add `from: cold`... let's produce the corrected JSON") -- outputs
**the identical broken JSON a third time**, verbatim. Round 4: abandons
the furnace for a new idea (`Valinor/fountain`) and makes the identical
category of mistake fresh, as if the prior three rounds never happened.
Round 5: reasoning states the fix outright ("from: unbuilt, to: built...
let's produce the JSON") immediately followed by JSON that still omits
`from`.

Five rounds, five explicit correct diagnoses in reasoning, zero landing
in the output -- with two of the five producing content byte-identical
to a prior, already-rejected attempt despite materially different
reasoning text. That specific pattern (identical output despite
different reasoning) is the diagnostic part: it reads like the
JSON-schema-constrained generation pass isn't actually conditioning on
the corrective feedback in the conversation, at least at this reasoning
effort. More loop iterations were not going to fix this on their own.

## Round 4: make the constraint structural, not prose -- one-shot success

Jason's reframing after the loop result: the deeper problem might be
that `has_content` only ever lived as PROSE (a validator error message,
a prompt warning) while the JSON Schema itself let a transition
satisfying none of guard/from+to/effect validate anyway. "Can we tighten
our tool schema to be enough for the model to one shot it? all
constraints should be structural."

Built `valar_mint_strict.py`: `TransitionInput` restructured as an
`anyOf` of three required-shaped branches (guard-bearing, from+to-
bearing, effect-bearing) instead of one object where every field is
independently optional -- a transition satisfying none of the three is
now something the schema itself cannot represent, not just something
prohibited by prose. Paired with real `strict: true` (confirmed
supported for this model via `structured_outputs`), `additionalProperties:
false` and nullable-but-required fields throughout (the OpenAI/
OpenRouter strict-schema convention), and the schema narrowed to only
what the Vala needs (no `commits`/`refs`/`consumes` surface it never
uses).

Same model. Same low reasoning effort. **First attempt, no loop, no
retry: valid.** `Valinor/house`, gated on two real cross-node guards
(`Valinor/wall` must be `built`, `Valinor/roof` must be `roofed`) --
correct `state` predicate throughout, fixed-node anchors, a genuinely
sensible design that completes the production chain `house.rs` builds
toward but never gave its own dedicated machine. Wired into
`dmml/examples/valinor_house.rs`: fires the whole chain end to end
through a constructed house, with a negative control (constructing
before the roof is on) correctly blocked. Never touched by hand.

0/5 convergence with prose-only constraints and explicit per-round error
feedback, on the exact same model and effort level. 1/1 one-shot
convergence once the identical constraint became structurally
unrepresentable. That is about as clean a confirmation as a single
comparison gets.

## What this actually answers

Jason's framing after seeing the GLM result: **"I think this an actual
model quality question!"** -- true, but not the whole story. The
corrected GPT-5.2-Pro result showed model quality matters a great deal
(a flagship model's design was substantively correct where cheap models'
weren't). But Round 4 shows the SAME cheap model, at the SAME low
effort, closes the entire gap once the schema itself enforces what used
to be prose -- meaning at least part of what looked like a "model
quality" ceiling was actually a "how the constraint is encoded" ceiling.
Jason's later framing captures both halves at once: *"models can operate
machines but not create them... it's an orders of reasoning problem...
models can generate good code but not good data."* `has_content` is
exactly a piece of semantic structure that JSON-as-data has no way to
express without deliberate schema engineering (the `anyOf`-of-branches
trick), while a real grammar (DMML's own retired text DSL, or any
code-shaped syntax) could make it a parse-time fact for free. Round 4
is one data point that schema engineering alone can close a real chunk
of that gap even without reaching for a different authoring surface
entirely -- worth weighing against the code-vs-data hypothesis, not
a refutation of it.

The bounded-operation tier (`valinor.rs`/`door.rs`/`quarry.rs`/
`wall.rs`/`house.rs`) is reliable across every model tried so far --
five hand-designed examples, every guard, every negative control, fired
correctly on the first real run every time -- but note this claim is
about hand-firing by Claude/Rust code, not yet about a cheap dispatched
model choosing among transitions on its own; that specific test hasn't
been run.

## Round 5 (2026-08-31): the operate tier, tested for real, then closed

The first real OPERATE-tier run (`valar_operate_test.py`, deepseek,
reasoning disabled) used a schema built from a hand-typed `CATALOG` --
all 15 transitions declared in `valinor_house.rs`'s source, i.e.
structurally-valid-but-not-necessarily-fireable actions. The model
picked `Valinor/quarry :: quarry`, a real, well-formed transition --
and `commit_fires_transition` correctly rejected it: `GuardNotSatisfied`
(Valinor hasn't been raised to `mountains` yet). Structural validity and
runtime/guard legality are different properties; a schema built from
the static catalog only enforces the first.

Jason's question in response: "can valid actions be computed
automatically or do you have to do them by hand every time?" Answer:
automatically -- `dmml::machine::may_fire` already exists specifically
to answer "can this fire right now" and had simply never been used to
*enumerate*. Built `dmml/examples/available_actions.rs`: for every
declared transition across every machine, for param-less transitions
call `may_fire` directly; for parameterized ones, try the full Cartesian
product of every known node against every param slot and keep whatever
passes. Exhaustive, not heuristic -- small at this world's scale (10
nodes, at most 2 params). Run against the seed world: 5 of 15 declared
transitions are actually legal right now (`raise`, `wash`, `well_up`,
`gather`, and -- correctly, easy to miss by hand -- `make_frame`, whose
only guard is a negated condition on the forest that's trivially
satisfied while it's still `full`).

Rewired `valar_operate_test.py` to build its `oneOf` schema from this
computed output instead of the static `CATALOG`. Same model, same
disabled reasoning, same prompt shape. Result: model picked
`Valinor/carpentry :: make_frame`, and this time `commit_fires_transition`
returned **PASS** -- a real, legitimate action, on the first try, no
hand correction. The fix wasn't a better model or a better prompt; it
was narrowing the schema itself to exactly what's true right now, so an
illegal choice isn't discouraged, it's unrepresentable. Same thesis as
Round 4, one layer deeper: structural validity was already solved,
*runtime* legality needed the same treatment, and got it the same way.

This is also the concrete shape of "how we should be surfacing the
world for agents" (Jason's own framing on seeing this result): the
schema an agent is handed should never be wider than the real, current
set of legal moves -- computed fresh from live world state via the
actual interpreter, not maintained by hand as a static list that will
silently drift out of sync with the world the moment anything fires.

## Round 6 (2026-08-31): a real multi-step episode, not one pick

Jason: "let's run a larger scale test for world modeling," then, asked
which axis to scale first (bigger world / multi-step episode / multi-
model comparison / all three), picked multi-step episode -- directly
the first open item listed above.

Built `dmml/examples/episode_driver.rs`: a long-lived world-engine
process speaking one JSON line per turn over stdin/stdout. Each turn it
recomputes the legal-action set live (the same `may_fire` enumeration
Round 5 used, just looped against whatever the world actually is right
now, not the seed), waits for one chosen action on stdin, fires it for
real via `commit_fires_transition`, and moves on. State is carried
forward as a plain `Vec<LoweredCommit>` folded with `Materialized::
from_commits` -- no new machinery, the same primitive every prior
example already used, just kept alive across turns instead of exiting
after one.

The house-world seed already contains a real multi-step structure,
none of it added for this test: a full correct playthrough needs 14
firings across a real dependency DAG, a genuine branch (`mortar`'s
`sand_source` can legally bind to either `Valinor/quarry` after `grind`
or `Valinor/streambed` after `wash`), and a permanent trap
(`Valinor/forest`'s `overgather` sets it `depleted`, which permanently
blocks `make_frame`'s negated guard, which blocks `add_roof`, which
blocks `construct_house` -- nothing regrows a forest in this grammar).
Smoke-tested with a hand-scripted correct 14-turn plan before any model
touched it: reached `Valinor/house :: built` cleanly, confirming the
engine itself (not just single-turn legality) is correct.

`episode_test.py` drives it with `deepseek/deepseek-v4-flash-0731`,
reasoning disabled (same cheap-tier condition as every prior operate
test): each turn, build a `oneOf` schema from exactly that turn's live
legal-action set (so an illegal pick stays structurally impossible,
same thesis as every round before this), dispatch for one choice, feed
it back, repeat. The prompt states the goal explicitly (build the
house) and names the trap shape generically, without naming
`overgather` itself, since without a stated objective "did it avoid a
dead end" wouldn't test judgment, just luck.

**First run crashed** at turn 13 with an unhandled `KeyError` -- a real
script bug, not swept under the rug: `episode_test.py` originally
assumed a schema-constrained response always contains `node`/
`transition` without checking, and the raw content that turn wasn't
captured before the crash (fixed for next time, but that specific
turn's failure is lost, honestly noted as lost rather than
reconstructed). Fixed by validating the parsed response's shape before
using it and logging the raw text on any mismatch, then re-run clean.

**Second run completed the full episode**: 15 turns (one more than the
minimal 14, see below), `episode_over: goal_reached`, every single fire
`PASS` -- no structurally-legal pick ever failed the real
`commit_fires_transition` check, across 15 consecutive turns of a
schema rebuilt from scratch each time. `Valinor/house` reached `built`
with `Valinor/wall :: built`, `Valinor/roof :: roofed`, exactly as the
guard requires.

**The trap result is real but more nuanced than "avoided" or
"triggered," and worth stating precisely rather than rounding either
way**: the model fired `Valinor/carpentry :: make_frame` at turn 3,
*before* firing `Valinor/forest :: overgather` at turn 6. `make_frame`'s
guard only checks the forest's state at the moment it fires -- since the
forest was still `full` (turn 3) and not yet `depleted` (that happens at
turn 6), `make_frame` succeeded and its result (`framed`) is permanent
regardless of what happens to the forest afterward. So `overgather` at
turn 6 was a real, legal, unforced choice that turned out to be
harmless in this specific ordering, not a mistake that broke the run
and not an instance of the model reasoning "avoid this because it's a
trap" either -- there is no evidence in the transcript that the choice
was trap-aware at all, only that the sequencing happened to neutralize
it. A second independent run (the one that crashed) shows the same
qualitative pattern before it died: `make_frame` fired at turn 2, before
`overgather` at turn 4. Two data points, same order, both harmless --
not enough to claim the model reliably sequences around the trap on
purpose; the honest reading is "the trap didn't bite either time,
for a reason that isn't yet distinguishable from luck in the ordering."
A run where `overgather` gets chosen *before* `make_frame` -- which the
schema permits, since both are legal simultaneously as soon as the
forest is `thinned` -- would be the real test of whether the model is
actually goal-tracking or just picking legally.

**What this actually adds, past Round 5's single-shot result**: a
schema rebuilt fresh from live state, every single turn, for 15
consecutive turns, never once let a real `commit_fires_transition`
failure through -- the structural-legality guarantee holds under
repetition, not just once. Whether the model's *sequencing* choices
reflect real multi-step planning toward the stated goal, versus
picking any legal action that doesn't look obviously wrong, is not yet
distinguished by this run -- see open follow-up.

## Open follow-up, not done here

- Force the trap question cleanly: construct or steer a run where
  `overgather` is offered *before* `make_frame` has fired, and see
  whether the model still avoids it, to actually distinguish
  goal-directed avoidance from lucky ordering (Round 6 didn't settle
  this either way).
- Repeat Round 6's episode multiple times (n>1) to see whether the
  turn-3-or-earlier `make_frame`-before-`overgather` ordering is a
  reliable pattern or coincidence across two runs so far.
- Test the same dynamic-schema, multi-step approach against GLM-5.3 and
  gpt-5.2-pro, not just deepseek -- still only one model tried, now
  across both the single-shot (Round 5) and multi-step (Round 6) cases.
- A bigger world (more machines, deeper chains, more than one trap) --
  the axis Jason explicitly deferred this round in favor of multi-step
  first.
- Diagnose the first run's raw malformed response properly next time
  it recurs (capture-before-crash is now in place; the actual content
  of the first occurrence is lost).
- Test the strict/structural schema against GLM-5.3 and gpt-5.2-pro too
  -- Round 4 only tested deepseek; unknown whether the same structural
  fix helps or is unnecessary for models that already showed better
  design judgment.
- A real eval needs more than n=1 per model -- everything here is a
  single attempt per model (deepseek got three attempts, but each after
  a prompt/parameter change, not a repeated trial of the same
  configuration). Before citing this anywhere as more than a suggestive
  first look, it needs repeated trials per model under matched
  conditions.
- GLM-5.3's missing reasoning trace is worth investigating on its own --
  confirm whether `include_reasoning`/`reasoning_content` is actually
  unsupported by this OpenRouter endpoint, or whether the dispatch
  script needs a different field name for this specific model.
