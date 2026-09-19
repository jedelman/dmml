#!/usr/bin/env python3
"""The conflict pruning, against the exhaustive probe it replaced.

`group_into_generations` decides which of a round's candidates may fire
together. It used to answer by simulating every pair; it now probes only
the pairs that could possibly interact, using the read sets
`scan-candidates` reports from the real parsed transition. That is a
soundness claim, not a speed claim: a pair wrongly called independent is
two firings applied in the same round that can invalidate each other --
a corrupted world, silently, with nothing else in this toolchain able to
see it.

So run the driver twice over the same scenario, once as it ships and
once with `may_conflict` forced to `True` (which is exactly the
pre-pruning behaviour, every pair probed), and require the two
transcripts to match line for line. The dry-run chooser is
deterministic, so this compares the whole run -- every binding, interest
score, generation boundary and minted machine -- not just the
partitions.

And require the pruning to have DONE something. A check that passes
because nothing was pruned is a check that would also pass with the
feature removed, which this project has shipped once already and does
not intend to again.

    SCAN_CANDIDATES=... FIRE_TRANSITION=... python3 conflict-pruning-selftest.py
"""

import importlib.util
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
# cannon-fanout by default: the densest committed scenario, so the most
# pairs to get wrong. Any scenario's rz.json can be passed instead.
SCENARIO = HERE / "cannon-fanout" / "rz.json"
NOISE = re.compile(r"/tmp/[A-Za-z0-9._\-]+")
GROUPING = re.compile(r"^  grouping:.*$", re.M)

# Forces every pair to be probed, whatever the read sets say -- the
# behaviour this selftest is comparing against.
#
# It patches `ReadIndex`, which is what the pair loop actually consults.
# An earlier version of this harness patched `may_conflict` instead, and
# was a silent no-op from the moment the index landed: `may_conflict`
# stayed correct, stayed tested, and stopped being on the path. That is
# exactly the failure the probe-count assertion below exists to catch,
# and it did catch it.
UNPRUNED = """
import importlib.util, sys
spec = importlib.util.spec_from_file_location("driver", {driver!r})
driver = importlib.util.module_from_spec(spec)
sys.modules["driver"] = driver
spec.loader.exec_module(driver)


class ProbeEverything(driver.ReadIndex):
    def __init__(self, reads):
        super().__init__(reads)
        self._all = set(reads)

    def readers_of(self, a_touch):
        return self._all


driver.ReadIndex = ProbeEverything
driver.main()
"""


def run(script: Path | None, selfcheck: bool) -> str:
    env = dict(os.environ)
    # The index selfcheck compares the index against `may_conflict`, so
    # it cannot run in the harness that forces `may_conflict` to True --
    # the two checks contradict each other by construction. It belongs
    # on the shipped run, which is where it means something.
    env.pop("DMML_INDEX_SELFCHECK", None)
    if selfcheck:
        env["DMML_INDEX_SELFCHECK"] = "1"
    with tempfile.TemporaryDirectory() as world:
        cmd = [sys.executable]
        cmd += [str(script)] if script else [str(HERE / "driver.py")]
        cmd += [str(SCENARIO), "--dry-run", "--world-dir", world]
        proc = subprocess.run(
            cmd, capture_output=True, text=True, cwd=HERE, timeout=1800, env=env
        )
    if proc.returncode != 0:
        print(proc.stdout[-4000:], file=sys.stderr)
        print(proc.stderr[-4000:], file=sys.stderr)
        raise SystemExit(f"conflict-pruning-selftest: driver exited {proc.returncode}")
    return proc.stdout


def totals(transcript: str) -> tuple[int, int]:
    probes = sum(int(n) for n in re.findall(r"(\d+) probe\(s\) run", transcript))
    pruned = sum(int(n) for n in re.findall(r"(\d+) pair\(s\) pruned", transcript))
    return probes, pruned


def main() -> int:
    global SCENARIO
    if len(sys.argv) > 1:
        SCENARIO = Path(sys.argv[1]).resolve()
    shipped = run(None, selfcheck=True)

    with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False) as fh:
        fh.write(UNPRUNED.format(driver=str(HERE / "driver.py")))
        harness = Path(fh.name)
    try:
        exhaustive = run(harness, selfcheck=False)
    finally:
        harness.unlink()

    probes, pruned = totals(shipped)
    if pruned == 0:
        print(
            "FAIL: nothing was pruned, so this comparison proves nothing. "
            "Either the scenario stopped producing conflicts to prune, or "
            "the read sets stopped arriving (scan-candidates on PATH?).",
            file=sys.stderr,
        )
        return 1

    def normalize(t: str) -> str:
        return GROUPING.sub("", NOISE.sub("WORLD", t))

    if normalize(shipped) != normalize(exhaustive):
        import difflib

        print("FAIL: pruning changed the run.", file=sys.stderr)
        diff = difflib.unified_diff(
            normalize(exhaustive).splitlines(),
            normalize(shipped).splitlines(),
            "exhaustive",
            "pruned",
            lineterm="",
        )
        print("\n".join(list(diff)[:60]), file=sys.stderr)
        return 1

    e_probes, _ = totals(exhaustive)
    if e_probes <= probes:
        print(
            f"FAIL: the unpruned run probed {e_probes} pair(s), no more than the "
            f"shipped run's {probes}. The harness is not actually disabling the "
            "pruning, so the transcripts matching proves nothing.",
            file=sys.stderr,
        )
        return 1

    print(f"  probed {probes} pair(s), pruned {pruned}; exhaustive probed {e_probes}")
    print("conflict-pruning-selftest: identical transcripts, and the pruning did something")
    return 0


if __name__ == "__main__":
    sys.exit(main())
