# Literature review: does academic/industry work already explain what Rounds 4-18 found?

**Status: real findings from real searches, done 2026-08-31 after Jason asked
"do a lit review, see if the academics have anything to say about it." Every
citation below was actually fetched or returned by a live search this
session, not recalled from training. Where a connection is strong (the same
phenomenon, independently documented) that's said plainly; where it's
suggestive or partial, that's said just as plainly. This document doesn't
replace `VALAR-EVAL-2026-08-30.md`'s own findings -- it grounds them.**

## 1. Structured-output enforcement is not uniform across providers

**Our finding** (Round 8, confirmed again Round 16/17): `strict: true`
JSON-schema mode is not equally enforced everywhere. Deepseek's endpoint
respected `const`-tagged schemas reliably; Google's route for
`gemini-2.5-flash-lite`/`gemini-3.1-flash-lite` returned JSON that flatly
ignored `const`/`type` constraints, non-deterministically, across repeated
identical calls.

**Literature.** [JSONSchemaBench](https://arxiv.org/pdf/2501.10868)
benchmarks exactly this: 10K real-world JSON schemas across 10K datasets,
evaluated against six constrained-decoding backends (Guidance, Outlines,
llama.cpp, XGrammar, OpenAI, Gemini) specifically because compliance rates
differ by backend and schema complexity -- our finding is the same
observation Made in this project's own harness, not a new phenomenon.
[How Structured Outputs and Constrained Decoding
Work](https://letsdatascience.com/blog/structured-outputs-making-llms-return-reliable-json)
and the vLLM/XGrammar production notes describe *how* real enforcement
should work (token-level masking against the schema at decode time) --
useful as a description of what a provider that DOES enforce `strict` is
actually doing, which sharpens what it means that Google's route for these
two models apparently wasn't doing it.

**What's genuinely ours, not covered by the benchmark literature**: the
benchmarks measure aggregate compliance rates per backend; they don't (as
far as this search found) document the specific behavior we caught directly
-- an API returning HTTP 200, no error, with content that silently
disregards `const` on a field while conforming to `type`. That's a sharper,
more operationally dangerous failure mode than "sometimes non-compliant,"
and worth naming precisely in the paper as this project's own contribution,
not assumed to be already characterized elsewhere.

## 2. Reasoning-token budget exhaustion returning empty content

**Our finding** (Round 4's deepseek dispatch, Round 14's glm-5.3): a model
with reasoning enabled can silently consume its entire `max_tokens` budget
on internal reasoning, returning `content: null` with no visible error --
confirmed directly multiple times this session, worked around by disabling
reasoning or raising the budget.

**Literature.** This is a well-documented, named industry failure mode, not
unique to this project. A filed bug against a reasoning model
([hermes-agent#9344](https://github.com/NousResearch/hermes-agent/issues/9344))
describes precisely this: "reasoning tokens exhaust output budget, producing
empty responses with no recovery path." Coverage of the mechanism: "API
calls return HTTP 200 with responses that parse cleanly, but
`choices[0].message.content` is null and `finish_reason` is `length`...
typical reasoning models require 1500-3000 tokens of thinking overhead...
with thinking on, all available tokens can be consumed by the internal
reasoning block before the model produces a single character of actual
output" ([Medium: How Token Budgets Can Shift LLM Benchmark
Accuracy](https://medium.com/@gsagar/my-model-knew-the-answer-but-wasnt-allowed-to-finish-717af0354501)).
vLLM's own reasoning-outputs documentation and a dedicated
`thinking_token_budget` feature exist specifically to cap this. Confirms our
fix (disable reasoning for the cheap operate tier, or raise `max_tokens`
substantially) is the standard mitigation, not an ad hoc workaround.

## 3. Forcing structured output degrades reasoning quality -- a real tension with this project's own thesis

**Our finding** (most sharply, Round 17): stripping the prompt to a bare
minimal menu -- maximal structure, minimal prose -- regressed badly (~75%
could-not-form-commit, worse than baseline), even under the mutex condition
that otherwise cut the rate in half.

**Literature, and this is the most important connection in this review**:
["Let Me Speak Freely? A Study on the Impact of Format Restrictions on
Performance of Large Language
Models"](https://arxiv.org/pdf/2408.02442) finds directly that format
restrictions (JSON mode, structured schemas) measurably degrade reasoning
performance versus unrestricted natural-language responses, especially on
tasks needing intermediate steps -- "stricter format requirements correlate
with degraded reasoning performance." A second, independent source
quantifies it: "forcing JSON during reasoning degrades accuracy by 10-15%...
the model should think first, then format the output" (from the same
reasoning-budget coverage cited in section 2).

**This directly complicates, not just supports, this project's own "no
logic should live in prose, it should live in the structure" thesis.** The
thesis is correct about where *enforcement* should live (Round 4's
`has_content` result, Round 5-8's operate-tier reliability). But Round 17
found, and this literature independently confirms, that the *presence* of
some prose space for the model to reason in -- even prose that enforces
nothing -- measurably helps it actually engage with the structure correctly.
Structure without any prose runway to reach it can perform worse than
structure with runway, even though the structure itself is identical either
way. Worth stating in the paper as a real qualification: structure should
hold the *constraint*; prose still appears to do real *cognitive* work in
getting the model to the constraint correctly, and removing it entirely is
not free.

## 4. Prompt wording doesn't move compliance; presence and position do

**Our finding**: Round 11 found rewording a framing paragraph (negation vs.
positive framing) flat, no effect either direction. Round 17 found removing
the framing paragraph entirely regressed badly. Together: it's not *what*
the framing says, it's *whether there's framing there at all* (and,
untested by this project, plausibly *where*).

**Literature**: directly relevant industry findings on instruction position
and hierarchy: "modern models show up to 61.8% performance variance when
instructions are reworded or repositioned, even when the semantic intent
stays constant," and "primacy and recency effects cause mid-prompt rules to
lose 30-50% compliance" ([The Instruction Position
Problem](https://tianpan.co/blog/2026/04/14/the-instruction-position-problem)).
[IHEval](https://arxiv.org/pdf/2502.08745) formally evaluates how models
follow an instruction *hierarchy* (system vs. user vs. embedded
constraints) as a distinct axis from instruction content. This supports
treating "is there a situating sentence here at all, and where" as a real,
separate variable from wording -- exactly what Round 11 and Round 17
independently found, from opposite directions, without a shared framework
connecting them until now.

## 5. Stale state between observation and action in multi-agent tool use

**Our finding, the central mechanism of this whole thread** (Round 13's
triangulation, Round 15's mutex fix): schema-conformance collapses when
state can change for reasons the querying agent didn't cause and couldn't
predict; a real mutex across the full query-decide-act cycle roughly halves
the collapse.

**Literature, and this is a precise, direct match**: ["Verified Tool Calls
Improve LLM Agent Reliability Under Non-Atomic
Failures"](https://arxiv.org/html/2608.02645v1) names exactly this failure
mode as one of four non-atomic tool-call problems: **"Stale Conflicts:
Another process modifies state between the agent's observation and its
action execution."** This is, almost verbatim, Round 13's triangulated
mechanism. That paper's fix is different from ours in shape but the same in
spirit -- not a lock, but "verify-before-retry": query real state
immediately before committing to an action, and only retry when
verification confirms the prior attempt didn't land. They report duplicate
actions dropping from 72% to 20% under high-fault conditions, and
verification (not retries themselves) as the source of the gain. **Worth
testing directly as an alternative to Round 15's mutex**: verify-then-act is
optimistic concurrency (cheaper, no serialization cost) where our mutex is
pessimistic (correctness by construction, real throughput cost, confirmed
this session -- Round 15 fit only 32 total attempts into 90 seconds versus
Round 9's 306). A real, not-yet-run comparison for this project: does
verify-before-retry recover most of the mutex's gain without its
throughput cost?

Separately, coverage of multi-agent race conditions in general reports
research establishing that **race conditions increase quadratically with
agent count** ([Handling Race Conditions in Multi-Agent
Orchestration](https://machinelearningmastery.com/handling-race-conditions-in-multi-agent-orchestration/)),
which would predict our 4-model swarm sits well past the point where
naive concurrency is viable at all -- consistent with how badly Round 9's
uncoordinated race condition performed before any fix was applied.

The formal theoretical frame for "the world may have changed since I last
observed it" is the **Partially Observable Markov Decision Process
(POMDP)** literature: "stale history may mislead agents in partially
observable environments" ([ASK in the
Dark](https://arxiv.org/html/2607.02686)), and general POMDP theory notes
that partial observability can make genuinely distinct states appear
identical to an agent, driving it toward suboptimal or malformed actions.
This project's operate-tier arena is, formally, exactly a multi-agent POMDP
once concurrency is introduced -- Round 9 through 16's whole investigation
can be read as an empirical POMDP case study that happened to be run before
anyone reached for that name for it.

## 6. A mediating arbiter for multi-agent shared-world state

**Our design** (`episode_arena.rs`): a single server holds the shared
world behind a mutex; every agent's proposed action is checked against it
by the same real interpreter (`commit_fires_transition`) regardless of who
proposed it -- the server is the sole arbiter of what's real.

**Literature**: DeepMind's [Concordia](https://arxiv.org/abs/2312.03664)
framework for generative-agent-based modeling formalizes almost exactly
this role, calling it the **"Game Master"** -- "a special agent... responsible
for simulating the environment where the agents interact," mediating
between LLM-driven agents and the actual state of a shared world. Different
implementation (Concordia's Game Master is itself an LLM-mediated
narrator; `episode_arena.rs`'s arbiter is a real deterministic interpreter,
no LLM in that role at all -- consistent with this whole project's "the
substrate is the real ground truth" discipline), same architectural
insight: a multi-agent shared world needs one authoritative mediator
checking every proposed change against real state, not agents trusting
each other's claims.

["Static Sandboxes Are Inadequate: Modeling Societal Complexity Requires
Open-Ended Co-Evolution in LLM-Based Multi-Agent
Simulations"](https://arxiv.org/pdf/2510.13982) argues, separately, that a
fixed, bounded simulation environment (exactly what this project's
house-world is -- a finite, exhaustible content chain, confirmed the hard
way in Round 18) limits what emergent multi-agent behavior can be observed;
real emergent complexity needs a world that keeps generating new possibility
space, not one a swarm can exhaust in under two minutes. This bears
directly on Jason's own framing ("enrich its complexity unintentionally") --
the current house-world is too small and too finite to be a real test of
that idea at scale; Round 9's collaborative build and Round 16's repeat of
it are real, but modest, instances of it, bounded by a content chain with an
actual ceiling.

## 7. Evolutionary prompt search needs a discriminating fitness landscape

**Our finding** (Round 18, the honest negative result): Round 11's GA never
found a signal because its isolated single-call harness sat at fitness
ceiling for every genome; the fix (a real live multi-agent arena) then hit
a second bug (world exhaustion) and, once fixed properly, a third problem
(short fresh sessions land in an easy-enough regime that everything scores
near-ceiling again).

**Literature**: this project's own approach -- mechanical text operators
(crossover, mutation) over a population of prompt genomes, fitness-scored
by real task performance -- is a real, established technique, not
invented from nothing:
[Promptbreeder](https://arxiv.org/pdf/2309.16797) and
[EvoPrompt](https://arxiv.org/pdf/2309.08532) are both published,
peer-reviewed instances of exactly this method, giving Round 11/18 real
academic precedent. More directly useful: a survey of automated prompt
engineering states the general best practice explicitly -- **"cover the
full difficulty spectrum by including easy cases (to avoid regression) and
hard cases (to drive improvement)"** -- which is precisely, in different
words, the diagnosis Round 18 reached on its own (near-ceiling scores on
every genome because every evaluation stayed in the easy regime). The
concrete fix Round 18 named but hasn't built -- seed each genome's fresh
evaluation from a mid-build snapshot instead of the pristine start -- is
exactly this best practice applied: manufacture hard-case coverage
deliberately rather than hoping a short session reaches it.

## 8. Two-stage extraction: reason freely, then format separately

**Not yet tried in this project.** Every dispatch so far (Rounds 4-18)
asked one model call to do both jobs at once -- decide what to do AND
emit it in the exact schema shape -- under `strict: true` in the same
call. Section 3 above already found, via "Let Me Speak Freely?," that
this coupling has a real cost.

**Literature**: this is a named, standard pattern, not a novel proposal
-- decouple deliberation from formatting into two sequential calls.
"The two-stage extraction pattern is motivated by empirical evidence
that strictly enforcing rigid output formats can degrade reasoning
performance... the model first performs [the reasoning] in free-form
text. A second LLM call then transforms this unstructured output into
the desired JSON format... adds one extra forward pass but works
reliably" (survey of structured-extraction pipelines, e.g. TimeTox and
similar clinical/legal extraction systems built exactly this way).

**Directly testable against this project's own numbers**: dispatch
the operate-tier decision in two calls -- (1) an unconstrained,
reasoning-enabled call that just answers "what do you want to do and
why" in free text against the real legal-action menu, (2) a second,
cheap, `strict: true` call that does nothing but extract that stated
intent into the exact schema shape, given the first call's answer as
context. This should, by the literature's own claim, avoid BOTH Round
4/14's reasoning-budget-exhaustion failure (reasoning has its own full
budget, not one shared with formatting) and Round 17's minimal-prompt
regression (the model gets real space to reason before ever touching
the schema) -- worth a real A/B against Round 15's mutex-only numbers,
not assumed to stack additively with it.

## 9. Idempotency keys for concurrent writes

**Our design** (`episode_arena.rs`): the mutex is the only defense
against a stale or duplicate write landing twice; there's no
deduplication of a genuinely repeated request (e.g. a client that
retries after a dropped response).

**Literature**: idempotency keys are the standard distributed-systems
answer to exactly this -- "client-supplied identifiers that scope a
request to a single logical operation... the naive check-then-act
pattern has a race condition where two concurrent requests with the
same key can both pass the existence check; every production
idempotency implementation should use atomic reservation or an
equivalent" (survey of idempotency patterns in distributed systems).
AWS's own guidance, cited in the same material, explicitly extends this
to agentic systems: "polling agents that claim tasks, emit
notifications, or trigger tool calls need dedupe keys and idempotent
claim protocols just as much as payment APIs do."

**Relevant, not yet a gap this project has actually hit**: the mutex
already prevents the specific race episode_arena.rs was built to
prevent (two proposals racing for the same guard). An idempotency key
would matter for a different, not-yet-tested failure mode -- a client
that times out waiting for a response and retries the identical
proposal, which the mutex alone doesn't protect against (it would
simply serialize the retry as a second, later, distinct attempt). Worth
naming as a real gap for a production version of this design, not
claimed as something Rounds 9-17 actually suffered from.

## 10. Self-consistency: majority vote over independent samples

**Not yet tried in this project.** Every operate-tier dispatch so far
has been one sample per turn, taken at face value.

**Literature**: [Wang et al.'s self-consistency
method](https://www.emergentmind.com/topics/self-consistency-sampling)
(the foundational technique, 2022) samples multiple independent
reasoning paths for the same question and takes the majority answer,
on the reasoning that "while any single chain might be incorrect,
aggregating multiple solutions can correct errors" -- real, large
accuracy gains on constrained-answer-space benchmarks (GSM8K +17.9%,
SVAMP +11.0%). Its own stated limitation matters here: "self-
consistency works well when outputs are relatively constrained (e.g.,
numerical or factual answers), but is less applicable to open-ended
generation, where answers cannot be easily compared for voting."

**This project's operate-tier action space is exactly the constrained
case the technique is suited to** -- a small, discrete, exactly-
comparable set of legal actions, not open-ended text. A directly
testable idea for the noisiest models (`gemini-2.5/3.1-flash-lite`,
near-zero conformance in isolation): sample N=3-5 responses per turn
instead of one, take the majority vote among schema-conformant answers
(discard non-conformant samples entirely, vote only among the ones
that parsed), and see whether this recovers meaningful conformance from
models otherwise unable to produce a single reliable sample. Real cost
tradeoff to weigh honestly, not assumed free: N calls per turn instead
of one, for models that are cheap per-call but not free.

## What this review actually changes

Nothing in this thread's own empirical findings is contradicted by any of
the above -- every citation either confirms a mechanism this project found
independently (sections 1, 2, 5, 7) or sharpens a real tension the project's
own thesis needs to carry forward honestly (section 3, the most important
one: format-forcing has a real cognitive cost, not just an enforcement
benefit). Five concrete, not-yet-run experiments this review surfaces
directly, worth naming as open follow-up in `VALAR-EVAL-2026-08-30.md`
rather than just here:

1. **Verify-before-retry vs. mutex** (section 5) -- an optimistic-
   concurrency alternative to Round 15's pessimistic lock, with a real
   throughput/correctness tradeoff to actually measure, not assume.
2. **Mid-build-snapshot seeding for the GA** (section 7) -- already named
   in Round 18's own follow-up, now with a citable best-practice grounding
   for why it should work, not just a guess.
3. **Two-stage extraction** (section 8) -- reason freely first in an
   unconstrained call, then a second cheap `strict: true` call formats
   that stated intent into the exact schema, decoupling the two costs
   Round 4/14 and Round 17 each hit separately.
4. **Idempotency keys** (section 9) -- a real gap for a production
   version of `episode_arena.rs`'s design, distinct from what the mutex
   already covers, not yet a failure mode this project has actually hit.
5. **Self-consistency majority voting** (section 10) -- sample N times
   per turn, vote among schema-conformant answers only, specifically
   for the noisiest models (`gemini-2.5/3.1-flash-lite`) whose action
   space is exactly the small, constrained-answer case the technique
   was shown to help most.
