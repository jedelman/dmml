#!/usr/bin/env bash
# Caller-driven cascade: fires a fixed, hand-specified set of transitions
# in a loop -- fire, apply the resulting commit into the running world,
# re-check, repeat -- until a full round produces nothing NEW or a step
# budget is hit. Pure orchestration on top of the existing
# `fire-transition` CLI -- no new engine primitive. DMML.Fire fires
# exactly one transition once; everything "cascading" here is this
# script re-invoking it.
#
# REAL FINDING, not anticipated going in (see dev-journal/2026-09-04-
# sense-machines-and-caller-driven-cascades.md for the full account):
# a naive version of this loop that treated "fire-transition exited 0"
# as the fire signal looped forever. DMML.Materialize's facts are
# collision-free -- an old (self, state, idle) fact is never retracted
# just because (self, state, smelted) got asserted alongside it, and
# DMML.Fire currently refuses to fire an EffectRetract at all (a real,
# disclosed Phase 3 gap -- see jedelman/dmml#4). So the implicit
# from->to guard (EXISTS(self state idle)) stays satisfied forever,
# and `smelt` "succeeds" every single round even after it has nothing
# new to do. The fix needs no new engine code: DMML.Materialize's own
# value-level dedup (`addAlternative` -- re-asserting an identical value
# is a no-op) means a transition whose fired output is byte-identical to
# its own last firing produced nothing new, whatever mayFire says --
# so THAT'S the real fixpoint signal, not exit code. Hashing each
# transition's own last output and stopping once it repeats is the
# whole fix, below.
#
# Usage: ./run.sh (run from dmml-hs/, needs fire-transition on PATH or
# FIRE_TRANSITION env var pointing at the binary)
set -euo pipefail

FIRE="${FIRE_TRANSITION:-fire-transition}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORLD_DIR="$(mktemp -d)"
trap 'rm -rf "$WORLD_DIR"' EXIT

cp "$HERE/world.dmml" "$WORLD_DIR/00-world.dmml"

hash_of() { echo "$1" | shasum -a 256 | cut -d' ' -f1; }

MAX_ROUNDS=5
round=1
last_smelt_hash=""
last_forge_hash=""
while [ "$round" -le "$MAX_ROUNDS" ]; do
  echo "=== round $round ==="
  new_this_round=0

  # Fixed, hand-specified attempt order -- not a generic scheduler walking
  # every declared machine's every transition; see the journal entry for
  # why that's a real, disclosed scope limit, not an oversight.
  worlds=("$WORLD_DIR"/*.dmml)
  if out=$("$FIRE" "$HERE/furnace.dmml" smelt smelts \
      $(printf -- '--world %s ' "${worlds[@]}") --param ore=ore/raw1 2>&1); then
    h=$(hash_of "$out")
    if [ "$h" != "$last_smelt_hash" ]; then
      echo "smelt fired (new):"
      echo "$out"
      printf '%s\n' "$out" > "$WORLD_DIR/$(printf '%02d' "$round")a-smelt.dmml"
      last_smelt_hash="$h"
      new_this_round=1
    else
      echo "smelt: legal, but identical to its own last firing -- nothing new, not counted"
    fi
  else
    echo "smelt: $out"
  fi

  worlds=("$WORLD_DIR"/*.dmml)
  if out=$("$FIRE" "$HERE/anvil.dmml" forge forges \
      $(printf -- '--world %s ' "${worlds[@]}") 2>&1); then
    h=$(hash_of "$out")
    if [ "$h" != "$last_forge_hash" ]; then
      echo "forge fired (new):"
      echo "$out"
      printf '%s\n' "$out" > "$WORLD_DIR/$(printf '%02d' "$round")b-forge.dmml"
      last_forge_hash="$h"
      new_this_round=1
    else
      echo "forge: legal, but identical to its own last firing -- nothing new, not counted"
    fi
  else
    echo "forge: $out"
  fi

  if [ "$new_this_round" -eq 0 ]; then
    echo "=== fixpoint: nothing NEW this round, stopping ==="
    break
  fi
  round=$((round + 1))
done

if [ "$round" -gt "$MAX_ROUNDS" ]; then
  echo "=== hit MAX_ROUNDS=$MAX_ROUNDS without reaching a fixpoint -- real bound, not a proof of termination ==="
fi
