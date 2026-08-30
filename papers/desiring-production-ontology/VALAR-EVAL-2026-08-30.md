# A real model eval: the Vala design tier

**Status: raw findings, not yet folded into `DRAFT.md`. Kept here as
grounding material, same discipline as `GROUNDING-2026-08-30-amber-
cracks.md`.**

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

## What this actually answers

Jason's framing after seeing the GLM result: **"I think this an actual
model quality question!"** -- and the corrected GPT-5.2-Pro result
confirms it, cleanly. The bounded-operation tier (`valinor.rs`/
`door.rs`/`quarry.rs`/`wall.rs`/`house.rs`) is reliable across every
model tried so far -- five hand-designed examples, every guard, every
negative control, fired correctly on the first real run every time. The
bounded-*design* tier (inventing new machinery from scratch, grounded in
an existing world) is not a fixed capability ceiling for LLMs in
general -- it's a real quality gradient. Two cheap/fast models
(deepseek-flash, glm-5.3) failed at the level of design itself across
four attempts; one flagship model succeeded at the level of design and
only fumbled two shallow, fixable surface details, one of which
(`update`-as-array) is a real schema-adherence question independent of
design quality, and the other (`a` vs `state`) is at least half this
harness's own prompt's fault for not saying so.

## Open follow-up, not done here

- Fix `valar_mint.py`'s `WORLD_SO_FAR`/`VALA_PROMPT` to state the guard-
  predicate convention explicitly ("guards check the `state` predicate,
  never `a`/rdf:type") and re-test whether that alone closes the gap for
  the cheaper models too, or whether the deeper design-quality gap
  (deepseek's/GLM's failure to encode any real guard at all) persists
  independent of that fix.
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
