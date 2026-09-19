#!/usr/bin/env python3
"""Choose a seed pool by MEASURING it, not by collecting more machines.

Motivated by a measurement that killed the obvious plan. Handed 81
machines with 78 distinct shapes, `check-fertility`'s structural walk
saturates immediately:

    pool  2 -> walk  2      pool 16 -> walk 11
    pool  4 -> walk 14      pool 32 -> walk 11
    pool  8 -> walk 11      pool 81 -> walk 11

Eighty-one machines buy exactly the horizon of four. But different
8-machine SLICES of that same set give walks from 2 to 26, and a
searched pool reaches 83. **Horizon is a property of which shapes are
present, not how many.** So the pool is worth searching for, and the
search is cheap -- `check-fertility` answers in milliseconds and needs
no Jev call at all, so a pool can be chosen before a single token is
spent on it.

Scores three things, because they are not the same question and a pool
can be good at one and useless at another:

  walk       how many generations before a bred shape repeats -- the
             structural horizon, i.e. how long a run can stay novel
  fertile    whether every generation in the cycle can still be built
             on -- a long walk of unanchorable machines is the "growth
             without reachability" failure, measured once as 60 rooms
             and 2 firings
  rank       independent circuits in the substance-flow graph -- whether
             matter circulates or the world runs down

Usage:
  select-pool.py <machines.txt> [--trials N] [--size N]... [--require-fertile]
"""

import argparse
import pathlib
import random
import re
import shlex
import os
import statistics
import subprocess
import sys


def fertility_binary() -> list[str]:
    return shlex.split(os.environ.get("CHECK_FERTILITY", "check-fertility"))


def probe(pool: list[str], generations: int) -> tuple[int, bool, int] | None:
    """(walk, fertile, cycle rank) for one pool, or None if it did not run."""
    try:
        out = subprocess.run(
            fertility_binary() + ["--generations", str(generations)] + pool,
            capture_output=True, text=True, timeout=60,
        ).stdout
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return None
    walk = re.search(r"(\d+) generation\(s\)", out)
    # No walk line at all means the probe did not run -- a bad path, an
    # unparseable machine -- and that is NOT a pool with a walk of zero.
    # Reading it as one reported "0 of 300 fertile" against a measured
    # 48%, which is the only reason the mistake was visible. A silent
    # failure that produces a plausible number would not have been.
    if walk is None:
        return None
    rank = re.search(r"independent circuits (\d+)", out)
    return (int(walk.group(1)), "check-fertility: FERTILE" in out, int(rank.group(1)) if rank else 0)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("machines", type=pathlib.Path, help="file listing one machine .dmml path per line")
    ap.add_argument("--trials", type=int, default=400)
    ap.add_argument("--size", type=int, action="append", default=None, help="pool sizes to sample (repeatable)")
    ap.add_argument("--generations", type=int, default=400)
    ap.add_argument("--require-fertile", action="store_true",
                    help="only report pools whose every bred generation can still be anchored on")
    ap.add_argument("--seed", type=int, default=20260919, help="deterministic: the same seed searches the same pools")
    ap.add_argument("--out", type=pathlib.Path, default=None, help="write the winning pool here, one path per line")
    args = ap.parse_args()

    machines = [l.strip() for l in args.machines.read_text().splitlines() if l.strip()]
    if not machines:
        print(f"{args.machines}: no machines listed", file=sys.stderr)
        sys.exit(2)
    sizes = args.size or [8, 10, 12, 14]
    rng = random.Random(args.seed)

    rows, failed = [], 0
    for _ in range(args.trials):
        pool = rng.sample(machines, min(rng.choice(sizes), len(machines)))
        r = probe(pool, args.generations)
        if r is None:
            failed += 1
            continue
        rows.append((*r, pool))

    if failed:
        print(f"warning: {failed} of {args.trials} probes produced no verdict "
              "-- check the machine paths resolve from this working directory", file=sys.stderr)
    if not rows:
        print(f"fatal: no probe returned a verdict. Is '{' '.join(fertility_binary())}' built, "
              "and do the listed paths resolve from here?", file=sys.stderr)
        sys.exit(2)

    keep = [r for r in rows if r[1]] if args.require_fertile else rows
    if not keep:
        print(f"no pool met the bar out of {len(rows)} sampled "
              f"({sum(1 for r in rows if r[1])} were fertile). Widen --trials or drop --require-fertile.")
        sys.exit(1)

    # Fertile first, then horizon, then circulation: an unanchorable pool
    # with a long walk is worse than a short anchorable one, because the
    # walk it has is a walk of machines nothing can attach beyond.
    keep.sort(key=lambda r: (r[1], r[0], r[2]), reverse=True)
    walks = [r[0] for r in rows]
    print(f"sampled {len(rows)} pools of size {sizes}: "
          f"{sum(1 for r in rows if r[1])} fertile ({100 * sum(1 for r in rows if r[1]) // len(rows)}%), "
          f"median walk {statistics.median(walks):.0f}, max {max(walks)}")
    print()
    print(f"{'walk':>5} {'fert':>5} {'rank':>4}  pool")
    for walk, fert, rank, pool in keep[:5]:
        print(f"{walk:>5} {str(fert):>5} {rank:>4}  {', '.join(pathlib.Path(p).stem for p in pool)}")
    if args.out:
        args.out.write_text("\n".join(keep[0][3]) + "\n")
        print(f"\nwinning pool -> {args.out}")


if __name__ == "__main__":
    main()
