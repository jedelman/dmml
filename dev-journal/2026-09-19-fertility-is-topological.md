# 2026-09-19 — Fertility is topological (with one correction), and interest is ranked

Jason: *"Ohhhhh. Fertility is a topological property isn't it. Also interest should be
ranked, I think, not an absolute threshold - maybe a sigmoid process."*

Both right. The first needed one correction that turns out to carry the whole
distinction this project had already been drawing without naming it.

## The correction: not the space, the digraph

The obvious reading is that "sustains" means the flow graph has a loop — first Betti
number > 0. That reading is **wrong**, and the counterexample is built and committed
(`examples/rhizome-demo/diamond.dmml`):

```
is clay/raw  ->  is brick/fired      V = 4, E = 4, one component
is clay/raw  ->  is tile/flat        b₁ = E - V + C = 1
is brick/fired  ->  is wall/section
is tile/flat  ->  is wall/section
```

A genuine non-contractible circle in the underlying space — and the world runs down,
because matter only ever goes one way round it. `check-fertility` on that file:

```
every strongly connected component is trivial and there is no source: the flow
graph is a partial order and every substance is a finite stock. This world RUNS DOWN.
```

**Direction is not topological data, and direction is exactly what decides it.** Which
is the same point `flowOf` already rested on: a break in a flow — a *coupure* — has an
input side and an output side, and they are not interchangeable.

## So the invariant is a non-trivial SCC

Topological, on the right object. Within a strongly connected component every substance
reaches every other and comes back, and the **cycle rank `E − V + 1`** of that component
counts its *independent* circuits — how many edges you would have to cut before matter
stops circulating. Rank 1 is a single loop one broken machine ends. Rank 3 is a world
with somewhere else for matter to go. Fertility gets a magnitude, not just a yes/no.

`check-fertility` now reports it. Two runs on the same demo, one `feed` apart:

```
with decay.dmml:     CIRCULATES: 1 strongly connected component(s), independent circuits 1
                       {is brick/fired, is clay/raw, is wall/section}  rank 1
                     This world SUSTAINS, and sustains ITSELF.

without decay.dmml:  no circuit, but is clay/raw is produced from nothing
                     -- sustained by a SOURCE.
```

## And the third verdict is not topological at all

Saying so sharpens it. **A source sustains a world with no circuit whatsoever.** Rain is
not a loop, it is an *opening* — the flow graph coupled to something outside itself.
Topologically a source is just a leaf. What makes it sustain is that the system is
**open**, which is a boundary condition, not a shape. It stops the moment the outside
does.

That distinction was already in the tool's output ("CYCLE… SUSTAINS" vs "sustained by a
source") and nobody had said *why* they were different in kind. They are: one is
topology, one is a boundary.

Read back onto the four connective operators, they turn out to be exactly the moves
available on a digraph:

| operator | move |
|---|---|
| `feed` | closes a circuit — raises the cycle rank |
| `bridge` | the same, in the reachability graph |
| `replenish` | opens the boundary instead — raises nothing |
| `vista` | an edge carrying no flow — a circuit in the reference graph, none in this one |

That was not designed. It fell out.

## Interest, ranked

The absolute threshold was wrong and the floor+band that replaced it was still a cliff.
The measurement that settles it: **122 live scores, mean 0.412, sd 0.086, range
0.21–0.59.** Every absolute cut this file tried was measuring the *calibration of the
scale*, not the content of the answer.

What the signal is good for is **order**. So: standardize each score against the run's
own running mean and sd, push it through a sigmoid, take that as the probability the
option gets built. Scale-free by construction — it never reads the raw number, so it
cannot be broken by Jev living in 0.2–0.6 rather than 0–1. The draw comes from the
Perlin field, so it is probabilistic *and* exactly reproducible.

Crucially this keeps both capabilities a `choice` lacks: a round entirely below the
run's baseline builds nothing, a round entirely above builds everything.

A prior (mean 0.412, sd 0.086, weight 12 — the measured figures, stated as what they
are) covers the first rounds and washes out. `refuse_below: 0.15` stays as an explicit
backstop, not a knob: set beneath the entire observed range, so on any run resembling
the measured ones it never fires, which is correct behaviour for a guard.

## What it does live

```
r1: 3 of 4 taken -- way/west 0.55 p=0.81, way/east 0.53 p=0.78, way/north 0.27 p=0.19
                  | left: way/south 0.24
r2: 1 of 2 taken -- watch/keeper 0.45 p=0.65 | left: way/south 0.30
r3: 3 of 5 taken -- tower/t0 0.53 p=0.81, room/g0 0.52 p=0.79, room/g2 0.51 p=0.77
                  | left: room/g1 0.51, way/south 0.28
r4: 4 of 5 taken -- tower/t4 0.58 p=0.86, ..., way/south 0.29 p=0.21 | left: room/g1 0.24
```

Two things worth noticing there.

**`way/south` — the passage full of fallen rubble — was passed over three rounds
running and then explored anyway**, at p=0.21. That is the unbidden arrival the loop
previously needed a separate `unbidden_every` allocation to buy. It now falls out of
the selection rule itself: a world where the unwanted way sometimes gets walked is what
a sigmoid *is*. The explicit allocation may now be redundant; not removed, but flagged.

**`room/g1` at 0.51 was left while `room/g2` at 0.51 was taken.** Identical scores,
different draws. Under a cliff those two are the same option; under a sigmoid they are
two coin flips with the same bias, which is the honest representation of a chooser that
mildly wants both.

## Still open

- `unbidden_every` may now be doing a job the sigmoid does better. Worth testing before
  removing.
- **Cycle rank is reported but nothing uses it.** The obvious next move is the
  connective proposer preferring the `feed` that raises rank over one that doesn't —
  which would make growth pursue *robustness* rather than mere connection.
- The breeding policy is still a 2-cycle (2026-09-18, untouched).
- `temperature` is set to 1.0 and never swept. Unlike the lane constant, it has not been
  measured against anything.
