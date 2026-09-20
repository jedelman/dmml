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

## What's deliberately not here yet

- **Stage 2 itself** (the batched Jev call, one `noul` per candidate,
  z-scored against `nose-baseline.json`, ranked output, non-blocking PR
  annotation). Both scripts above were tested against synthetic scores
  in place of a real Stage 2 call — see the test transcript in the
  commit that added `baseline.py` for exactly what was exercised.
- Cluster A currently only compares `.py` and `.hs` files, and only
  within a single naming family per basename — it doesn't (yet) cluster
  files with *different* names that turn out to implement the same
  logic (e.g. a Python and a Haskell version of the same check).
- No CI workflow file wires any of this up yet — there's no point
  writing `.github/workflows/code-nose.yml` before Stage 2 exists to
  call from it; it would just be an unrun stub.
