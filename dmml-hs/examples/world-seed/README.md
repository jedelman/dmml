# world-seed

The real, persistent, iteratively-grown alpha world seed -- distinct
from `jev-driver-demo/cannon-mega`, which stays as the one-off capstone
that proved the cannon+driver pipeline (its own run's output was never
persisted, by design, and is left alone). This directory's output IS
meant to persist and grow, round after round, reviewed with Jason
between rounds rather than run-and-forget.

Same relational-architecture genre the cannon defines (`hall`/`forge`/
`vault`/`spur`/`fork` -- see `../../app/Cannon.hs`'s module doc), same
`jev-driver-demo/driver.py` orchestration. Nothing new about the
mechanism here; what's new is treating it as a persistent, resumable
world instead of a demo.

## Layout

- `chapter1/` -- the cannon-mega dungeon's 15 machines + its original
  seed (`mega-world.dmml`), copied in as this world's first chapter.
- `state/` -- chapter 1's real, persisted resolution: the 11 fact
  commits from re-running `chapter1/candidates.json` with `--dry-run`
  (deterministic first-legal-candidate selection), which reproduces the
  live capstone run's own choices exactly (left at every fork -- verified
  by direct comparison, not assumed). `--dry-run` was used here
  specifically because the outcome was already real, live-Jev-decided
  canon (see the `70d3ebd` commit in this repo's history); no need to
  re-spend a Jev call re-deciding something already settled.
- `wingD/` -- round 1's new content: a fourth wing, cannon-generated
  (`cannon hall room/dHall room/entry`, `cannon fork fork/d room/dHall
  path/dn path/ds`, `cannon forge room/dnForge path/dn`, `cannon vault
  room/dnVault room/dnForge`, `cannon spur room/dsSpur path/ds`), plus
  `wingD-seed.dmml` seeding its machines' initial `state` facts --
  needed because a freshly cannon-generated machine, unlike chapter 1's,
  isn't already covered by `mega-world.dmml`'s own seed facts. Forgetting
  this the first time round 1 was built produced exactly the
  single-segment-guard-adjacent landmine `dmml-authoring`'s SKILL.md
  warns about ("A machine with no `state` fact at all will refuse every
  `from -> to` transition unconditionally") -- caught by the dry-run
  sanity pass before it ever reached a live Jev call.
- `round-001/` -- round 1's config and its real, live-Jev-decided
  output (`audit.jsonl` has the full response incl. confidence/
  probabilities, not just the winner). Chose to breach dHall, then fork
  left at 0.95 confidence (favoring dnVault's sigil over dsSpur's relic
  and sealing dsSpur forever), then forge and vault.

## The resumable pattern

`driver.py` builds `RunState` fresh from `config["world_seed"]` on every
process invocation -- it does NOT read a `--world-dir`'s prior contents
back in as state. To make a new round continue from a prior one for
real (rather than re-deciding already-settled history), each round's
`candidates.json` lists, in `world_seed`, in order:

1. The original seed file(s).
2. Every fact-commit file the previous round(s) actually wrote (in the
   numbered order `driver.py` gave them).
3. Any new seed facts the new round's machines need (e.g.
   `wingD-seed.dmml`).

Already-resolved candidates from prior rounds are safe to keep listed
in `machines`/`candidates` too -- their guarded `from -> to` transitions
naturally go illegal once their `state` fact is no longer `sealed`, so
`driver.py`'s own legality check filters them out before they'd ever
reach Jev. No separate pruning step needed.

## Local-only, for now

Nothing here has been minted to atproto. This is `dmml-hs` local
firing only (`fire-transition`, `render-snapshot`, `check-guard-
literals`) -- publishing (`dmml-agent-nucleus/harness.py`) is a
separate, later, explicitly-decided step once the seed's shape is one
we actually want to make real on a shared network.

## Toolchain note

Building `fire-transition`/`cannon`/`check-guard-literals`/
`render-snapshot` needs a real GHC+cabal (`ghcup`) plus a JDK for
`DMML.Jgit`'s `libjvm` link dependency (see `../../dmml-hs.cabal`'s own
comment on this) -- set both in a gitignored `dmml-hs/cabal.project.local`:

```
package dmml-hs
  extra-include-dirs: <JAVA_HOME>/include, <JAVA_HOME>/include/linux
  extra-lib-dirs: <JAVA_HOME>/lib/server
```

and `LD_LIBRARY_PATH` must include `<JAVA_HOME>/lib/server` at runtime,
not just link time.
