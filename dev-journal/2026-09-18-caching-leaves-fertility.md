# 2026-09-18 — Cache the snapshot, any leaf is valid, and fertility is decidable

Three from one message. Two worked, one worked and then revealed something worse,
and the third turned out to be a real result rather than an estimate.

## 1. Cache the materialized snapshot — 141× at scale

`fire-transition` re-parses every `--world` file on every call, and the driver
called it once per candidate per round. So a round cost (candidates × world-files)
parses of files that had not changed. The engine was never the problem — it is
linear at ~0.12ms/fact.

New `scan-candidates` binary: materialize once, reuse for every candidate.

| facts | cands | per-call | batch | speedup |
|------:|------:|---------:|------:|--------:|
| 200 | 20 | 0.68s | 0.04s | 16× |
| 800 | 50 | 5.26s | 0.11s | 46× |
| 1600 | 100 | 19.19s | 0.24s | 81× |
| 3200 | 200 | 74.67s | 0.53s | 141× |

A 200-candidate round over 3200 facts: **75 seconds → 0.53.** Round cost is now
linear in world size rather than quadratic.

Deliberately a separate binary, not a mode on `fire-transition`: that CLI's
contract is "fire one transition, print the commit," which is what a person runs
and what `cascade-demo/run.sh` depends on. Verified byte-identical output against
it, and the driver keeps the per-candidate path as a fallback so the batch path can
be *checked* rather than trusted.

## 2. "Any leaf is valid" — the correction, and what it exposed

I had framed run length as depending on the chooser picking transitions that open
new ground. That was wrong, and Jason named it: *"The chooser shouldn't have to be
playing the game."* Jev's job is to want things, not to feed the cannon.

The cause was that growth was keyed to `cleared` nodes — the **delve's** frontier.
That made the world able to take shape only where the delve had already walked.
Separated the two: `growable_leaves` (where the WORLD can grow) from
`unmapped_frontier` (where the DELVE can press on, which is still what Jev is
asked about).

**And that made things worse in an instructive way.** The long run went from 2
machines to 60 — and fired **twice**. It had built a 60-room corridor into nothing:

```
machine room/g40
  transition breach()
    guard room/g38 `cleared` mark/yes
    assert path/left `cleared` mark/yes    <- clears a PATH
    ...                                    <- never `assert self `cleared``
```

Every room guarded on the previous being cleared; none of them ever cleared
itself. Unreachable the moment the second was minted. Growth unbounded, fertility
zero — the eager-cannon failure in a new costume.

Fixed with `anchorable_nodes`: a leaf is only worth building on if something can
ever assert `cleared` on it. That is the LOOP doing its own job, not the chooser
doing it — whether a place can ever be reached is a structural fact about the
machines, and making Jev responsible for it was the original mistake.

## 3. Fertility IS analytically decidable — and here is why

Jason: *"I wonder if there's a way to analytically predict fertility?"* Yes, and
exactly rather than statistically.

**Crossover never invents a transition.** `Union` takes the parents' transitions,
`Splice` a prefix and a suffix, `Chimera` pairs one parent's guard list with the
other's effect list. So every transition reachable from a seed is drawn from
`GuardLists(seed) × EffectLists(seed)` — finite. Quotient by node renaming (the
only thing `breed` introduces that is not already in a parent) and the reachable
**structure** space is finite. The driver's policy is deterministic besides, so the
walk is eventually periodic: simulate until a shape repeats and you have the whole
answer. No branching-process approximation, no sampling.

Fertility itself is a property of the **attachment convention**, not the grammar:
the cannon anchors with `guard <parent> `cleared` mark/yes`, and the only thing
that ever asserts that about a room is the room itself. A machine with no
`assert self `cleared`` is a dead end.

`check-fertility` implements it, and predicted both observed runs before either was
re-run:

```
$ check-fertility fork-mouth.dmml
  fork/mouth   [STERILE -- never clears itself]
  g0  union x fork/mouth   [STERILE]
  STERILE at generation g0
```

...which is exactly the 60-unreachable-rooms run. And on a richer seed:

```
  g0 union x fork/main      [fertile]
  g1 chimera x room/wForge  [fertile]
  g2 splice1 x room/wVault  [fertile]
  g3 union x room/eHall     [fertile]
  g4 chimera x fork/main    [STERILE]   <- the fork's effects, no self-clear
  g5 splice1 x room/wForge  [STERILE]
  g6 union x room/wVault    [fertile]   <- recovers
```

The sterile generations are always a **chimera with a fork**. A fork clears a path
rather than itself, so an offspring taking its effects inherits "opens a way"
without "is itself cleared." One such crossover ends a lineage, and this says which
one and when, in milliseconds, without spending a call.

## What this actually tells us about run length

Both demo seeds are small, in different ways, and that is the real finding:

- **`cannon-grow`** (one fork, nothing else): the lineage is sterile at g0. No
  amount of loop cleverness fixes it — the seed cannot sustain growth under this
  attachment convention.
- **`cannon-dungeon`** (a closed 5-machine map): almost no unbuilt leaves. It is a
  *finished* dungeon; there is nowhere to attach.

So we still have not produced a long run, and I want to be straight that the
performance work does not by itself buy one. What changed is that the limit is now
**legible in advance**: `check-fertility` says whether a seed can sustain growth
before a single token is spent, and `scan-candidates` means that when a fertile
seed does exist, size stops being the constraint.

## Still open

- **A genuinely fertile seed has not been written.** From the analysis, one wants
  at least one self-clearing room whose effects survive chimera — i.e. the fertile
  property has to be robust under taking the *other* parent's effects, which for
  the current cannon means several self-clearing parents so the cycle keeps
  landing on one.
- **`check-fertility` hard-codes the driver's breeding policy.** A real coupling,
  stated in its haddock: if `plan_extension` changes, the prediction is only as
  good as its agreement with it.
- **Fertility is measured against the `cleared` convention**, not derived. A
  dungeon expressing reachability another way needs a different predicate here.
