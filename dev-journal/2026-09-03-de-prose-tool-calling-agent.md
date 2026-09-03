# De-prose as a real tool-calling agent, not a scripted completion loop

Follow-up to `2026-09-03-de-prose-operator.md`. After reviewing that
pipeline's results, Jason's framing: "we're not running agents yet,
these are just completions... which makes our results all the more
impressive... let's figure out how to upgrade these lil buddies to
proper agents." Then, concretely: "all of these should be tools: check,
commit."

## The real distinction, stated precisely

`deprose.py`'s pipeline (ore extraction → smelting → assay → deposit)
is entirely driver-orchestrated. When a candidate fails
`check-declared`, the Python driver notices, builds a repair prompt, and
asks again — the model never sees that it failed in any structured way,
it's just handed a new prompt and produces a new completion. Every
"decision" in that pipeline (retry, accept, reject) is the driver's, not
the model's. This matters for Paper 2's meta-agent claim too: agency
there lives in the substrate's orchestration, not in any individual
call — worth stating plainly rather than leaving implicit.

`deprose_agent.py` changes that concretely: `check` and `commit` are
real OpenAI/OpenRouter-style function-calling tools. The model drafts,
calls `check`, reads a real structured JSON result (which stage failed,
what the actual parser/checker/gate error was), and *decides* what to
revise and when to call `commit` — including deciding for itself how
many times to commit, or not to commit at all. The driver's job shrank
to: expose the two tools, execute them faithfully against the real
`validate-commit`/`check-declared`/`retro-gate` binaries (never trust
the model's own claim that something is valid), and cap the round
budget so a stuck loop terminates.

Both tools share one implementation, `run_checks` — `commit` is not a
rubber stamp on the model's say-so, it re-runs the identical
parse → self-declaration → whole-tree-gate sequence as a final gate and
refuses to write anything that fails, exactly like a failed `check`
would report. This was a deliberate design choice, not laziness: it
guarantees `commit` can never be weaker than `check`, so there's no
incentive for the model to skip checking and just try committing.

## First real test: the same failure the scripted pipeline couldn't escape

Mary Oliver's "Wild Geese" (kept local, not redistributed — see prior
gitignore entries) stalled `deprose.py`'s scripted repair loop: 3
bounded repair attempts, the last of which hit a genuine duplicate-fact
parser error (`myDespair` and `yourDespair` both trying to relate to one
shared `exchange` concept, colliding on the same subject-predicate pair)
that the fixed retry budget never recovered from.

Same poem, agentic version: `check` failed twice (parse errors),
passed on the third draft, `commit` succeeded, and the model then ended
the session cleanly on its own — no forced tool call, no exhausted round
budget. Real convergence in exactly the failure territory the scripted
version got stuck in. Output: one clean commit, correct use of the
relation-style hedge predicates from the earlier Rule 5 change
(`wildGeese \`seemsTo\` belonging`, `despair \`seemsTo\` shared`), and —
notably — it correctly used an attribute fact (`wildGeese . harshness =
true`) for "harsh and exciting" rather than forcing everything through
a hedge-relation where an attribute fit better. Nobody told it to make
that distinction; it inferred which grammar form suited which content.

## Second real test, with real cost numbers: source1.txt (Mara/Ashgrove)

Run against `results/deprose-test/agent-world1/` (empty world), the
harness now reports real per-round token/timing stats (added specifically
because the first test above had none — Jason asked for them
afterward). Full run:

- 9 rounds, 8 check/commit tool calls, 3 commits.
- **41,455 prompt + 1,358 completion = 42,813 total tokens.**
- 44.5s API time, 44.8s wall time (0.3s local check/build/gate time —
  confirms the cost here is almost entirely API round-trips, not local
  compute).

For comparison, `deprose.py`'s scripted pipeline used roughly 3,500
tokens for the equivalent single-passage extraction (one ore-extraction
call plus at most a couple of small repair calls). **The agentic
version cost about 10x more tokens for the same source text**, because
the full conversation — system prompt, every prior tool call and its
result — gets resent on every round; token cost grows with round count,
not just with output size. This is a real, disclosed tradeoff, not
something to gloss over: agentic self-correction bought real robustness
(the poem convergence above) at a real, measurable multiplier in cost.

**A second real, honest finding from the same run**: the model split
one short passage into 3 separate commits rather than 1. Checked each
for actual duplication (the same failure mode Rule in the shared prompt
explicitly warns against, and that `deprose.py`'s ore-extraction prompt
had to be fixed for earlier) — they're NOT duplicates. Each commit adds
genuinely distinct, non-overlapping facts (core entities/relations,
then occupation/parentage/status, then quarry-closure). So this is a
legitimate decomposition, just more fragmented than necessary — one
commit would have covered it fine. Different from the earlier same-run
duplication bug, but still a real quality gap worth noting: the agent
has no cost-of-fragmentation pressure in its reward signal, so it default
to finer granularity than a human editor would choose.

## What's still open

- Whether the token-cost multiplier is fixable (e.g., summarizing/
  truncating tool results instead of keeping full history, or a smaller
  round budget) without losing the self-correction benefit that made the
  poem case work, is untested.
- The 3-way commit fragmentation suggests the prompt's "most short
  passages need exactly one commit" guidance isn't being weighted very
  strongly against the model's own sense of what counts as "separate."
  Worth a sharper rule or a stronger example, not yet tried.
- No comparison yet on a case where the SCRIPTED pipeline succeeds
  cleanly (source1/source2 already did, post-fix) — only tested the
  agentic version's advantage on a case picked because scripted failed
  there. A case where both succeed would show whether the agentic
  version's extra cost buys anything when the scripted pipeline was
  already fine, or whether the multiplier only pays for itself on hard
  cases.
- Letta/persistent cross-invocation memory (discussed, not built) stays
  deferred — nothing in either test above needed memory beyond one run's
  own accumulating message history.
