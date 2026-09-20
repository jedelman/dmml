#!/usr/bin/env python3
"""CI gate: is the checked-in nose-baseline.json actually current?

Minting stays a manual, deliberate action -- someone runs baseline.py
locally (or via workflow_dispatch) and commits the result like any
other file change, reviewed like any other PR. This script is the
OTHER half: a required, cheap, no-API-call check that runs on every PR
and fails the build if the baseline is missing, malformed, or stale --
so "the baseline exists and was minted recently" is enforced the same
way a lockfile-in-sync check or a generated-file-matches-source check
would be, without spending a live Jev call on every PR just to verify
one already ran recently enough.

Usage:
    python3 check_baseline.py --baseline-path nose-baseline.json --max-age-days 30
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

REQUIRED_FIELDS = {"n", "mean", "sd", "updated_ref", "updated_at"}


def fail(msg: str) -> None:
    print(f"::error::code-nose baseline gate: {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--baseline-path", type=Path, default=Path(__file__).parent / "nose-baseline.json")
    ap.add_argument(
        "--max-age-days",
        type=float,
        default=30,
        help="fail if updated_at is older than this many days; 0 disables the age check",
    )
    ap.add_argument(
        "--min-n",
        type=int,
        default=1,
        help="fail if the baseline has been minted from fewer than this many scores",
    )
    args = ap.parse_args()

    if not args.baseline_path.exists():
        fail(
            f"{args.baseline_path} does not exist. Run "
            f"`python3 code-nose/baseline.py --scores <live stage2 output> --mint` "
            f"once from a main checkout to create it -- this is the one manual "
            f"step nothing here automates on purpose."
        )

    try:
        data = json.loads(args.baseline_path.read_text())
    except json.JSONDecodeError as e:
        fail(f"{args.baseline_path} is not valid JSON: {e}")

    missing = REQUIRED_FIELDS - data.keys()
    if missing:
        fail(f"{args.baseline_path} is missing field(s): {sorted(missing)}")

    if data["n"] < args.min_n:
        fail(f"baseline has n={data['n']}, below --min-n={args.min_n} -- not enough scores minted yet")

    if args.max_age_days > 0:
        try:
            minted_at = time.strptime(data["updated_at"], "%Y-%m-%dT%H:%M:%SZ")
        except ValueError:
            fail(f"updated_at {data['updated_at']!r} isn't in the expected UTC format")
        age_days = (time.time() - time.mktime(minted_at)) / 86400.0
        if age_days > args.max_age_days:
            fail(
                f"baseline is {age_days:.1f} days old (minted {data['updated_at']} "
                f"from ref {data['updated_ref']!r}), older than --max-age-days="
                f"{args.max_age_days}. Someone needs to run a fresh manual mint."
            )

    print(
        f"code-nose baseline OK: n={data['n']}, mean={data['mean']:.3f}, "
        f"sd={data['sd']:.3f}, minted {data['updated_at']} from {data['updated_ref']!r}"
    )


if __name__ == "__main__":
    main()
