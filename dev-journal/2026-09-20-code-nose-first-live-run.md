# 2026-09-20 — code-nose's first live Jev call

Jason: "the API key is in the env. Pew pew!" First real spend against
Jev through `code-nose/stage2_score.py`, after two prior sessions built
Stage 1 (mechanical candidate generation), diff-scoping, the mint-gated
baseline, and Stage 2 (the batched `noul` call) all against `--dry-run`
stand-ins only. Full design context: `claude-memory/conversations/`
sessions on using Jev as a nose, not a decision engine.

## Two real batches, deliberately different in kind

**Cluster B (structural-template outliers, 9 candidates — `app/Check*.hs`,
`Retro*.hs`, `Atproto*.hs` missing a shared CLI shape element):**
```
9 questions, 0.6s, 2353->250 tokens, noul decisiveness 0.127
scores: 0.50-0.65 (every single one above 0.5)
```

**Cluster A (sibling-family drift, 40 of 54 candidates — the real
semantic content, `compliance*/{dispatch,score}.py` forks):**
```
40 questions, 0.758s, 11887->1097 tokens, noul decisiveness 0.325
scores: 0.32-0.84
```

## The finding that justifies the whole baseline design, on the first real call

Cluster B's scores are compressed into a narrow high band (0.50-0.65)
with weak decisiveness (0.127) — a naive "flag above 0.5" rule would
have flagged all nine, indiscriminately. Cluster A, which carries real
semantic disagreement rather than mere shape/line-count comparison,
discriminates far more sharply (decisiveness 0.325, range 0.32-0.84).
**Same pattern the DMML fiction-interest work already found** (weak
decisiveness on thin/bookkeeping descriptions, sharper on ones carrying
a real fact to discriminate on) — reproducing on an entirely different
domain, code review rather than narrative, is real evidence the
mechanism generalizes rather than being an artifact of one task.

Combined: **n=49, mean 0.633, sd 0.100.**

## Real signal, not noise

Top of the real ranking: `compliance/dispatch.py`'s `MAX_TOKENS = 8000`
disagreeing with three different siblings' `12000` scored 0.81, 0.78,
0.76 — the same real divergence, surfaced consistently across every
pair it appears in, exactly the class of finding Stage 1's cluster-A
design was built to catch (a fix or a deliberate change landing in one
fork and not propagating to its siblings). Bottom of the ranking (0.32)
was a `score.py` divergence in a cosmetic string-formatting line —
correctly ranked as the least interesting of the batch.

## The mint gate held on real data

Piped both batches' raw scores into `baseline.py --mint` from this
session's actual branch (`claude/code-review-jev-design-b6gns2`, not
`main`):

```
# baseline: refusing to write -- ref 'claude/code-review-jev-design-b6gns2'
# is not 'main'. (Computed result shown above, not written.)
```

Computed-but-not-written result matched the hand-computed stats above
exactly (mean 0.6332653061224488, sd 0.10017459248527957 pre-Bessel vs.
post — `nose-baseline.json` itself is still empty; the real baseline
mints for the first time whenever this branch's work lands on `main`
and a CI run calls `baseline.py --mint` there for real). Raw scored
output saved to `dev-journal/artifacts/2026-09-20-code-nose-first-live-run.ndjson`
(49 rows) so the real mint, whenever it happens, isn't the only record
of this measurement.

## Open

- Only 49 of 63 total Stage 1 candidates on this repo have ever been
  scored live (cluster C is empty on this repo; cluster A's 54 were
  capped to 40 by `--max-questions`'s default). The other 14 haven't
  been asked about at all yet.
- No CI workflow file exists yet to make any of this automatic — this
  was a manual `pew pew` from a live session, not a PR-triggered run.
- Two batches is not enough to say anything about calibration drift
  over time — that needs the baseline to actually accumulate across
  real PRs, not two runs in one sitting.
