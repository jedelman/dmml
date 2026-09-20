#!/usr/bin/env python3
"""Persisted running mean/sd for Stage 2's nose scores, updated as a CI
step -- so a candidate is ranked against "everything this nose has ever
seen," never against a fixed absolute cutoff. Same fix the DMML project
already needed for Jev's `noul` interest scores: 122 live scores lived
in 0.21-0.59 and every absolute threshold either built nothing or built
everything, while standardizing against the run's own mean/sd (then a
sigmoid) worked at every scale.

THE GATE: this script always COMPUTES what the baseline would become
if updated with a new batch of scores, and always prints that result.
It only ever WRITES the checked-in baseline file when both:
  1. `--mint` is passed, and
  2. the current ref is the repo's main branch (default: "main").
A PR run can and should call this without --mint (or on a non-main
ref) to preview how its own findings would land against the baseline
-- but the checked-in file itself only moves on main, so one noisy PR
branch can't skew what every other PR is ranked against.

Merging uses Chan et al.'s parallel-variance combination so an existing
baseline (n0, mean0, M2_0) and a new batch (n1, mean1, M2_1) combine
into one without ever re-reading the historical raw scores -- the
persisted file is the only state that needs to survive.

Usage:
    # preview only, never writes:
    python3 baseline.py --scores stage2_scores.ndjson

    # mint for real (only takes effect on main):
    python3 baseline.py --scores stage2_scores.ndjson --mint
"""

from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

DEFAULT_BASELINE_PATH = Path(__file__).parent / "nose-baseline.json"


@dataclass
class RunningStats:
    n: int = 0
    mean: float = 0.0
    m2: float = 0.0  # sum of squared deviations from the running mean

    @property
    def sd(self) -> float:
        if self.n < 2:
            return 0.0
        return math.sqrt(self.m2 / (self.n - 1))

    def to_dict(self, **extra) -> dict:
        return {"n": self.n, "mean": self.mean, "m2": self.m2, "sd": self.sd, **extra}

    @classmethod
    def from_dict(cls, d: dict | None) -> "RunningStats":
        if not d:
            return cls()
        return cls(n=d.get("n", 0), mean=d.get("mean", 0.0), m2=d.get("m2", 0.0))


def batch_stats(scores: list[float]) -> RunningStats:
    """Welford's single-pass streaming mean/variance over one batch."""
    stats = RunningStats()
    for x in scores:
        stats.n += 1
        delta = x - stats.mean
        stats.mean += delta / stats.n
        delta2 = x - stats.mean
        stats.m2 += delta * delta2
    return stats


def merge_stats(a: RunningStats, b: RunningStats) -> RunningStats:
    """Chan, Golub & LeVeque (1979) parallel combination -- combines two
    running (n, mean, M2) triples without re-touching either's raw data."""
    if a.n == 0:
        return b
    if b.n == 0:
        return a
    n = a.n + b.n
    delta = b.mean - a.mean
    mean = a.mean + delta * b.n / n
    m2 = a.m2 + b.m2 + delta * delta * a.n * b.n / n
    return RunningStats(n=n, mean=mean, m2=m2)


def load_scores(path: Path | None) -> list[float]:
    text = path.read_text() if path else sys.stdin.read()
    scores: list[float] = []
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        # accept either a bare float per line, or an ndjson object
        # carrying a "score" field (Stage 2's expected output shape)
        try:
            scores.append(float(line))
            continue
        except ValueError:
            pass
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(obj, dict) and "score" in obj:
            try:
                scores.append(float(obj["score"]))
            except (TypeError, ValueError):
                continue
    return scores


def current_ref(main_branch: str) -> str:
    # GitHub Actions checks out a detached HEAD even on `push` events, so
    # `git rev-parse --abbrev-ref HEAD` would report "HEAD", not the
    # branch -- GITHUB_REF_NAME is what Actions sets instead, and it's
    # right in exactly the case a plain git call is wrong.
    env_ref = os.environ.get("GITHUB_REF_NAME")
    if env_ref:
        return env_ref
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--abbrev-ref", "HEAD"],
            capture_output=True, text=True, check=True,
        ).stdout.strip()
        return out or "HEAD"
    except (subprocess.CalledProcessError, FileNotFoundError, OSError):
        return "HEAD"


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--scores", type=Path, default=None, help="ndjson/float-per-line file (default: stdin)")
    ap.add_argument("--baseline-path", type=Path, default=DEFAULT_BASELINE_PATH)
    ap.add_argument("--main-branch", default="main")
    ap.add_argument("--mint", action="store_true", help="write the merged result to --baseline-path (gated to --main-branch)")
    args = ap.parse_args()

    scores = load_scores(args.scores)
    if not scores:
        print("# baseline: no scores read -- nothing to merge", file=sys.stderr)
        sys.exit(0)

    existing_raw = None
    if args.baseline_path.exists():
        existing_raw = json.loads(args.baseline_path.read_text())
    existing = RunningStats.from_dict(existing_raw)
    incoming = batch_stats(scores)
    merged = merge_stats(existing, incoming)

    result = merged.to_dict(
        updated_ref=current_ref(args.main_branch),
        updated_at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        batch_n=incoming.n,
        batch_mean=incoming.mean,
    )
    print(json.dumps(result, indent=2))

    ref = current_ref(args.main_branch)
    if not args.mint:
        print(f"# baseline: preview only (pass --mint to write) -- ref is {ref!r}", file=sys.stderr)
        return
    if ref != args.main_branch:
        print(
            f"# baseline: refusing to write -- ref {ref!r} is not "
            f"{args.main_branch!r}. Baseline only mints from the main "
            f"branch so one PR can't skew what every PR is ranked "
            f"against. (Computed result shown above, not written.)",
            file=sys.stderr,
        )
        return

    args.baseline_path.write_text(json.dumps(result, indent=2) + "\n")
    print(f"# baseline: minted to {args.baseline_path} (n={merged.n}, mean={merged.mean:.3f}, sd={merged.sd:.3f})", file=sys.stderr)


if __name__ == "__main__":
    main()
