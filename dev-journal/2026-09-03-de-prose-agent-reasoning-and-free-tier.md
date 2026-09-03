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
