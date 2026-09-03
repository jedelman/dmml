# Authoring tools (Phase 1) and a real narration-compulsion null result

Two things, related but separate: Jason's question about whether it'd
help to give authoring agents real tools instead of asking them to
hand-write DMML syntax, and the systematic follow-up on the single
confabulation incident (dev-journal/2026-09-03-de-prose-agent-
reasoning-and-free-tier.md).

## Phase 1 of 3: syntax scaffolding, built and verified

Jason's framing: "would it make sense to provide some scripts or tool
calls to help agents author incrementally... this is what machine
operation is supposed to be." Three-phase plan, this entry covers the
first: **syntax tools now** (no engine changes), **generalize
`Effect`** next (currently `EffectAssert Text | EffectRetract Text` in
both `dmml-hs` and the "real" Rust `dmml` crate -- a transition can only
assert/retract a single bare state ident, confirmed by reading both
implementations directly, not assumed), **generalize machines** after
that (letting a transition mint new nodes, not just mutate self --
real precedent exists in `dmml-runtime`'s `GenerateFrontier` effect,
though that's a hardcoded Rust game-effect kind, not a DMML-language-
level primitive yet).

`dmml_authoring.py`: pure syntax assembly, deliberately NOT semantic
templating. `build_commit(verb, declares, mints, facts)` takes
structured input and assembles guaranteed-valid DMML text --
identifier rules, quoting, no duplicate facts all checked and rejected
with a precise reason *before* any text is emitted, not left for the
real parser to discover. Explicitly not a Python macro standing in for
DMML semantics: the assembled text still goes through the exact same
real `check`/`commit` pipeline as anything else. This distinction
matters given Section 10/11 of Paper 2's whole argument (DMML is the
evidence, not any tool's or agent's say-so) -- a macro that skipped
verification would be exactly the kind of ungoverned intermediary that
argument warns against; this one doesn't skip anything, it just makes
correct syntax easy to produce.

Wired in as a third tool (`build_commit`, alongside `check`/`commit`)
in `deprose_agent.py`. Verified for real, live: a real session
(`source2.txt`, empty world) called `build_commit` directly, assembled
15 facts correctly on the first try, and converged in 4 rounds with
**zero parse or self-declaration failures** -- notably cleaner than
typical earlier runs, though this is one session, not yet a controlled
comparison against the free-text baseline.

## The real narration-compulsion experiment: a genuine null result

`narration_compulsion.py`: systematic version of the single real
incident already on record -- runs many short sessions under
deliberate, silent friction (`--max-string-length 50`, never mentioned
in the system prompt) and classifies each session's final "no tool
call" sign-off against the real ledger (`result["committed"]`,
populated only by actual tool calls, never the model's own words).

Real, completed run: 12 sessions, Kimi, reasoning on (matching the
original incident's conditions), cap=50.

**Result: 11 accurate_success, 0 confabulated, 1 no_final_text (round
budget exhausted without a natural sign-off), 0 undersold. A real,
clean 0% confabulation rate.**

**This is a genuine null result, not a failure to reproduce anything
-- and it's informative precisely because it's clean.** The original
incident happened under `openrouter/free`, which assigns a different
underlying model to every call in the conversation. This run held the
model fixed (Kimi throughout). Zero confabulations under a fixed
strong model, one confabulation observed under real router rotation --
small samples on both sides, but the real, disclosable implication is
that **model rotation itself, not friction or incompleteness alone,
looks like the more likely causal factor** for the original incident:
a different model landing mid-conversation, without the same continuity
of "having just tried and failed" the prior model built up, may be
substantially more prone to narrating a plausible-sounding summary
disconnected from what the tool-call ledger actually shows.

**Not yet done, the obvious next step**: rerun this same experiment
against `openrouter/free` instead of a fixed model, same friction, same
sample size, to test that implication directly rather than infer it
from one clean run plus one anecdote from a different subsystem.
