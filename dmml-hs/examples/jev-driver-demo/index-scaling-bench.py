#!/usr/bin/env python3
"""ReadIndex against the pairwise loop it replaced: same answer, and
what it costs at scale.

Two jobs, and the first one is the important one.

**Agreement.** `may_conflict` is the specification of when two firings
can interact; `ReadIndex` answers the same question without visiting
every pair. This script builds candidate populations of five sizes and
asserts the two produce the IDENTICAL ordered pair list. A disagreement
here is a corrupted round in the driver -- two candidates fired in the
same generation that can in fact invalidate each other -- so the assert
is the point and the timings are the footnote. (The driver carries the
same check against real rounds under `DMML_INDEX_SELFCHECK=1`; this one
reaches sizes no real run has produced yet.)

**Cost.** Pruning removed the simulations but not the PAIRS -- the loop
still asked `may_conflict` about every one, which is quadratic in
candidates however cheap each answer is. Inverting the question makes
the cost track the conflicts actually found rather than the pairs
considered.

Synthetic, and calibrated rather than invented: the touch/read set sizes
and the wildcard fraction are sampled from the largest round of a real
`cannon-fanout` run (31 candidates; 1-4 reads and 1-3 touches each; 2
wildcard reads of 96; no unparsed guards). The subject pool grows with
candidate count, because a world with more candidates in flight is a
bigger world. What this does NOT model: a round where most candidates
really do collide, which is the case the index cannot help with and
where it degrades to the pairwise cost it started from.

    python3 index-scaling-bench.py
"""

import importlib.util
import random
import sys
import time
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "driver", Path(__file__).resolve().parent / "driver.py"
)
driver = importlib.util.module_from_spec(spec)
sys.modules["driver"] = driver
spec.loader.exec_module(driver)

# Measured off a real round, not chosen.
READ_SIZES = [1] * 12 + [2] + [3] * 12 + [4] * 6
TOUCH_SIZES = [1] * 12 + [2] * 13 + [3] * 6
PREDS = [f"p{i}" for i in range(105)]  # dmml's real predicate count
WILD_FRACTION = 2 / 96


def population(n, rng):
    subjects = [f"n/{i}" for i in range(n * 2)]
    touched, reads = {}, {}
    for k in range(n):
        cid = f"c{k}"
        touched[cid] = {
            (rng.choice(subjects), rng.choice(PREDS))
            for _ in range(rng.choice(TOUCH_SIZES))
        }
        rs = set()
        for _ in range(rng.choice(READ_SIZES)):
            pred = rng.choice(PREDS)
            rs.add(
                (None, pred)
                if rng.random() < WILD_FRACTION
                else (rng.choice(subjects), pred)
            )
        reads[cid] = rs
    return list(touched), touched, reads


def pairwise(ids, touched, reads):
    """What the driver did before the index: ask about every pair."""
    return [
        (a, b)
        for i, a in enumerate(ids)
        for b in ids[i + 1 :]
        if driver.may_conflict(touched[a], reads[b])
    ]


def indexed(ids, touched, reads):
    """What it does now: ask the slots A touches who reads them."""
    idx = driver.ReadIndex(reads)
    rank = {c: i for i, c in enumerate(ids)}
    out = []
    for i, a in enumerate(ids):
        for b in sorted(idx.readers_of(touched[a]), key=rank.__getitem__):
            if rank[b] > i:
                out.append((a, b))
    return out


def main():
    print(
        f"{'candidates':>11}  {'pairs':>10}  {'conflicts':>9}  "
        f"{'pairwise':>10}  {'indexed':>10}  {'speedup':>8}"
    )
    for n in (50, 200, 800, 3200, 12800):
        ids, touched, reads = population(n, random.Random(7))
        t = time.perf_counter()
        want = pairwise(ids, touched, reads)
        t_pairwise = time.perf_counter() - t
        t = time.perf_counter()
        got = indexed(ids, touched, reads)
        t_indexed = time.perf_counter() - t
        if got != want:
            print(
                f"FAIL: ReadIndex disagrees with may_conflict at n={n} "
                f"({len(got)} pairs vs {len(want)})",
                file=sys.stderr,
            )
            return 1
        print(
            f"{n:>11}  {n * (n - 1) // 2:>10}  {len(want):>9}  "
            f"{t_pairwise:>9.3f}s  {t_indexed:>9.3f}s  {t_pairwise / t_indexed:>7.1f}x"
        )
    print("\nindex-scaling-bench: identical pair lists at every size")
    return 0


if __name__ == "__main__":
    sys.exit(main())
