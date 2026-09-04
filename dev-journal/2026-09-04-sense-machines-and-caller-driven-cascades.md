# Sense machines confirmed as pure content; a real caller-driven-cascade bug

Follow-up to Phase 2/3 (`dev-journal/2026-09-03-phase-2-3-effect-
generalization-and-firing.md`). Jason asked what the generalized `Effect`
does for token efficiency on complex-world authoring, and separately
wanted sense machines / recurrent-cascading effects provisioned for
builder agents. Instruction: sketch it all as real DMML first, caller
drives any looping, cascades stay undecided for now.

## Sense machines: already fully expressible, zero new code

`written-world/dev-journal/2026-08-20-sense-machines-as-production-
rules.md` predicted this back in August, before `Effect` generalization
existed: "a sense-machine is a machine whose effects assert `perceives`
instead of `state`, once `Effect` generalizes from a bare value to a full
`(predicate, object)` pair." Phase 2 built exactly that generalization.
Tested the prediction directly rather than assuming it held:

`examples/sense-demo/sentinel.dmml`:

```
machine watchtower/sentinel
  states
    idle
    perceiving

  transition scan(vantage)
    idle -> perceiving
    guard self `postedAt` $vantage
    guard $vantage `adjacent` room/courtyard
    assert self `perceives` room/courtyard
    assert perceiving
```

Fired against a real world (`examples/sense-demo/world.dmml`, sentinel
posted at `tower/north`, `tower/north adjacent room/courtyard`):

```
$ fire-transition examples/sense-demo/sentinel.dmml scan perceives \
    --world examples/sense-demo/world.dmml --param vantage=tower/north

commit perceives
  declare relation perceives
  declare relation state
  watchtower/sentinel `perceives` room/courtyard
  watchtower/sentinel `state` perceiving
```

Passes real `validate-commit` and `check-declared` (merged with the
world's own context). Negative case checked too, not just the happy
path: same machine, a `world-bad-vantage.dmml` where the vantage point
is adjacent to `room/cellar` instead, correctly refuses ("transition's
guards do not currently hold") rather than perceiving something not
actually reachable. No new grammar, no new Haskell module, no new
constructor -- Phase 2's `Effect` generalization was the entire
prerequisite, exactly as predicted a session earlier.

## Caller-driven cascades: built, and a real bug found immediately

Jason's framing: caller drives the loop, cascades (a language-level
mechanism) stay undecided for now. Built the caller-driven version first,
in pure orchestration on top of `fire-transition` -- no new engine
primitive, per instruction.

`examples/cascade-demo/`: two independent machines,
`smithy/furnace`'s `smelt(ore)` (guarded on `self stocked $ore`, asserts
`$ore refinedInto ingot/batch1`) and `smithy/anvil`'s `forge()` (guarded
on `ore/raw1 refinedInto ingot/batch1` -- a fact `smelt` is the only
thing that can produce). `forge` is illegal until `smelt` fires; nothing
wires them together except the shared world snapshot. `examples/
cascade-demo/run.sh` fires both every round, applies whatever succeeds
into a growing world directory, and loops.

**First version was wrong, and instructively so.** It treated "fire-
transition exited 0" as the fire signal and looped `smelt` forever, every
round, forever -- not a script bug in the usual sense, a real interaction
between two already-verified pieces of the system that neither one's own
tests could have caught in isolation:

- `DMML.Materialize`'s facts are collision-free by design (2026-09-02's
  "collision-free mints" redesign) -- a fact is never overwritten, only
  added as a new live alternative. `smithy/furnace`'s original `(self,
  state, idle)` fact from `world.dmml` is NEVER retracted just because
  `(self, state, smelted)` gets asserted alongside it.
- `DMML.Fire` currently refuses to fire an `EffectRetract` at all (the
  real, disclosed Phase 3 gap, tracked as
  [jedelman/dmml#4](https://github.com/jedelman/dmml/issues/4)) -- so
  even a transition author who wrote `retract idle` next to `assert
  smelted` would hit the same wall today.
- `DMML.Guard.evalExists` fans out over EVERY live alternative for a
  (subject, predicate) pair (documented, deliberate -- see `DMML.Guard`'s
  own module doc comment). So the implicit `from -> to` guard, `EXISTS(self
  state idle)`, stays satisfied FOREVER once `idle` is asserted, whether
  or not `smelted` also got asserted later. `mayFire` correctly reports
  "yes, legal" every single round -- it was never wrong, the loop's
  assumption that "legal" meant "new" was.

**The fix needed no new engine code**, and follows directly from a line
already in the sense-machines journal entry back in August: "the existing
no-op-reassertion discipline handles 'nothing changed' for free" --
`addAlternative` dedups on VALUE, so re-asserting an identical fact is
already a no-op at the data level. That means a transition whose fired
OUTPUT is byte-identical to its own last firing produced nothing new,
regardless of what `mayFire` says about legality. Hashing each
transition's last output and stopping once it repeats is the whole fix
(see `run.sh`'s `last_smelt_hash`/`last_forge_hash`). Fixed, reran:

```
=== round 1 ===
smelt fired (new): ... ore/raw1 `refinedInto` ingot/batch1 ...
forge fired (new): ... ingot/batch1 `title` "A forged ingot" ...
=== round 2 ===
smelt: legal, but identical to its own last firing -- nothing new, not counted
forge: legal, but identical to its own last firing -- nothing new, not counted
=== fixpoint: nothing NEW this round, stopping ===
```

One real cascade round: `smelt` produces the fact that makes `forge`
legal, and `forge` fires in the SAME round it becomes eligible, driven
entirely by the caller re-checking after each fire -- exactly the
capability asked for. Both produced commits verified against real
`validate-commit` and `check-declared` (merged with the accumulated
world), not just eyeballed.

## What this means for token efficiency (the other half of Jason's question)

Concretely, not just in the abstract: the cascade demo's whole two-step
chain (mint a refined-ore fact, then mint a forged-ingot fact with a
title) took 2 `fire-transition` calls, each a single deterministic CLI
invocation -- zero LLM round trips once the two machines existed. Before
Phase 2/3, the same outcome needed an agent to author 2 separate commits
by hand (or via `dmml_authoring.build_commit`), each its own reasoning +
tool-call round trip, and to have already worked out for itself that
`ingot/batch1`'s title fact should wait until after the refine fact
existed -- exactly the kind of dependency ordering a guard now enforces
structurally instead of relying on the agent to get right. The real
efficiency claim from the previous conversation holds up under an actual
test, not just an argument: pre-modeled, governed content (a real machine
set an agent doesn't have to re-derive each turn) turns "populate this
part of the world" from N reasoning-and-tool-call round trips into N
cheap, deterministic CLI fires plus however many round trips it takes to
author the machines ONCE.

## What's still open

- The caller-driven dedup fix (content-hash on each transition's own last
  output) is real and it works, but it's a driver-side workaround for a
  gap that's really in `DMML.Fire`: `EffectRetract` firing
  ([jedelman/dmml#4](https://github.com/jedelman/dmml/issues/4)) is what
  would let a `smelt`-shaped transition retract its own `idle` fact for
  real, which is the actually-correct fix for state hygiene under
  repeated firing -- this session's dedup trick sidesteps the need for it
  in THIS demo, it doesn't close the gap.
- `run.sh`'s attempt order is fixed and hand-specified (try `smelt`, then
  try `forge`, every round) -- not a generic scheduler that walks every
  declared machine's every transition looking for anything newly legal.
  A generic version of that scheduler is exactly the kind of thing that
  starts to look like a real cascade PRIMITIVE rather than caller
  orchestration -- which is precisely the design Jason said he's still
  thinking through, so it wasn't built here.
- `run.sh`'s `MAX_ROUNDS` bound is a real safety measure, not a
  termination proof -- nothing here checks a machine set for a genuine
  infinite-refire cycle beyond the specific idle/smelted case this demo
  happened to surface and fix.
