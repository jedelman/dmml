# opus-world-test — Stillwater Lock

A companion to `../haiku-world-test/`, built the other way round: an
Opus-model agent authoring from the same
`dmml/.claude/skills/dmml-authoring/SKILL.md`, aiming for a world large
enough that the skill's own hardest rule (fire *every* transition
through to an end state) actually has something to catch.

A canal lock station on the tidal River Nire: gatehouse, lock chamber,
upper gate, sluice, mill race, corn mill, granary, signal mast, toll
ledger, a keeper, an apprentice, and a barge waiting below with timber
for the moor road.

- `world.dmml` — 169 facts over 30 subjects, 52 declared predicates.
- Seven machine files, 24 transitions total:
  `keeper/orrin`, `apprentice/wren`, `gate/upper`, `sluice/upper`,
  `mill/nire`, `barge/heron`, `mast/signal`.

Every file passes `validate-commit`. Every one of the 24 transitions was
fired for real, in the order below, through to each machine's terminal
state — `keeper/orrin` standdown, `apprentice/wren` resting,
`gate/upper` secured, `sluice/upper` logged, `mill/nire` banked,
`barge/heron` departed, `mast/signal` logged. No deadlock.

## The verified firing order

```
keeper/orrin      wake
apprentice/wren   learnGates
keeper/orrin      handKey        bearer=apprentice/wren
apprentice/wren   takePost
mast/signal       raiseAmber
barge/heron       hailStation
mast/signal       showGreen      vessel=barge/heron
gate/upper        unseal
sluice/upper      lift           gate=gate/upper
apprentice/wren   walkTheWalls   reach=reach/upper
gate/upper        swingOpen      warden=apprentice/wren
barge/heron       enterChamber   chamber=chamber/lock
sluice/upper      fillChamber    chamber=chamber/lock
barge/heron       rise
keeper/orrin      recordPassage  vessel=barge/heron entry=entry/heronone
mill/nire         engage
mill/nire         grind          grain=cargo/barley
mill/nire         bank
keeper/orrin      standDown
barge/heron       depart         gateway=gate/upper destination=road/moor
gate/upper        secure
sluice/upper      logDischarge   record=record/dischargeone
mast/signal       logSignal      record=record/greenone
apprentice/wren   standEasy
```

Two orderings inside that sequence are real, load-bearing constraints,
not stylistic:

- `sluice/upper.lift` must fire between `gate/upper.unseal` and
  `gate/upper.swingOpen` — it guards on `leaf/ajar`, which `unseal`
  creates and `swingOpen` retracts.
- `barge/heron.depart` must fire before `gate/upper.secure` — it guards
  on `leaf/open`, which `secure` retracts.

## The design rule this world is built around

`DMML.Fire.fireTransition` gates every firing through
`DMML.Retroconsistency.gateConsistentTree`, which scans **every**
non-`$param` guard on **every** machine — including the firing
transition's own — and refuses the firing if any guard that held before
is blocked after. That is what makes `../haiku-world-test/` deadlock.

The consequence, stated as an authoring rule: **a fact that a literal
(non-`$param`) guard requires may never be retracted by anything.** So
this world splits every fact into two kinds.

**Terminal facts** — asserted once, never retracted, safe to guard
literally. These are the unlock chain that makes the world interlock:
`station/stillwater access access/open`, `apprentice/wren skill
skill/gatework`, `apprentice/wren holds key/gatehouse`,
`station/stillwater warden apprentice/wren`, `mast/signal signal
signal/green`, `race/mill flow flow/running`, `chamber/lock level
level/high`, `mill/nire output sack/flourone`, `granary/stillwater stock
stock/flour`, `ledger/tolls entry entry/heronone`.

**Transient facts** — flipped by a retract/assert pair, and therefore
guarded only through `$param` (which `gateConsistentTree` excludes,
because a param guard's meaning depends on a specific firing's own
bindings) or not guarded at all: `gate/upper leaf`, `chamber/lock
level` in its `level/low` phase, `chamber/lock sill`, `sluice/upper
paddle`, `stone/grinding motion`, `barge/heron berth`, `cargo/barley
state`, and every machine's own `state` (the implicit `from -> to`
guard is not part of `transitionGuards`, so it is never scanned).

There are no negated guards anywhere in this world, on purpose — see
the note below.

## Two refusals worth knowing about, verified directly

Both were reproduced against a three-fact throwaway world, not reasoned
about:

1. A transition that retracts a fact its **own** literal guard requires
   is refused — `fire: refused -- firing would break the following
   currently-held guard(s) ... actor/one's openDoor (predicate status)`.
   The machine does not even need a second machine to deadlock against
   itself.

2. The classic one-shot idiom — `guard not self \`seen\` marker/here`
   plus `assert self \`seen\` marker/here` — is **also** refused, for the
   mirror-image reason: the negated guard held before and is blocked
   after. Negated guards are effectively unusable in a live world unless
   their pattern involves a `$param`.

## What the world actually demonstrates

- Cross-machine gating on literal facts: `apprentice/wren.learnGates`
  and `mill/nire.engage` were both refused ("guards do not currently
  hold") before, respectively, `keeper/orrin.wake` and
  `sluice/upper.lift` had fired, and succeeded after.
- Cross-subject effects: `keeper/orrin.handKey` moves
  `holds key/gatehouse` from the keeper to the apprentice, which is what
  unlocks `apprentice/wren.takePost` on a different machine.
- Minting fresh nodes by firing: `entry/heronone` (keeper),
  `sack/flourone` (mill), `record/dischargeone` (sluice),
  `record/greenone` (mast) — none exist in `world.dmml`.
- Multi-hop literal guards: `keeper/orrin.standDown` guards
  `mill/nire \`output\` sack/flourone \`at\` granary/stillwater`, so the
  keeper cannot end his watch until the mill machine has banked its
  flour; `apprentice/wren.standEasy` guards
  `ledger/tolls \`entry\` entry/heronone \`records\` barge/heron`, a node
  the keeper's own transition minted.
- A chained retract: `mill/nire.bank`'s
  `retract self \`grinds\` stone/grinding \`motion\` motion/spinning`
  consumes two facts, each independently cited to a different prior
  commit.
