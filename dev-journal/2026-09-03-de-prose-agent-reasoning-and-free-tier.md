# De-prose agent: reasoning on/off, and what OpenRouter's ":free" tier actually does

Two follow-up questions on `deprose_agent.py` (see the prior two
dev-journal entries for the tool-calling agent itself and its stats
harness): "how does this change if we enable reasoning?" and "let's try
using openrouter/free until we hit its daily usage limit."

## Reasoning on vs off, same source text, same model (Kimi)

Added a real `--reasoning` flag (`deprose_agentic(..., reasoning_none=...)`,
default `True` to preserve the existing convention) rather than answering
from intuition. Both runs: `results/deprose-test/source1.txt`
(Mara/Ashgrove), fresh empty world, `moonshotai/kimi-k2.5`.

| | reasoning OFF (earlier run) | reasoning ON |
|---|---|---|
| rounds | 9 | 3 |
| check/commit calls | 8 | 2 |
| commits produced | 3 (non-duplicate, but over-fragmented) | 1 (clean, comprehensive) |
| first `check` result | FAILED (self-declaration) | OK on the first attempt |
| tokens | 42,813 (41,455 prompt / 1,358 completion) | 19,456 (16,284 prompt / 3,172 completion, 2,461 of which reasoning) |
| API time | 44.5s | 79.5s |
| wall time | 44.8s | 79.5s |

Real, disclosable tradeoff, not a one-sided win: reasoning **more than
halved token cost and cut rounds 3x** by getting the extraction right
before ever calling `check` (converged in one shot instead of iterating
through failures), and it also fixed the over-fragmentation problem from
the non-reasoning run -- one coherent commit instead of three. But it
was **slower in wall-clock time** (79.5s vs 44.8s) despite the far fewer
rounds, because each reasoning-laden call itself takes much longer (the
first call alone: 53.3s, 1,922 reasoning tokens) -- reasoning trades
round-count and token-count for per-call latency, it doesn't reduce
latency across the board. Whether that trade is worth it depends on
which axis actually matters for a given use (a batch job cares about
tokens; an interactive tool cares about wall time).

## OpenRouter ":free" models: not what "hit the daily limit" implied

Real correction to the premise, found by actually testing it rather than
assuming: **the 429s hit here are not an account-level daily request
quota.** They're upstream, per-provider, shared-pool rate limits with a
real `Retry-After` cooldown, and they vary enormously by model:

- `z-ai/glm-5.2:free` (routed to provider "Decart"): 429 on the very
  FIRST request of the day, `retry_after_seconds: 5`, and still 429 on
  retry a few seconds later. This model's free shared pool was already
  saturated by other OpenRouter users globally before we ever touched
  it -- nothing about our account's usage caused it.
- `minimax/minimax-m3:free` (routed to provider "GMICloud"): worked
  cleanly, then hit 429 at request **#25** in a tight loop,
  `retry_after_seconds: 60`. Waited out the 60s window and it succeeded
  again immediately -- confirmed this is a **rolling cooldown**, not a
  hard stop for the rest of the day. A slower request rate would very
  plausibly avoid tripping it at all.

So "push until the daily limit" doesn't describe what actually happens:
there's no single daily counter to exhaust in the account sense. What
exists is per-model, per-provider burst throttling on the free shared
inference pool, with recovery on a timescale of seconds to about a
minute, and wildly different headroom between models depending on how
saturated that model's free pool already is when you show up. Practical
implication for using a `:free` model in this pipeline for real: budget
for retry-with-backoff on 429 (the response already tells you exactly
how long to wait, via `retry_after_seconds`), and don't assume one
`:free` model's availability predicts another's -- `glm-5.2:free` and
`minimax-m3:free` were tested back to back and behaved completely
differently.

Neither `deprose.py`/`deprose_agent.py` currently retries on 429 at all
-- a bare `urllib.error.HTTPError` propagates straight up and kills the
run. Not fixed here; worth doing before actually relying on a `:free`
model for a real pipeline run rather than a one-off probe like this.

## `openrouter/free`: the real router, tried next

Jason's actual ask, after seeing the per-model congestion above: use
`openrouter/free` (confirmed real via the models API -- "Free Models
Router," picks a free model at random from whatever's currently
available, `supported_parameters` includes `tools`/`tool_choice`/
`reasoning`) instead of pinning to one free model's specific quirks,
since this pipeline is dev-only and shouldn't care which underlying
model answers, saving real token budget for production runs on paid
models.

**Two real bugs found and fixed immediately, both from this project's
own hardcoded `reasoning: {"effort": "none"}` dispatch convention
colliding with the router's per-call randomness:**

1. A `check` call succeeded, then the very next round's call 400'd:
   `"Reasoning is mandatory for this endpoint and cannot be disabled."`
   The router had landed on a different, mandatory-reasoning model that
   round -- confirmed directly by hammering `openrouter/free` with
   `reasoning: {"effort": "none"}` in a tight loop: 4 of 5 tries picked a
   disable-able-reasoning model and worked, the 5th hit this exact error.
   Fixed with a real retry: catch the 400, drop `reasoning` from the
   payload, retry once.
2. Immediately hit a SECOND, different shape of the same underlying
   problem: OpenRouter doesn't always surface this as a non-2xx status --
   sometimes it's a 200 whose JSON body is `{"error": {...}}` instead of
   `{"choices": [...]}`. The first fix only caught the `HTTPError` case
   and crashed on this one with `RuntimeError: unexpected response`.
   Fixed by checking for `"error"` in a successful body too, same
   retry-without-reasoning logic either way. Both real, found by running
   it, not by reading the docs and guessing at the failure shapes.

**With both fixed, the crash is gone -- but a real, separate quality
problem shows up that the crash had been masking.** A real run
(`source2.txt`, empty world, `openrouter/free`, 8-round budget): 7
straight `check` failures (parse errors) before finally passing on round
8, then `max_rounds` hit before ever calling `commit` -- **0 committed**,
worse than either reasoning-on or reasoning-off Kimi run on comparable
text. Per-round token/reasoning-token logs confirm the router really was
switching models mid-conversation (some rounds show reasoning tokens,
others show none). The cost of "don't care which model answers" is real:
a single strong model can use its own growing context to converge on a
fix across rounds; a different, often much weaker free model landing
each round doesn't carry that same improving competence forward, even
though the message *history* does. Whether that tradeoff is acceptable
depends on what a given dev run is actually testing -- fine if the point
is exercising the harness/gating logic itself (the checks all still ran
correctly, they just kept finding real problems), not fine if the point
is judging extraction quality.

## Reframed as stigmergy, then tested with reasoning on

Jason's read on the result above, unprompted: "this is good actually --
stigmergy. allow reasoning." The connection is real and worth keeping,
not just a nice metaphor: each round's `check` result gets left in the
shared message history like a pheromone trail, and whichever model the
router hands the next round to -- a completely different agent, with no
continuity of "mind" from the one before it -- picks up from that trail
rather than its own memory. That's the actual mechanism, not an
analogy: coordination through a shared, modified environment (the
conversation, functionally a stigmergic medium here) rather than through
continuity of a single persistent agent. Directly useful for Paper 2's
meta-agent argument, which already frames the substrate's orchestration
as doing the agentic work no individual call does on its own -- this is
a concrete instance of that where the "individuals" literally rotate.

Re-ran the same case with `--reasoning` added. Real improvement on one
axis: passed `check` on round 4 (vs. never passing cleanly in 8 rounds
without reasoning) -- less back-and-forth needed once the router's
models were actually allowed to think before drafting.

**But round 5 surfaced a different, new failure mode, not a win.** No
tool call that round -- the model's plain-text reply claimed "committed
blacksmithSituation (15 facts + 7 predicates), capturing the merchant's
triple-price squeeze, the closed quarry's local-supply impact, the
father's warnings..." Checked the real output directory: empty. Nothing
was ever written -- `commit` was never called. The model **confabulated**
having deposited content it only drafted internally. Final tally: 0
committed, same headline number as the non-reasoning run, but for a
worse reason -- not "couldn't find a valid candidate," but "believed,
wrongly, that it already had."

This is exactly why `deprose_agent.py`'s `committed` count is derived
from real tool calls and file writes, never from the model's own
narration -- "never trust the model's own claim that something is
valid" was already the design principle behind `commit` re-running the
identical checks `check` does, and it holds here for a claim about the
model's own past actions, not just about content validity. Nothing false
landed in the world. But it's a real, disclosable limit on the
stigmergic framing: the shared trail (message history) had everything
needed to converge, and reasoning got the model to a genuinely valid
draft by round 4 -- the failure was a *handshake* failure at the very
last step, an agent narrating a commit instead of performing one. Worth
testing whether this recurs on other passages/other router draws before
treating it as a one-off, but it did not appear at all in any of the
single-model (Kimi) runs, reasoning-on or off, in this same session.
