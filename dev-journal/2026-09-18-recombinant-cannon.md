# 2026-09-18 — The recombinant cannon

Thread #2 from the 09-18 generativity session, the one left explicitly open:
"turn the cannon from a template-stamp into a breeder that crosses two parent
machines into structurally novel offspring, handling transition-name collisions
by renaming at authoring time. The graft primitive (#3) is its enabler and is
done; #2 itself is not built."

Built. `src/DMML/Recombine.hs` + `breed`/`pool` subcommands on `app/Cannon.hs`.

## What the cannon was, and what it is now

The four variants and the fork made the cannon a **template stamp**: every shot
came from a shape hand-authored in `Cannon.hs`, so the space of possible
architecture was exactly as large as that one file. `breed` and `pool` make it a
**breeder**: it takes two machines that already exist — including two the cannon
fired earlier, or two an earlier breeding produced — and crosses them into
offspring whose shape is in neither the file nor either parent.

The input language is the output language (ordinary Surface `.dmml`, via the
real `parseMachineSurface`), so this is closed under itself. Verified by hand to
a third generation: bred two offspring together, bred that result against a
mega-dungeon room, all clean through `check-guard-literals`. That closure is the
difference between a generator with a fixed vocabulary and one whose vocabulary
grows with its own output.

## The problem that made this real work: state alignment

A machine has ONE current state. Parent A's lifecycle is `sealed -> open`,
parent B's is `unchosen -> chosen`. Naively union the state lists and you get a
four-state machine with two disjoint lifecycles — not a crossover, a corpse:
once A's transition fires the machine sits in `open`, and every B transition is
dead forever because nothing can ever put it back in `unchosen`. It would parse,
type-check, render, and never work.

So B's state space is aligned onto A's, positionally, and every copied B
transition's `from`/`to` is rewritten through that map. The offspring's
lifecycle IS A's; what it inherits from B is B's relational content. Surplus B
states (a longer-lived parent) append and extend rather than fork.

Positional alignment is a disclosed choice — it assumes states are declared in
lifecycle order, which every machine in this project happens to follow, but it
is not derived from the transition graph.

## The bug this session's own output caught

First working version printed a union offspring whose `goLeft` transition read
`sealed -> open` and, three lines later, `assert self \`state\` chosen`. A
transition names its destination **twice** — on the `from -> to` line and in the
lifecycle effect that records the arrival as a fact — and the first cut remapped
only the first. The result asserts a state the offspring does not declare, from
which nothing can transition out. Same dead-machine failure as naive state
unioning, arriving through a different door.

Worth recording precisely how invisible it was: it parses, it renders, it
round-trips through the real parser byte-for-byte. The round-trip check passes
on the broken version. Only an assertion that the two namings agree catches it,
and that assertion is now `lifecycleAgrees` in the selftest — confirmed to go
red when the fix is reverted, not just assumed to.

The same insight fixed a latent category error nearby: the `open` in
`assert self \`state\` open` is stored as a `TermNode` but names a STATE, not a
node in the world, so node re-derivation must skip it entirely.

## The three crossover modes

- **union** — all of A's transitions, then all of B's, remapped. The
  authoring-time equivalent of a double graft. Note what a union offspring
  actually IS, since the obvious reading is wrong: because alignment rebases B
  onto A's single lifecycle, a union of two two-state parents is a **menu**, not
  an accumulation — all transitions leave the same state and arrive at the same
  one, so exactly one ever fires and the rest are foreclosed. A room that could
  be a vault OR a fork OR a forge, and firing decides which. Same permanent real
  choice the `fork` variant exists to create, reached by another route.
- **splice K** — single-point crossover with the transition list read as a
  chromosome. Varying K gives a family of siblings from one pair.
- **chimera** — the one no graft can do. Crossover INSIDE a transition, at the
  only seam a transition has: it says what must hold (guards) and what follows
  (effects). A chimeric transition keeps A's identity, lifecycle and guards and
  takes B's world effects. Real output, vault × fork: a breach that requires the
  forge cleared AND a gold key, and opens the west path. Neither parent had it.

Lifecycle effects are deliberately not crossed — that would be the alignment bug
in miniature. The rule: the lifecycle belongs to A, the consequences come from B.

`pool` fires a whole frontier from one pair (every mode, both orientations,
structurally deduplicated) rather than a single shot — six distinct candidates
from vault × fork. Orientation is genuinely asymmetric: chimera A×B keeps A's
guards, B×A keeps B's.

Also `reanchor`: a bred offspring inherits BOTH parents' literal reachability
guards, so by default it sits at a confluence, only opening where both lineages
are cleared. Real architecture, sometimes wanted — but a breeder that can only
produce confluences cannot extend a frontier. `reanchor` re-roots guards of the
shape `<node> \`cleared\` mark/yes`, which is a CONVENTION of this dungeon
language, not a property of the grammar. Stated as such.

## Verification, and what changed about verification in this repo

**A real toolchain finally existed.** GHC 9.4.7 plus megaparsec, aeson,
parser-combinators and unordered-containers, all from Ubuntu's `libghc-*-dev`
packages rather than Hackage. That sidesteps the TUF failure every prior session
hit, and it means `DMML.Surface` compiled for the first time in this lineage.
Consequences beyond this feature:

- All seven existing `examples/jev-driver-demo/*-selftest.hs` build and pass
  against the **genuine** modules — no stubs.
- `spawn-fire-selftest.hs` carried a caveat that its round-trip through the real
  `parseMachineSurface` was UNVERIFIED, with an instruction to close it once a
  real build existed. Closed properly: the check is now actually written and
  actually calls the parser, rather than the comment being edited to claim it.
  The `spawn` grammar addition in `Surface.hs` is exercised by that call, so it
  is no longer uncompiled or untested either.

`cannon-breed-selftest.hs` asserts universal properties over all 44 offspring
breedable from three fixtures in every pairing and orientation — states
declared, lifecycle agreement, unique idents, no parent identity surviving in a
node literal, real round-trip — plus targeted checks on alignment, surplus
states with a colliding ident, collision renaming, chimera's novelty,
asymmetry, splice range refusal, reanchor, and pool dedup. 27 checks, all green.

It is registered as a **cabal executable** and wired into CI, unlike its
neighbours under `examples/`. Deliberate: those were written in stub sandboxes;
this one runs against real modules, and the invariant it protects is invisible
to every other check in that job.

## Found, not fixed: CI has never actually built this package

`dmml-hs CI` is red on `main` and has been for at least the last five runs. It
fails at the `cabal update` step:

    <repo>/root.json does not have enough signatures signed with the appropriate keys

cabal 3.8.1.0's bundled Hackage TUF root is too old for Hackage's current keys.
The job dies before `cabal build all` ever runs, so no CI run has compiled this
package — which is the structural reason so many caveats in this repo say
"uncompiled, disclosed." The likely one-line fix is bumping `cabal-version` in
`.github/workflows/dmml-hs-ci.yml` to ≥3.10, whose root.json is current.

Not done here: it is outside "build the recombinant cannon," and it cannot be
verified from this sandbox — the only way to test a workflow change is to push
it and watch. Flagged for a deliberate call.

## Still open

- **Self-extending loop** — wire the cannon into the round loop so fresh
  architecture is fired off the live frontier each generation. Breeding now makes
  this much more interesting (the frontier could breed with itself), but the
  loop itself is still not built.
- **`EffectGraft` union-recombination in a live run** — still only the copy case
  has ever fired.
- **Multi-point crossover** — only single-point splice exists. Nothing structural
  blocks it.
- **Fitness** — there is no selection pressure anywhere. `pool` fires a frontier;
  which offspring becomes real is still entirely Jev's choice, by design. A
  breeder without selection is half an evolutionary loop, and the missing half is
  deliberate, not overlooked.
