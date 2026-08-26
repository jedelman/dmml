---
name: dispatch-methodology
description: Standing practice for briefing and reviewing any dispatched model or agent in this project — OpenRouter models (ox-alpha, Deepseek, Kimi), independent sub-agents, or any future dispatch. Distilled from Jason's own teaching methodology across the Benjamin close-reading series (2026-08-25), not tied to that essay specifically. Load this before dispatching a model for an interpretive, adversarial, or independent-reading task.
---

# Dispatch methodology

This project dispatches other models and agents constantly — Coder/
Reviewer (Kimi/Deepseek) for engine code, an adversarial-review model
for philosophical critique, fresh sub-agents for independent readings.
The practice below is not about DMML or Benjamin specifically; it's how
those dispatches should be briefed and reviewed, distilled from watching
Jason correct my own dispatch practice in real time.

**The adversarial-review model referenced below as `ox-alpha` was
OpenRouter's stealth listing for that role at the time these examples
were built; it's since been unveiled as `z-ai/glm-5.3` (confirmed by
Jason, 2026-08-26 — see written-world's `CLAUDE.md` for the full
history and current dispatch specifics). Examples below keep the
`ox-alpha` name since they're recording what actually happened at
dispatch time; use `z-ai/glm-5.3` for any new dispatch in this role.

## Point authoring dispatches at `AUTHORING.md`

Any dispatch that will `declare` new vocabulary (not just review existing
facts) should get a pointer to `AUTHORING.md`'s reuse guidance in its
brief, the same way it gets the DMML syntax itself. The ontology is
deliberately open — `declare` is closed only until extended — which makes
diffusion, dispersal, and dilution of near-duplicate vocabulary a real,
un-enforced risk, confirmed concretely in `paper_predicate_convergence.rs`:
generic-word convergence (`claim`) says little, task-specific-coinage
convergence (`counterClaim`) says a lot. Don't rely on a dispatched model
to independently rediscover that distinction each time — brief it in.

## Give the primary material, not your own summary

A compressed fact-list of "here's what I found" produces a dispatch that
reacts to your framing, not an independent judgment. If the task is
interpretive (a reading, a critique, an evaluation), the dispatched
model needs the actual primary source — the essay, the code, the spec —
not your digest of it. Confirmed directly: `ox-alpha` given only a
summary of 44 facts produced a real but summary-shaped critique;
a fresh agent given the actual primary text produced points the summary
couldn't have surfaced at all (a cross-section link neither the summary
nor the original model had built). The gap wasn't model quality — it was
what each one was allowed to see.

## Review point-by-point; never apply wholesale, never reject wholesale

A dispatched model's output is not a verdict — it's a set of separate,
individually-checkable claims. Some will be right, some wrong, some
half-right. Go through them one at a time. Accept what holds up, dispute
what doesn't, and when disputing, build the disputed claim faithfully
first (don't caricature it) before building your counter. Two real
instances: two of ox-alpha's three challenges were accepted as real
citations; the third was built as its own commit and then disputed by a
second commit that consumed it, rather than silently overridden or
uncritically adopted.

## Disagreement is a feature to build, not smooth over

Don't resolve a real disagreement into one paragraph that "balances both
views." If the mechanism supports it (in DMML: `consumes` + a `disputes`
commit), make the disagreement a real, separately-checkable, coexisting
artifact — both sides remain citable, neither erases the other. A
synthesis, when one comes, should itself be a new artifact built on both,
not a retroactive edit to either.

## Extensions should engage the actual prior move, not just the topic

When a second dispatch responds to ground a first one already covered
(a rebuttal, a synthesis, a further reading), check whether it's
actually citing the specific prior claim/commit or just producing
parallel commentary on the same subject. A real extension consumes the
prior artifact directly (checkable: does it cite that fact's specific
predicate, not just talk about the same topic).

## Check comprehension at a human pace, not just at task boundaries

Don't wait until a task is "done" to confirm real understanding landed.
Genuine mid-task checks ("what do you think," "explain this back,"
"where do you think this goes") surface drift or missed connections
while they're still cheap to fix, and they're not a formality — they
should produce a real answer with actual content, not a status update.

## Don't lose the actual object to the apparatus built to study it

Especially true for multi-model dispatch pipelines: it's easy to end up
more interested in the dispatch mechanism (prompts, review cycles,
citation postures) than in the actual subject the mechanism exists to
serve. Periodically check: is this still in service of understanding the
real thing, or has the tooling become the thing being admired?

## Rigor and lightness are not in tension

None of the above requires grimness. A joke mid-task doesn't lower the
bar for what counts as a checked claim; it just means the standard
doesn't have to feel punishing to hold.
