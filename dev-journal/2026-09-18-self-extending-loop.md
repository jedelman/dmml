# 2026-09-18 — The self-extending loop, and what it found

Two asks: fix the CI cabal pin, then run the self-extending loop. Fitness
explicitly deferred — Jason: *"We can build fitness in once we're saturated on
richness."*

## 1. CI: the cabal pin, fixed and verified

`.github/workflows/dmml-hs-ci.yml` pinned `cabal-version: 3.8.1.0`, which ships a
Hackage TUF root predating Hackage's current signing keys. Every run died at
`cabal update`:

```
<repo>/root.json does not have enough signatures signed with the appropriate keys
```

...before `cabal build all` ever ran. **This package has never been built by CI,
not once.** That is the structural reason so many caveats in it say "uncompiled,
disclosed" — it was never a sandbox quirk, it was pinned into the workflow.

Bumped to `3.10.3.0`. **Verified, not assumed**: downloaded cabal 3.10.3.0 in this
sandbox and ran `cabal update` against real Hackage —
`Package list of hackage.haskell.org has been updated. The index-state is set to
2026-09-18T09:13:58Z.` The failing step now passes with the version CI will use.

What is still unverified: whether `cabal build all` then succeeds on a GitHub
runner, in particular the library's `extra-libraries: jvm` link. No CI run has
ever reached that step, so nothing is known about it either way. The first green
`cabal update` will tell us.

## 2. The self-extending loop

`driver.py` grew an `extend` config block. At the end of each round — after that
generation's winners have applied — the driver fires the **cannon** at the live
frontier, mints new architecture, seeds its initial state, and registers its
zero-parameter transitions as ordinary candidates for the next round.

Everything it adds is *ordinary*: an ordinary Surface machine file, an ordinary
world commit seeding its state, ordinary candidates through the same
dry_fire/dedup/grouping path. Nothing downstream knows a machine was minted
mid-run rather than written into the config. That is the whole point.

**Growth policy, fully deterministic** (no randomness anywhere, so a `--dry-run`
rehearsal mints exactly what a live run will):

- Parent A is the newest **offspring**, so generation N+1 crosses generation N and
  the lineage deepens instead of re-stamping five hand-authored shapes forever.
- Parent B cycles over the **seed** machines — a fixed-length list.
- Crossover modes and room variants cycle by index.
- Anchor cycles through frontier nodes so growth spreads.

One bug caught in a dry run before the live one: parent B was originally cycled
over the whole pool with `seq % len(pool)`. The pool grows by one every time `seq`
does, so the index locks and breadth silently collapses — eight straight
generations all crossed with `room-wForge`. Cycling over a fixed-length list fixes
it. Also made cannon-minted nodes count against the existing `max_minted_nodes`
cap: once `extend` is on, growth is the dominant source of new world, and a node
budget that quietly stopped covering it would read as a bound without being one.

### Live run, real Jev calls

Seeded with the 5-machine `cannon-dungeon`, capped at 8 minted machines. Ran to
completion: **9 rounds, 13 firings, 8 machines minted mid-run off the live
frontier**, stopping on the growth cap rather than on a fixpoint — which is the
proof that the loop is genuinely self-extending. All 8 minted machines pass
`check-guard-literals`.

The architecture does get stranger with depth. `room/g7` (union of g6 with
`room-wVault`) is a single room that is *either* a forge yielding the gold key,
*or* an east passage, *or* — if you already hold the key — a vault yielding a
sigil. Three mutually exclusive readings of one room; no parent had that. The mode
cycling does real work in both directions: **chimera narrows, union broadens.**
`room/g3` came out a bare hall. Some offspring are boring, which is what a frontier
is for.

## 3. What the run found: recombination defeats the fork

The delve chose **WEST** at the entry fork in round 1. That is supposed to seal the
east permanently — `Cannon.hs`'s own haddock calls the fork *"the one place a
chooser downstream faces a real, permanent decision."* By round 8 it was breaching
eastern rooms.

The chain is exact:

1. Round 1: `fork-west` fires. `path/west` cleared, `fork/main` → `chosen`. The
   fork can never fire again; `path/east` is never cleared.
2. `room/g0` was bred from `room-eSpur × fork-main` (chimera) and inherited
   `assert path/east \`cleared\` mark/yes` — the fork's **yield**, without the
   fork's **lock**.
3. Round 7: `room_g5-goRight` fires → `path/east` cleared.
4. Round 8: `breach-eHall` becomes legal. Round 9: `breach-eSpur`.

**The generalization, which is the real finding:** a fork's two transitions are
mutually exclusive because they share ONE `unchosen -> chosen` lock on ONE machine.
That exclusivity is a property of the *machine*, not of the transitions. Breed a
transition onto a different machine and it arrives with its effect under a
different lock. So **any invariant this language enforces through a shared
lifecycle lock is breakable by breeding a transition out of the machine that holds
it.** Exclusivity is per-machine; crossover moves transitions between machines.

Two honest readings, and both are true:

- **Emergent richness.** The dungeon routed around its own foreclosure. An
  offspring did something neither parent could — precisely what recombination is
  for.
- **A soundness hole.** The single most meaningful permanent choice in the dungeon
  is not actually permanent once breeding is on.

**Deliberately not prevented**, per Jason's stated priority (richness now, fitness
later). The fix that would work — refusing to copy any transition that shares its
lock with a sibling — would forbid breeding forks at all, and forks are the
richest parents in the corpus. Recorded prominently in `DMML.Recombine`'s haddock
instead, with the real remedy for whoever wants the guarantee back: **encode the
foreclosure as a FACT** (a `path/east \`sealed\` mark/yes` the offspring must also
guard on), because a fact survives recombination and a lifecycle lock does not.

## Still open

- **No fitness, by design.** `pool`/`extend` fire a frontier; which offspring
  becomes real is entirely the chooser's call. Deferred until saturated on richness.
- **Frontier detection is a text-scan** for the `cleared` convention, same class of
  heuristic as the existing minted-node tracking, and disclosed alongside it. It
  would not see a `cleared` fact some later commit retracted — nothing in this
  dungeon does that yet.
- **Parameterized transitions on minted machines are skipped**, never auto-bound —
  same reasoning `discover_fact_native` already gives. Printed and logged, not
  silently dropped.
- **`cabal build all` on a runner** is still unproven (see §1).
- **`EffectGraft` union-recombination in a live run** — still only the copy case.
