#!/usr/bin/env python3
"""Real, EXECUTED self-test for the MINTING-vs-BINDING param split
(`driver.classify_params`, `unit_kind`, `refresh_minting_params`) -- the
change that let the rain actually fall.

Why this check exists at all, stated plainly: the driver used to skip
EVERY parameterized transition of a minted machine, on the honest ground
that binding a param is a real decision it would not guess. That is true
of a param a guard could resolve and exactly wrong about a param nothing
guards, which the open world says is a NAME FOR SOMETHING ARRIVING (see
`DMML.Ast.Effect`). The cost was invisible in every other check in this
repo: `cannon replenish` emitted a `fall(unit)` that parsed, rendered,
round-tripped and read correctly, and could never be chosen, so the flow
graph gained a source on paper while the world still ran down.

That failure mode is a classification mistake, not a Haskell one, so it
needs a check on this side of the line. Runs against the real driver
module -- no reimplementation of the reader -- and, when a `cannon`
binary is on hand (CANNON, or PATH), against the cannon's own real
output, which is the check that would have caught the original bug.

Run from dmml-hs/examples/jev-driver-demo/:

    python3 minting-params-selftest.py
    CANNON="$(cabal list-bin cannon)" python3 minting-params-selftest.py
"""

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from driver import (  # noqa: E402
    Candidate,
    RunState,
    classify_params,
    parse_machine_text,
    refresh_minting_params,
    unit_kind,
)

FAILURES: list[str] = []


def check(label: str, ok: bool) -> None:
    print(("  ok   " if ok else "  FAIL ") + label)
    if not ok:
        FAILURES.append(label)


def only(text: str, ident: str) -> dict:
    m = parse_machine_text(text)
    return next(t for t in m["transitions"] if t["ident"] == ident)


# A source: guarded by nothing, asserting its param into existence.
RAIN = """machine weather/w0
  states
    gathering
    spent

  transition fall(unit)
    gathering -> spent
    assert $unit `in` quarry/north
    assert self `cleared` mark/yes
    assert self `state` spent
    retract self `state`
"""

# A binding decision: the param names something that must already exist.
TAKE = """machine crew/hauler
  states
    idle
    working

  transition haul(rock)
    idle -> working
    guard $rock `in` quarry/north
    retract $rock `in` quarry/north
    assert self `state` working
    retract self `state`
"""

# What breeding produces routinely: one parent's signature over the
# other's effects, leaving a formal param nothing uses.
VESTIGIAL = """machine room/g5
  states
    gathering
    spent

  transition fall(unit)
    gathering -> spent
    assert self `took` ?rock
    assert self `state` spent
    retract self `state`
"""


def main() -> None:
    print("classification")
    guarded, minting, opaque = classify_params(only(RAIN, "fall"))
    check("a param nothing guards, asserted into being, is MINTING",
          minting == {"unit": "in quarry/north"} and not guarded and not opaque)

    guarded, minting, opaque = classify_params(only(TAKE, "haul"))
    check("a param a guard resolves is a BINDING decision, left to the binding question",
          guarded == ["rock"] and not minting and not opaque)

    guarded, minting, opaque = classify_params(only(VESTIGIAL, "fall"))
    check("a param used nowhere is OPAQUE -- not minted, and not called a binding",
          opaque == ["unit"] and not minting and not guarded)

    print("the substance's kind is read off the world, never invented")
    with tempfile.TemporaryDirectory() as tmp:
        wf = Path(tmp) / "world.dmml"
        wf.write_text(
            "commit setup\n"
            "  rock/1 `in` quarry/north\n"
            "  rock/2 `in` quarry/north\n"
            "  cart/7 `in` yard/south\n"
        )
        state = RunState(world_files=[str(wf)], machine_files=[], candidates={})
        check("two rocks in the quarry say what falls into it is a rock",
              unit_kind(state, "in", "quarry/north") == "rock")
        check("a relation nothing yet stands in yields no kind, rather than a guess",
              unit_kind(state, "in", "lake/deep") is None)

        print("each firing brings a NEW one")
        rain = Candidate(id="rain", machine="x", transition="fall", verb="b",
                         params={}, description="", minting_params={"unit": "in quarry/north"})
        silt = Candidate(id="silt", machine="y", transition="fall", verb="b",
                         params={}, description="", minting_params={"unit": "in quarry/north"})
        state.candidates = {"rain": rain, "silt": silt}
        state.known_nodes = {"rock/1", "rock/2", "cart/7"}

        refresh_minting_params(state)
        first = rain.params["unit"]
        check("the name carries its kind and how it arrived", first == "rock/fall0")
        check("two sources in one round do not mint the same node",
              rain.params["unit"] != silt.params["unit"])
        check("and neither collides with a node the world already knows",
              {rain.params["unit"], silt.params["unit"]}.isdisjoint(state.known_nodes))

        # What firing does: the new node becomes part of the world.
        state.known_nodes |= {rain.params["unit"], silt.params["unit"]}
        refresh_minting_params(state)
        check("next round brings a second rock, not the same one back",
              rain.params["unit"] != first)

        print("a machine with no minting params is untouched")
        plain = Candidate(id="rest", machine="z", transition="rest", verb="b",
                          params={"n": "fixed"}, description="")
        state.candidates = {"rest": plain}
        refresh_minting_params(state)
        check("its params are left exactly as they were", plain.params == {"n": "fixed"})

    print("the cannon's own output")
    cannon = os.environ.get("CANNON") or shutil.which("cannon")
    if not cannon:
        print("  SKIP no cannon binary (set CANNON or put it on PATH) -- "
              "the fixtures above still ran")
    else:
        out = subprocess.run(
            cannon.split() + ["replenish", "weather/w0", "in", "quarry/north"],
            capture_output=True, text=True, check=True,
        ).stdout
        guarded, minting, opaque = classify_params(only(out, "fall"))
        check("`cannon replenish` really emits a source this driver can fire "
              "-- the bug that shipped, caught at its source",
              minting == {"unit": "in quarry/north"} and not guarded and not opaque)

    print()
    if FAILURES:
        print(f"minting-params-selftest: {len(FAILURES)} FAILED:")
        for f in sorted(FAILURES):
            print("  - " + f)
        sys.exit(1)
    print("minting-params-selftest: all checks passed")


if __name__ == "__main__":
    main()
