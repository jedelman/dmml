# code-nose — Stage 1 (mechanical candidate generation)

Design context: `claude-memory/conversations/` sessions on using Jev
(the `choice`/`score`/`noul` judgment API this repo already runs 30+
live batched calls through, in `dmml-hs/examples/jev-driver-demo/`)
as a **nose**, not a decision engine — it flags "something's off here"
per candidate, independently and in parallel (`noul`), ranked against
its own running baseline rather than judged against a fixed threshold.
It never blocks, never auto-fixes.

This directory is Stage 1 only: **no Jev call, no API key, fully
deterministic.** It finds *candidates* mechanically and hands each one
to Stage 2 already phrased as a comparison ("X and Y agree on
everything except this"), because the description handed to a `noul`
call is what determines whether the score means anything — a lesson
paid for directly in this project's `noul` work (a flat, undiscriminating
score turned out to be a bookkeeping-vs-world bug in the description,
not a limit of the model).

## What it finds

- **Cluster A — sibling-family drift.** Files sharing a basename across
  different top-level directories (`compliance/dispatch.py`,
  `compliance-parallel/dispatch.py`, ...) are compared against one
  reference per family (star topology, not all-pairs — see the comment
  in `cluster_a_candidates`: all-pairs on this repo's own compliance*
  families produced 129 candidates on a single run, already past the
  ~30-60 question ceiling measured for one live Jev call elsewhere in
  this project). Only code-value hunks are surfaced; comment-only
  divergence is expected and ignored.
- **Cluster B — structural-template outliers.** Files sharing a naming
  prefix (`Check*.hs`, `Retro*.hs`, `Atproto*.hs`) are checked against
  the shape the family implies (doc comment, `getArgs`, an empty-args
  usage branch, `exitFailure` on the failing path) and against the
  family's own median line count.
- **Cluster C — comment-claim vs. code-value.** A comment naming a
  number ("capped at 8000", "now 12000") next to an assignment of a
  different number. Deliberately does **not** try to judge staleness —
  it surfaces the pair and lets Jev or a human decide. Backward
  references ("bumped **from** 8000") are excluded on purpose: a real
  false positive on this repo's own `compliance-surface/dispatch.py`
  (comment said "Bumped from 8000", code correctly said 12000) caught
  this before it shipped — see the comment on `BACKWARD_REFERENCE_RE`.

## Usage

```sh
python3 stage1_cluster.py --root .. > candidates.ndjson
python3 stage1_cluster.py --root .. --cluster A --cluster B > candidates.ndjson
```

Output is newline-delimited JSON, one candidate per line:

```json
{"cluster": "A", "basename": "dispatch.py",
 "files": ["compliance/dispatch.py", "compliance-parallel/dispatch.py"],
 "anchor_lines": [[60, 66], [36, 40]],
 "claim": "compliance/dispatch.py and compliance-parallel/dispatch.py share 63% of their lines ... but disagree here: MAX_TOKENS = 8000 / MAX_TOKENS = 12000",
 "context_identical": "63% line-level similarity outside this hunk"}
```

`claim` is written already as the comparison Stage 2 hands to a `noul`
call — never a raw diff, never file-level bookkeeping.

## Diff scoping

```sh
python3 stage1_cluster.py --root .. --changed-since origin/main > candidates.ndjson
```

Emits only candidates whose `files` intersect the merge-base diff
against `REF`. A pre-existing sibling-family disagreement nobody's PR
touched is noise, not signal — and it also keeps each PR's candidate
count well under whatever a single Jev batch can hold (see the star-
topology note above: unscoped, this repo alone produces 63; a typical
PR touches far fewer files than the whole `compliance*` family). If
git or `REF` can't be resolved, it falls back to an unscoped run with a
stderr warning rather than silently reporting zero — going quiet on a
tooling failure is worse than an occasional noisy run.

## Baseline (`baseline.py`)

Persisted running mean/sd for Stage 2's nose scores (Welford/Chan
online variance — merges a new batch into the checked-in file without
re-reading historical raw scores). Ranking a candidate's score against
this, not a fixed threshold, is the same fix the DMML project's own
`noul` interest scores needed: they lived in 0.21-0.59 for 122 live
calls, and every absolute cutoff either built nothing or built
everything.

**The mint gate:** the script always computes and prints what the
baseline *would* become — a PR can preview this freely. It only
*writes* the checked-in `nose-baseline.json` when `--mint` is passed
**and** the current ref is `main` (checked via `GITHUB_REF_NAME` under
GitHub Actions — Actions checks out a detached HEAD even on `push`
events, so a plain `git rev-parse --abbrev-ref HEAD` would misreport
"HEAD" there — falling back to a real git call otherwise). A PR branch
that calls `--mint` by accident gets a clear stderr refusal and exit 0,
never a write and never a broken build. This is deliberate: one noisy
PR's scores must never be able to skew what every other PR is ranked
against, so only a merge to `main` moves the shared baseline.

```sh
# preview only, from anywhere -- never writes:
python3 baseline.py --scores stage2_scores.ndjson

# real mint -- only takes effect when run from CI on main:
python3 baseline.py --scores stage2_scores.ndjson --mint
```

`--scores` accepts either one float per line or ndjson objects with a
`"score"` field (Stage 2's expected output shape), from a file or stdin.

## Stage 2 (`stage2_score.py`)

One batched Jev call, one `noul` per Stage 1 candidate — independent,
parallel, never a `choice` between them. The request shape (endpoint,
auth, the `noul` question's `criteria: {"true": ..., "false": ...}`
shape) and the ranking math (z-score against a blended baseline+prior,
then a sigmoid) are carried over **verified**, not reconstructed from
memory, from this repo's own real Jev integration: `driver.py` on
branch `claude/recombinant-cannon-opr929` (`call_jev_batch`'s `noul`
type, `interest_probability`). That code has actually been run live
against Jev; this one follows its request/response handling line for
line rather than guessing at a schema for an external API.

**Deliberately not carried over:** that branch's prior (mean 0.412, sd
0.086, weight 12) is a measured fact about *its* domain — 122 live
scores on a fantasy-narrative interest question. It says nothing about
a code-review `noul` distribution. This script's prior defaults to
`--prior-weight 0` (none at all); with no baseline and no prior it
ranks by raw `noul` score and says so on stderr, rather than silently
standardizing against borrowed numbers. Once `code-nose` has minted its
own real baseline the same way that project measured its own, a real
prior can be set from what's actually observed here.

```sh
# rehearse the whole pipeline with no API key (deterministic stand-in scores):
python3 stage1_cluster.py --root .. --changed-since origin/main \
  | python3 stage2_score.py --dry-run --repo-label "jedelman/dmml"

# live, and feed the scores straight into a baseline mint on main:
python3 stage1_cluster.py --root .. --changed-since origin/main \
  | python3 stage2_score.py --api-key "$TYPESAFE_API_KEY" --scores-only \
  | python3 baseline.py --mint
```

Every full-pipeline path above (cold-start ranking, truncation
reporting past `--max-questions`, the scores-only hand-off into
`baseline.py --mint`, a second batch merging into an existing baseline)
was run for real against this repo's own Stage 1 output in `--dry-run`
before this shipped — see the commit that added `stage2_score.py`.

Two real live batches have now been spent against Jev (49 real `noul`
scores; see `dev-journal/2026-09-20-code-nose-first-live-run.md`) —
the request/response code path and the ranking math are no longer just
tested, they've been run for real.

## CI (`.github/workflows/code-nose.yml`)

Two jobs, deliberately independent:

- **`baseline-gate` (required).** Runs `check_baseline.py` — cheap, no
  API call, no LLM. Fails the PR if `code-nose/nose-baseline.json` is
  missing, malformed, or older than `--max-age-days` (default 30).
  **Minting stays manual on purpose** — nothing in CI ever mints or
  auto-commits the baseline; a person runs `baseline.py --mint` from a
  `main` checkout whenever they want to refresh it, and commits the
  result like any other file change, reviewed like any other PR. This
  job is the other half: it enforces that someone actually did that
  recently, the same way a lockfile-in-sync check does — without
  spending a live Jev call on every PR just to verify one already ran.
- **`review` (advisory, never fails the build).** Stage 1 scoped to
  the PR's diff (`--changed-since origin/<base>`) → Stage 2 against
  the result → a ranked markdown table posted to the job summary.
  Falls back to `--dry-run` with a `::warning::` if `TYPESAFE_API_KEY`
  isn't available — expected for a fork PR, since GitHub Actions
  withholds repo secrets from `pull_request`-triggered workflows on
  forks. Every branch of this logic (empty candidates, no-key fallback,
  real live key) was run for real, outside the workflow, before this
  shipped — see the commit that added the workflow file.

**Current real state, disclosed rather than hidden:** `check_baseline.py`
fails right now, honestly, because `code-nose/nose-baseline.json`
doesn't exist yet — the two live batches so far were run from a
non-`main` branch, so `baseline.py`'s own gate correctly refused to
write it (see the first-live-run journal entry). The very first real
mint, from `main`, is the one manual step this repo needs before
`baseline-gate` goes green.

## What's deliberately not here yet

- **The first real mint on `main`.** Everything is wired and tested;
  nobody has yet run `baseline.py --mint` from an actual `main`
  checkout, so `nose-baseline.json` doesn't exist in the repo and the
  `baseline-gate` job will fail until someone does.
- Cluster A currently only compares `.py` and `.hs` files, and only
  within a single naming family per basename — it doesn't (yet) cluster
  files with *different* names that turn out to implement the same
  logic (e.g. a Python and a Haskell version of the same check).
- Truncation past `--max-questions` currently drops candidates in
  Stage 1's emission order, not by any priority — a 100-candidate PR
  loses the same way regardless of which findings matter more.
- The `review` job's YAML was validated for structure (`yaml.safe_load`)
  and every embedded shell step was both `bash -n` checked and actually
  *executed* against real repo state outside the workflow (see that
  commit) — but the workflow file itself has never been run by GitHub
  Actions, which per this repo's own stated practice elsewhere
  (`dmml-hs-ci.yml`'s CI-fix history) is the only way to fully verify a
  workflow change. Worth confirming on its first real PR run rather
  than assuming.
