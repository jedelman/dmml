#!/usr/bin/env python3
"""Test-only stand-in for the real `fire-transition` binary, so the
driver's loop mechanics (legality, dedup, budgets, audit log) can be
smoke-tested without a GHC/cabal build. Not shipped as part of the
real pipeline -- delete once the real binary is available.

Mimics: smelt is legal exactly once (needs world.dmml's initial
`stocked` fact, which nothing here ever re-asserts); forge is legal
once smelt has fired. Both produce a fixed commit text so the driver's
dedup-by-hash logic gets exercised on the second attempt.
"""
import sys
from pathlib import Path

args = sys.argv[1:]
machine, transition, verb = args[0], args[1], args[2]
worlds = [args[i + 1] for i, a in enumerate(args) if a == "--world"]
world_text = "\n".join(Path(w).read_text() for w in worlds)

if transition == "smelt":
    if "ore/raw1" not in world_text:
        sys.exit(1)
    print("commit round\n  ore/raw1 `refinedInto` ingot/batch1\n  smithy/furnace `state` smelted")
elif transition == "forge":
    if "refinedInto" not in world_text:
        sys.exit(1)
    print("commit round\n  ingot/batch1 `title` \"A forged ingot\"\n  smithy/anvil `state` forged")
else:
    sys.exit(1)
