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

## Round 7 (2026-08-31): parallel, multiple models, and a wrong claim caught

Jason: "let's do a parallel test, and with multiple models," then, in
the same message, checked a specific claim rather than taking it on
faith -- "over gather actually has an effect, but the sim didn't run
long enough to show it." That claim was verified against the actual
grammar, not assumed: grepped every guard in `episode_driver.rs` for
anything reading `Valinor/forest`'s state, and found exactly one --
`make_frame`'s. Nothing else in the world was wired to react to
`depleted`, so no amount of extra turns would have surfaced a hidden
effect. Asked which was intended (a bug in the world, or a request to
extend it); Jason picked extend.

**The grammar change.** `Valinor/streambed`'s `wash` now carries the
same negated forest-depleted guard `make_frame` does -- deforestation
destabilizes the streambed too. Confirmed directly before trusting it:
overgathering removes `wash` from the legal-action set for good, and a
direct fire attempt fails `GuardNotSatisfied`.

**A wrong claim, caught before it shipped further.** The first version
of this change's own doc comment reasoned from the "single-mutable-
resource" limitation named earlier in this session and concluded
overgathering early makes the house *permanently* unbuildable ("no
rescue path"). That reasoning was never actually run against the
engine -- and it was wrong. `EXISTS` guards are a momentary check at
firing time, not a lock: `Valinor/quarry` can be cited as `mortar`'s
`sand_source` while transiently `sand`, and still continue its own
chain to `brick` right afterward with nothing blocked (confirmed by
direct test). The real consequence is narrower than claimed: `mix` has
to catch `quarry` during that transient `sand` window specifically;
push `quarry` straight through to `brick` first and `mortar` is stuck
`unmixed` forever (also confirmed directly -- that ordering ends the
episode at `no_legal_actions`). Overgathering early collapses two
independent sand sources into one timing window; it doesn't seal the
house off outright. Both the Rust doc comment and this eval were
corrected to say exactly that, not smoothed into the cleaner-sounding
original claim.

**The parallel run itself surfaced two more real, unplanned findings**
before any of the three models finished a turn:
- `openai/gpt-5.2-pro` rejected the schema outright: *"schema must have
  a 'type' key"* alongside a bare `const` property -- OpenAI's
  strict-mode validator is stricter than deepseek's endpoint about this
  exact shape, a real cross-provider schema-portability gap this
  project hadn't hit before (every prior strict-schema test only ever
  targeted deepseek).
- Both `z-ai/glm-5.3` and `gpt-5.2-pro` rejected `reasoning: {"enabled":
  false}` outright (*"Reasoning is mandatory for this endpoint and
  cannot be disabled"*) -- glm-5.3's constraint was already documented
  in this repo's `CLAUDE.md`; gpt-5.2-pro's was new, found by direct
  probe. Switching both to `reasoning: {"effort": "low"}` fixed the
  rejection, but glm-5.3 then returned empty content on several turns
  even at low effort (the same reasoning-burns-the-budget failure mode
  already documented for deepseek at higher effort elsewhere in this
  project).

Mid-run, Jason: "Claude! Cheap models only from here!" -- the in-flight
run (already partway through billed gpt-5.2-pro and glm-5.3 reasoning
calls) was killed immediately, and the run redone with deepseek alone,
disabled reasoning, no further spend on the other two. `glm-5.3` and
`gpt-5.2-pro` stay commented out in `episode_test_multi.py` rather than
deleted, so a future paid run can restore them without re-deriving the
reasoning config.

**The clean deepseek-only rerun**: 15 turns, `goal_reached`, every fire
`PASS`. `overgather` fired at turn 6, `wash` had already fired at turn
4 -- the trap didn't bite this run either, same qualitative pattern as
both Round 6 runs (3/3 now). A separate, deliberately adversarial
scripted run (not model-driven) confirmed the failure mode is real when
the ordering is actually wrong: overgather first, then push quarry
straight to `brick` without ever mixing mortar from it -- episode ends
`no_legal_actions`, `mortar` permanently `unmixed`. The open question
from Round 6 stands, sharper now: three real, independent deepseek runs
have all threaded a real, narrower-than-first-claimed needle
successfully; still not enough to call it judgment rather than a
favorable ordering bias in how the model tends to sequence a build.

## Round 8 (2026-08-31): a swarm of cheap models, and the first real failure

Jason: "let's try a swarm with some more cheap models - Gemini flash
lite, glm flash lite, etc." Before dispatching anything, probed real
candidates against OpenRouter's own `/models` listing and a live tiny
request each -- direct lesson from Round 7's blind-dispatch burn.
`google/gemini-3.5-flash-lite` and `z-ai/glm-5.3-flash` both reject
disabled reasoning outright (mandatory, same failure as Round 7's
non-flash models). `google/gemini-2.5-flash-lite`,
`google/gemini-3.1-flash-lite`, and `z-ai/glm-4.7-flash` all accept it.

**The probe surfaced something bigger than a config detail.** With
`reasoning: {"enabled": false}` and `strict: true`, both gemini-flash-
lite variants returned JSON that flatly ignored the schema's
`const`/`type` constraints -- different, non-existent node/transition
values on repeated calls, `params` coming back as a string once despite
`type: null` being required. `z-ai/glm-4.7-flash`'s probe response
respected the constants correctly. This means `strict: true` is not
uniformly enforced across OpenRouter's providers -- Google's route for
these lite models falls back to unconstrained generation despite
`structured_outputs` being listed as a supported parameter. Real
consequence for the whole operate-tier methodology: from this round on,
every model's response is checked locally against the offered
`legal_actions` before being sent to the engine (`episode_swarm.py`'s
`is_schema_conformant`), never trusted because a `strict` flag was set.

**Four models, run in parallel, independent worlds:**

| Model | Result |
|---|---|
| `deepseek/deepseek-v4-flash-0731` | Clean win, 15 turns, `goal_reached`. Washed at turn 5, overgathered at turn 9 (after already safe) -- 4th consecutive successful deepseek run across Rounds 6-8. |
| `z-ai/glm-4.7-flash` | **The first real goal-failure in this project.** Overgathered at turn 8 without ever washing -- episode ended `no_legal_actions` at turn 9, `Valinor/mortar` permanently unmixable. Every pick up to that point was schema-conformant and fired real; the failure is a genuine bad ordering choice, not a malformed response. |
| `google/gemini-2.5-flash-lite` | Non-conformant on the very first turn: proposed `Valinor/forest :: chop(params: "logs")` -- `chop` doesn't exist anywhere in this world's grammar. Episode ended immediately; nothing was sent to the engine. |
| `google/gemini-3.1-flash-lite` | Same failure mode: `Valinor :: form(None)` on turn 1, also not a real transition. Confirms the probe's finding wasn't an artifact -- it reproduces on the first real dispatch. |

**What this round actually adds**: the first clean negative result at
the judgment level (glm-4.7-flash genuinely walked into the load-
bearing trap this project only made real in Round 7), and a second,
different failure mode entirely at the structural level (both gemini-
lite models never engaged with the schema's actual constraints at all).
These are not the same kind of failure and shouldn't be scored as one
"models can/can't operate machines" number: one model reasoned inside
the fenced space and chose badly; two others weren't fenced by the
space in the first place, at this provider route, regardless of what
`strict: true` was supposed to guarantee.

## Round 9 (2026-08-31): a live shared arena, and prose framing eating structure alive

Jason, on seeing Round 8's first real goal-failure: "models can make
mistakes in the world! that's okay! it makes the world interesting!
it's when they can't form commits that we lose their contributions!
what I'm interested in is having them interact **in the world**, in
order to **enrich it's complexity unintentionally**." Asked how models
should share one world, he picked, verbatim: "Parallel race - new
commits get broadcast" -- not round-robin, not all-propose-one-fires.

**New infrastructure.** `episode_driver.rs`'s synchronous, one-turn-at-
a-time stdin/stdout protocol can't express genuine concurrency, so this
is a new binary, `episode_arena.rs`: a tiny `std`-only TCP server
(no new dependencies), the shared world (`Arc<Mutex<Vec<LoweredCommit>>>`)
living in the server process, any number of clients connecting
concurrently. `{"query": true}` returns current state + legal actions;
`{"actor": ..., "node": ..., "transition": ..., "params": ...}`
attempts a real commit. The mutex around "check the guard, then apply"
IS the race's resolution point -- confirmed directly before trusting
it: two rapid-fire identical proposals (`raise`, `raise` again) landed
one `PASS` and one `FAIL: GuardNotSatisfied`, the second correctly
losing to a world that had already moved. `episode_arena_client.py`
runs N models as independent, unsynchronized asyncio loops -- query,
dispatch, submit, repeat, no waiting on each other -- for a fixed
90-second wall-clock window rather than a fixed step count, since
nothing here is goal-bounded by design.

**Real methodological break from Round 8, on purpose**: this client
does NOT locally gate a model's proposal against the offered
`legal_actions` before submitting (Round 8's `is_schema_conformant`).
Jason's framing is explicit that a legal-but-bad move is real content,
not noise -- so every parseable proposal goes straight to the arena,
which is the actual arbiter. Only a response that can't be parsed into
an action AT ALL is logged and skipped, since there's nothing to submit.

**The result nobody predicted going in: all four models built the
whole house together, with zero coordination.** 15 real commits landed
across the 90-second window (out of 306 total attempts across all four
models), composing into the complete build -- `glm-4.7-flash` fired 8
of them (`raise`, `uplift`, `quarry`, `grind`, `gather`, `well_up`,
`wash`, `make_frame`), `deepseek` fired 5 (`overgather` -- harmless
again, make_frame already secured -- `wet`, `fire`, `add_roof`,
`construct_house`), and, strikingly, `gemini-3.1-flash-lite` and
`gemini-2.5-flash-lite` -- the two models that were almost entirely
unable to stay inside the schema (below) -- each landed exactly ONE
real, correctly-timed commit: `gemini-3.1-flash-lite`'s single hit was
`Valinor/mortar :: mix` citing `Valinor/quarry` as `sand_source` while
quarry happened to be transiently `sand` (commit 8, before `quarry`
itself later continued on to `wet`/`fire` at commits 12-13) -- the
exact narrow-window mechanism Round 7 identified by deliberate
adversarial testing, here discovered organically, unprompted, by a
model that got almost everything else wrong. `gemini-2.5-flash-lite`'s
one hit was the `build` that used it. Neither model was told about that
mechanism or was even reliably inside the schema; each still supplied
the one piece the whole structure needed at the moment it needed it.
This is close to a direct, if small, demonstration of what "enrich the
world unintentionally" was asking for.

**But the real headline finding is a problem, not a success, and it's
worth stating precisely rather than folding it into the nice story
above.** Across the whole run: 210 of 306 attempts (69%) were "could
not form a commit" -- the model wrote free-form, natural-language-
flavored JSON (`{"action": "build", "location": "Valinor/streambed",
"object": "bridge", "details": "Construct a simple wooden bridge..."}`,
`{"action": "explore", "location": "Valinor/forest", "description":
"Venture into the depleted forest..."}`) that doesn't remotely match
`{node, transition, params}`, let alone the `const`-tagged branches.
Another 56 (18%) reached the server but failed even the untagged-enum
parse there (a value with the right key but the wrong type, e.g.
`params` as a bare string instead of an object -- the same failure
mode Round 8's probe caught for gemini-lite specifically). Only 40 of
306 (13%) were well-formed enough to actually reach
`commit_fires_transition`; of those, 15 landed.

**This happened to EVERY model, including `deepseek`** -- the one
model that had been 100% schema-conformant across every prior round
(Rounds 4 through 8, dozens of real dispatched turns, zero prior
parse failures of this kind). The only thing that changed between
Round 8's clean deepseek run and this one is the prompt's framing:
Round 8 stated an explicit goal ("reach Valinor/house :: built") and
described the schema as the complete menu; this round explicitly said
"there is no single goal here; take any action that seems worthwhile,
interesting, or that develops the world further" (Jason's own
"unintentionally enrich" framing, translated into the prompt). Nothing
about the schema itself changed -- same `anyOf` of `const`-tagged,
`strict: true` branches, same underlying primitive. Given permission to
be inventive, deepseek mostly stopped trying to fit inside the fence at
all, proposing real, often plausible-sounding ideas (a bridge over the
streambed, exploring the depleted forest, a well drawing on spring and
quarry) that the grammar has no way to represent.

**Why this matters for the paper's actual thesis, stated as sharply as
the finding deserves**: every earlier round located the operate/design
boundary as if it were solely a property of the schema's own
structure -- tighten the schema (Round 4, Round 5) and the gap closes,
regardless of model or prompt. This round shows that's incomplete.
Prose framing has real causal power over whether a model even attempts
to stay inside a structural fence, independent of whether the fence
itself is well-built. `strict: true` fully determines what's ACCEPTED
once a model tries to conform -- Round 4 through 8 all confirm that
half. It does nothing to make a model try, and an open-ended,
creativity-inviting prompt measurably suppresses that trying, at least
for deepseek, at least here. "No logic should live in prose, it should
live in the structure" is a claim about where a constraint's
*enforcement* lives; it is not a claim that prose has no effect on
whether the constrained surface gets engaged with in the first place --
and this round is the first real data point showing prose framing can
suppress engagement with an unchanged, fully-capable structural fence.
That's a real complication for the thesis to carry forward, not a
result to smooth past.

## Round 10 (2026-08-31): the negation hypothesis, tested directly -- and it doesn't hold

Jason's read on Round 9's collapse, immediate and specific: "I think
the 'no' is what's throwing them off! positivity only for these lil
guys 🤗" -- Round 9's prompt was genuinely negation-heavy ("there is no
single goal", "may already be stale", "not a failure to avoid at all
costs", "may no longer be legal", "not an error"), and small/cheap
models are known to handle negation unreliably.

Rewrote `episode_arena_client.py`'s prompt to state everything
affirmatively -- no "no," no "not," no mention of staleness, failure,
or error at all, just "explore freely and take whichever action feels
most worthwhile or interesting right now." Same four models, same
90-second window, same unchanged schema mechanism, same arena. Reran
rather than assuming the fix worked.

**It didn't move the number.** Round 9 (negation-heavy): 210 of 306
attempts could not even be parsed into an action -- 68.6%. Round 10
(positive-only): 194 of 272 -- 71.3%. Flat, if anything marginally
worse, well within what looks like ordinary run-to-run noise rather
than a real effect in either direction. Per-model rates moved in both
directions (deepseek improved 84%->79%; `glm-4.7-flash` improved
85%->76%; both gemini-lite models got worse, 60%->68% and 60%->71%
respectively) -- not a consistent signal, which is itself informative:
if negation were the load-bearing variable, removing it should have
helped broadly, not helped two models and hurt two others.

**The hypothesis is checked and doesn't survive contact with the real
data.** Negation-heavy phrasing is not what's driving the structural-
conformance collapse Round 9 found. Something else about open-ended,
non-goal-directed framing itself -- independent of whether it's phrased
positively or negatively -- is what pulls a model toward writing free-
form natural-language JSON instead of matching the schema. Worth
recording plainly rather than declaring victory on a fix that looked
plausible and simply didn't work: the "no" was a reasonable hypothesis,
tested in good faith, and ruled out by the actual numbers.

One genuine, unplanned charm from this run, kept in the log rather than
edited out: several of the "could not form a commit" proposals under
positive framing were thematically apt in a way nothing prompted --
`{"Valinor/forest": "replanting"}`, `"regrowing"`, `"replanted"`,
repeated across dozens of `gemini-3.1-flash-lite`/`gemini-2.5-flash-
lite` attempts, and `deepseek` proposing to "line the streambed with
stones from the quarry... preventing erosion" -- unprompted echoes of
exactly the forest-depletion/streambed-erosion mechanism Round 7 wired
into the grammar, arrived at by models that were simultaneously unable
to express the idea in the form the engine could actually accept.

## Round 11 (2026-08-31): a real GA over the prompt, and the confound it accidentally removed

Jason: "can we run an evolutionary algorithm on the prompt itself?"
Built `prompt_evolution.py`: isolates the exact variable that differed
between Round 9 and Round 10 (the framing paragraph, held between a
fixed intro and fixed closing instruction) and optimizes it directly
against schema-conformance rate -- population 5, 3 generations, 6 real
dispatched trials per genome per generation, one model (deepseek), real
mechanical genetic operators (sentence crossover, synonym swap,
sentence shuffle, schema-reminder append, emphasis prepend,
truncation), not another LLM call asked to "improve" the prompt. Every
trial ran against the SAME real, live-queried seed state, for a
controlled, fair comparison.

**Every genome scored at or near ceiling -- 1.00 or 0.83 across all
three generations, including `round9-negation` and `round10-positive`
themselves, the exact texts that scored 68.6%/71.3% could-not-form-
commit in the real 90-second arena.** The GA never found a fitness
landscape to climb because there wasn't one under these conditions --
which means the controlled setup, in isolating the framing-paragraph
variable, had accidentally also removed whatever was actually causing
Round 9/10's collapse. A result that looks like "the prompt doesn't
matter, everything works" is not evidence the problem is solved; it's
evidence the experiment stopped reproducing the problem, and that
needed chasing down before drawing any conclusion at all.

**Follow-up, run immediately rather than left as a guess**
(`prompt_evolution_followup.py`): same exact genome text
(`round9-negation`, the one real historical text with a known 84%-
failure real-world rate), four conditions crossing two candidate
confounds -- the short intro used in the GA vs. Round 9/10's actual
fuller intro (which names concurrent agents and warns the world may go
stale), and the simple seed state (5 param-less actions) vs. a real
mid-build state offering a parameterized action (`mortar :: mix` with
real `sand_source`/`water_source` values). **All four conditions
scored a perfect 6/6.** Neither the fuller intro's concurrency/
staleness language nor a parameterized action's added structural
complexity reproduces the collapse either.

**What's actually been ruled out now, precisely:** negation (Round 10).
The framing paragraph's content, whatever it says (this round -- five
very different genomes, all near-ceiling). Intro length/detail
(follow-up). State/param complexity (follow-up). None of it. Every
single-shot, isolated dispatch to deepseek with this exact genome text
was schema-conformant nearly every time, regardless of which of these
variables changed.

**What's left, unconfirmed but sharpened considerably:** whatever
actually differs between "one isolated dispatch call" (this round's
every condition) and "one model in a tight, repeated, unsynchronized
loop making dozens of rapid-fire calls while three OTHER models are
doing the same concurrently against the same live-changing world" (the
real Round 9/10 arena). Real candidates, none tested yet: the growing
"Actions taken so far" history block Round 9/10's actual client
appends turn by turn (never reproduced in either this round or the
follow-up, since both used a single one-shot dispatch with no history);
genuine request-volume/concurrency effects at the API layer across four
models firing simultaneously; or something about sustained, repeated
dispatch to the same model specifically (as opposed to one clean,
isolated call) that degrades output quality over a session. This is a
real, live open question, not a solved one -- the honest state of Round
11 is "ruled out four plausible variables, found the actual cause is
still elsewhere," not "fixed."

## Round 12 (2026-08-31): concurrency, tested directly and ruled out

Jason: "let's see if we can reproduce the collapse. is it the
concurrency?" Built `concurrency_isolation_test.py`: the exact
structural shape of the real arena run (same four models, same 90-
second window, each in its own unsynchronized asyncio loop, dispatched
concurrently) but with everything else stripped to the isolated
single-shot condition Round 11 already tested clean -- a FROZEN static
seed state (never changes, no `episode_arena` connection at all) and
no history block (every call is independent, as if it were the agent's
first turn, every time). If conformance collapses here anyway, sustained
concurrent request volume is the driver. If it stays near-ceiling,
concurrency is ruled out too.

**For the cleanest case, it's ruled out.** `deepseek` scored 20/20 =
100% conformant under real 90-second, four-model concurrent dispatch
pressure -- identical to its isolated single-shot rate in Round 11, and
nowhere near its real-arena rate (Round 9: only 8 of 50 attempts even
parsed with a `node`/`transition` at all, i.e. ~16%). `glm-4.7-flash`
scored 35/38 = 92%, also far above its real-arena rate (~15-24%
conformant across Rounds 9-10). Neither model's conformance degraded
over the 90-second window either -- the 15-second time-bucket breakdown
shows no fatigue/volume trend for the models that were mostly
conformant to begin with, ruling out a "degrades under sustained load"
version of the concurrency hypothesis too, not just a "concurrency
itself" version.

`gemini-2.5-flash-lite` and `gemini-3.1-flash-lite` both scored 0%
here, out of 118 and 88 attempts respectively -- but this doesn't
implicate concurrency either. It's consistent with Round 8's original
finding that these two models' `strict: true` enforcement is broken at
the provider level, independent of condition; a truly exact-match
schema check was never run against them in isolation before this round
to confirm the baseline is this low, but nothing here suggests
concurrency is what's driving it rather than the same enforcement gap
Round 8 already found.

**Net: concurrency, tested directly with the real structural shape of
the collapse-producing run, does not reproduce the collapse for the
models where the collapse was cleanest and best-documented.** Ruled-out
list is now five: negation, framing-paragraph content, intro length,
state/param complexity, and concurrent request volume/sustained load.

**Correction, caught immediately on the next round rather than left
standing**: this section originally named "the growing 'Actions taken
so far' history block the real client appends per-agent" as the
remaining untested candidate. That was wrong, and checked before
anything was built on it -- `episode_arena_client.py`'s `agent_loop`
(re-read directly, not assumed) never accumulates or passes any
history to `build_prompt` at all; every dispatch in Rounds 9, 10, and
12 alike was already a single fresh, stateless request with no memory
of prior turns. That history-block language only ever existed in the
older, single-agent `episode_test.py` (Round 6), a different script
entirely. There was nothing to "clean."

## Round 13 (2026-08-31): the moving target, triangulated from data already in hand

Jason, on hearing concurrency was ruled out: "yep, maybe it's the
moving target. what if we cleaned their context?" The context-cleaning
question is what surfaced the Round 12 correction above -- there was no
context to clean. But "moving target" turns out to be exactly right,
made precise rather than vague, by comparing three conditions already
run and logged, no new dispatch needed:

| Condition | deepseek conformance |
|---|---|
| Single-agent, real live evolving world, state changes only from its own actions (Rounds 6 & 8: `episode_driver.rs`/`episode_swarm.py`, two independent full 15-turn runs) | 30/30 = 100% |
| Multi-agent, real live evolving world, state can change from OTHER agents' unpredictable concurrent commits (Round 9: real `episode_arena` run) | 8/50 = 16% |
| Multi-agent dispatch structure and volume, but a FROZEN, never-changing world (Round 12) | 20/20 = 100% |

This triangulates precisely, ruling out two coarser versions of "moving
target" along the way: it is not merely that the world changes over
time (row 1 -- deepseek handles a real, live, self-caused evolution
perfectly, across two separate full playthroughs) and it is not merely
concurrent dispatch pressure without real stakes (row 3, Round 12). The
actual driver, narrowed by elimination to the one condition present in
row 2 and absent from rows 1 and 3: **state that can change between
when an agent queries and when its action lands, for reasons the
querying agent did not cause and cannot predict or attribute.** A
model reasoning about "the world I just saw" is reasoning about
something that may already be a different world by the time its commit
arrives, through no action of its own -- and unlike simply being TOLD
this might happen (Round 11's `full_intro` condition tested that
framing alone, isolated from real multi-agent unpredictability, and it
stayed at 100%), actually being subject to it seems to be what erodes
schema-conformance, not knowing about the possibility in the abstract.

This is a different, sharper claim than "concurrency" or "history," and
it reframes what Round 9's collapse actually was: not a prompt-
engineering problem (Rounds 10-11 already showed prompt text isn't the
lever) and not an infrastructure-load problem (Round 12), but something
closer to genuine multi-agent unpredictability itself taxing a small
model's ability to stay inside a structural fence -- plausibly because
the model is implicitly reasoning about a snapshot it can no longer
fully trust, even though nothing in the prompt asks it to reason about
trust at all.

## Round 14 (2026-08-31): drift attribution, built and tested -- it doesn't fix it either

Jason: "remember how we built a drift primitive?" -- `dmml::interpret::
diverges`, already real, already proven (`dmml/examples/drift_machine.rs`,
CLAUDE.md's "DMML first" section), computing exactly "what differs
between two materialized snapshots." Round 13 had named this shape of
fix as the thing worth testing rather than assuming: give a querying
agent structured attribution of what changed since it last looked, not
a bare fresh snapshot and not a narrated warning.

Built it for real. `episode_arena.rs` now tracks each actor's own
last-seen `Materialized` snapshot server-side (`Arc<Mutex<HashMap<
String, Materialized>>>`, keyed by the `actor` every request already
carries) and calls `diverges(&previous, &current)` on every `Query`,
returning the result as `changed_since_you_last_looked` -- real,
computed data, not guessed. An actor's first-ever query correctly
reports no drift (nothing to compare against yet); confirmed directly
before trusting it, with a two-actor manual test: actor A's first look
is empty, actor B fires `raise`, A's next look shows exactly
`Valinor: unformed -> hills` attributed to nothing it did, and B's own
next look shows nothing new (it already knew about its own change).
`episode_arena_client.py` renders that drift as plain `subject: before
-> after` lines ahead of the current-state block -- structured, never
folded into a sentence.

**It did not help. If anything, it was worse.** Same four models, same
90-second window, same everything else as Round 9: overall could-not-
form-commit rate was 173/219 = 79%, against Round 9's 68.6% and Round
10's 71.3%. `deepseek` specifically: 20/22 = 91% could-not-form-commit,
worse than both its Round 9 (84%) and Round 10 (79%) rates. The sample
is smaller than prior rounds (adding the per-actor mutex lookup and a
longer prompt slowed each round-trip enough that fewer total attempts
fit in the same 90 seconds -- 219 total vs. Round 9's 306), so this
specific number carries more noise than the earlier rounds' larger
samples; but there is no reading of it as an improvement, and a script
bug caught mid-run (the final summary query didn't send the now-
required `actor` field, fixed on the spot) didn't affect the logged
per-turn data, only the end-of-run printout.

**A real, well-motivated mitigation, built correctly, tested honestly,
and it doesn't fix the collapse.** Worth sitting with rather than
rationalizing: "give it slightly more, but structured" was Round 13's
best guess at the mechanism, framed carefully to avoid just re-trying
prose (Round 10 already showed extra prose doesn't help) -- and even a
real, computed, correctly-attributed structural diff, using an already-
proven primitive, didn't move the number. Two live possibilities, not
yet distinguished: (a) the actual driver isn't "the agent lacks
information about what changed" at all, so no amount of better
information fixes it -- something else about being one of several
concurrent actors is what taxes schema-conformance, independent of
what the agent is told; or (b) the drift block, exactly like the
"Actions taken so far" block Round 13 first (wrongly) suspected, is
itself extra prose competing for the model's attention against the
strict schema, and adding real information in unstructured-adjacent
form doesn't escape that problem just because the underlying data is
computed rather than guessed.

## Round 15 (2026-08-31): a real mutex across the whole cycle -- and the number roughly halves

Jason: "maybe a mutex is the correct primitive - they really have to
take turns?" A sharper proposal than plain round-robin: not a fixed
rotation, but a real mutual-exclusion lock held across an agent's
ENTIRE query-decide-act cycle, not just around the arena's existing
commit step (which was already mutex-protected, but only for the
instant of firing). While one agent holds it, no other agent can query
or act at all -- the world genuinely cannot move between when an agent
looks and when its action lands, by construction, the same guarantee
Rounds 6 and 8's single-agent runs had for free (nothing else was ever
acting there) and Round 9's "parallel race" deliberately gave up in
favor of genuine concurrency.

Implemented as one `asyncio.Lock` in `episode_arena_client.py`, wrapping
each agent's full query -> dispatch -> submit sequence. Same four
models, same 90-second window, same drift-attribution mechanism from
Round 14 left in place (a real confound, named honestly below, not
hidden).

**Could-not-form-commit dropped to 37.5% (12/32)** -- roughly half of
every prior round: Round 9's 68.6%, Round 10's 71.3%, Round 14's 79%.
Strikingly uniform across all four models too -- exactly 3 of 8
attempts each, including the two gemini-lite models that had shown
near-total non-conformance in every isolated test so far (Round 12:
0/118 and 0/88; Round 8's original probe). `deepseek` and `glm-4.7-
flash` each landed 5 of 8 real commits (62.5%), far above their real-
arena rates in every prior multi-agent round.

**This is the strongest single result in the whole operate-tier
thread, and it fits the mechanism Round 13 triangulated precisely**:
if the driver really is state changing for reasons the querying agent
didn't cause and couldn't predict, then removing that possibility
entirely -- not describing it away (Round 11), not attributing it after
the fact (Round 14), but making it structurally impossible for the
duration of one agent's reasoning -- is exactly the fix that should
work. It did.

**Two things kept honest rather than smoothed into a cleaner story:**

1. **Small sample, real tradeoff.** Full serialization is expensive:
   only 32 total attempts fit in the same 90-second window that gave
   Round 9 306 attempts (8 per model here vs. up to 120 per model
   there) -- turn-taking has a real throughput cost, the same way a
   real mutex always trades concurrency for correctness. n=8 per model
   is genuinely small; the effect size (roughly halving a rate that's
   been stable across four other conditions) is large enough to take
   seriously, but this wants a longer run or repeated trials before
   being treated as a settled number, not just a striking one.
2. **This tests "mutex" and "drift attribution" together, not mutex
   alone.** Round 14's drift-attribution code was left active rather
   than reverted, so this result cannot yet distinguish "the mutex did
   it" from "the mutex plus drift did it" -- worth knowing precisely,
   not assumed, especially since Round 14 showed drift ALONE (without a
   mutex) made things worse, not better. The clean next step is named
   below.

`gemini-2.5-flash-lite`'s and `gemini-3.1-flash-lite`'s remaining
failures also show a residual, different problem the mutex doesn't
touch: several `arena_protocol_error`s (right top-level shape, wrong
value type -- `params` sent as a bare string again) -- Round 8's
original schema-portability finding for these two models, unaffected
by turn-taking, still there underneath the improvement.

## Round 16 (2026-08-31): the mechanism question, and mutex isolated from drift

Jason, in one message: "isolate mutex alone, rerun without drift" plus
a sharper conceptual question -- "how is changing during reasoning
communicated to the agent?"

**The mechanism question first, because it shaped what the isolation
run actually needed to measure.** It isn't literally communicated
during reasoning at all -- there is no live channel into a single
forward pass; a model generates against a prompt already frozen at
query time, and nothing streams into it mid-generation. That mechanism
cleanly explains a *legality* failure (the world moved, the guard no
longer holds, `commit_fires_transition` rejects it) but does not
obviously explain a *parseability* failure (`could_not_form_commit`),
since whether JSON parses correctly is fixed at generation time,
unaffected by anything that happens after generation starts. The best
account on offer: under free concurrency, several other agents can each
complete a full query-decide-act cycle in the gap between one agent's
own two looks, so successive snapshots can land on combinations no
single coherent turn order would ever produce -- several different
agents' half-finished intentions overlapped into one state, not a
smooth progression. The mutex doesn't give the agent awareness of
anything; it bounds how much the world can move per gap, changing the
*distribution of snapshots* the model ever has to reason about, not
what it's told.

**The isolation run confirms this reading directly, with no
narration added at all.** `INCLUDE_DRIFT_IN_PROMPT = False` -- the
server still computes real drift internally (harmless, keeps the code
exercised) but the client never surfaces it; the mutex alone is the
only thing distinguishing this run from Round 9's baseline.

| Condition | could-not-form-commit |
|---|---|
| Round 9: no mutex, no drift (baseline) | 68.6% (210/306) |
| Round 16: mutex alone, no drift | **51.1% (23/45)** |
| Round 15: mutex + drift | 37.5% (12/32) |

Mutex alone is a real, substantial improvement on its own -- about 17.5
points off baseline -- with the model given zero new information, no
attribution, nothing narrated or computed and shown to it. This is
strong support for the distribution-of-snapshots account over any
account requiring the model to be "informed" of anything: the only
variable that changed was which state combinations it happened to see,
never what it was told about them. Adding drift attribution on top
(Round 15) improves further, to 37.5% -- the two interventions appear
to be doing distinct, complementary work rather than one subsuming the
other, though with n=32-45 per condition this reads as a real trend
worth taking seriously, not yet a settled ratio.

Per-model, mutex-alone: `deepseek` 6/12 (50%), `glm-4.7-flash` 5/11
(45%), `gemini-2.5-flash-lite` 6/11 (55%), `gemini-3.1-flash-lite` 6/11
(55%) -- all four models improved from their respective baselines, and
the swarm collaboratively finished the full house again in this run
too (`Valinor/house: built` in the final state), the second time a
mutex-based run has produced a complete, unplanned collaborative build.

## Open follow-up, not done here

- Repeat Rounds 15 and 16 at longer duration or across multiple runs
  to firm up n=32-45-sized samples before treating the specific gap
  between mutex-alone and mutex+drift as more than a real, promising
  trend.
- Test whether the distribution-of-snapshots account predicts
  anything further: does could-not-form-commit correlate, turn by
  turn, with how many OTHER agents' commits landed between this
  agent's own two most recent looks (a direct "burstiness" measure)?
  If it does, that's real, quantitative support for the account above,
  not just a plausible story fit to two data points.
- Distinguish (a) from (b) above directly: rerun Round 12's frozen-
  world isolation test, but with a FAKE, hardcoded non-empty drift
  block prepended to the identical prompt (no real second agent, no
  real live arena, just the same-shaped text Round 14 produces). If
  conformance stays high, the drift text itself isn't the problem and
  (a) is more likely -- something about genuine multi-agent
  unpredictability itself, not what's said about it. If conformance
  drops back toward Round 14's level even against a frozen world, the
  extra text is competing with the schema regardless of whether it's
  computed or narrated, and (b) is closer to right.
- This triangulation (Round 13) used existing data, not a new
  controlled run built to test it directly -- still worth running once:
  single deepseek agent querying a real, live `episode_arena` while
  OTHER (non-dispatched, scripted) agents fire real, unpredictable
  commits into the same world concurrently, isolating "I am not the
  only cause of change here" from every other variable already ruled
  out, independent of whether drift attribution is present.
- Prompt-length/latency is now itself a confound worth controlling for
  directly -- Round 14's smaller sample came partly from slower round
  trips; a fairer A/B needs equal wall-clock attempt counts, not just
  equal duration.
- Only once a fix is actually confirmed against the real mechanism does
  re-running the GA become worthwhile -- optimizing prompt text was
  never the lever; two real attempts at a structural fix (drift
  attribution) and zero at prompt wording have worked so far, which is
  itself informative about where to keep looking.
- Route the 56 arena-protocol-errors (right shape, wrong value type)
  through the same real engine error path as a normal FAIL rather than
  a raw parse rejection, so "reached the engine but was malformed" and
  "never reached the engine" are both visible as the same kind of lost
  contribution in future runs, not split across two buckets.
- Retry gemini-2.5-flash-lite/gemini-3.1-flash-lite through a different
  route if OpenRouter exposes one (a non-Google-native provider, or a
  different API shape) to see whether the `strict` violation is
  Google's provider integration specifically or the model itself.
- Now that local schema-conformance checking exists
  (`episode_swarm.py`), retroactively distinguish "non-conformant" from
  "conformant but wrong" as a permanent two-axis result for every future
  round, not just this one.
- Fix the gpt-5.2-pro schema shape (add an explicit `"type"` alongside
  every `const` property) and retry both it and glm-5.3 with a real
  budget, now that the reasoning-config and schema issues are diagnosed
  rather than just worked around by dropping them.
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
