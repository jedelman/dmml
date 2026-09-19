# 2026-09-19 — The full run

Jason: *"run the whole thing live with shape_every on"* — everything at once, on
`cannon-fanout`, a world that starts as pure reachability and has to bootstrap its own
matter.

## It crashed, and only this configuration could have found it

Round 5, `NameError: cannot access free variable 'anchor'`.

`unbidden_every` reads a single `anchor` variable that only the **pre-fan-out** growth
path ever assigned. Once interest drives growth, several anchors are spent per round and
none of them is called `anchor`, so the name is simply never bound. It fires at the first
round where `round_no % unbidden_every == 0`.

Both switches had existed for hours. **No previous run had ever had both on at once** —
every earlier run turned one feature on and left the rest at their defaults. A
feature-at-a-time run could not have found this; it needed the whole thing.

Fixed by tracking `anchors_used` in both growth paths, which is what the unbidden
allocation actually wants anyway: avoid *everywhere* growth already went this round, not
just one place.

## The run

**8 rounds, 8 Jev calls, 80 firings, 33 machines.** Every round grew something;
4.1 machines per round on average, 5 at peak. 211 interest scores, mean 0.422, range
0.24–0.60 — the same distribution as every previous live run, which is why the ranked
sigmoid rather than any absolute bar.

| built | by |
|---|---|
| breed 20, imply 5, feed 3, vista 3, stamp 1, transmute 1 | chosen 30, **shape allocation 2**, unbidden 1 |

## The bootstrap chain, in order

```
r1  transmute  water/running -> rubble/fallen      the first matter in a world with none
r2  feed       rubble/fallen -> water/running      first circuit closes            rank 1
r2  imply      water/running => sound/wind         first machine anywhere asserting
                                                   ?unit `yields` -- a NEW SHAPE
r3  imply      draft/cold => sound/none            (chosen)
r3  imply      rubble/fallen => sound/none         (ALLOCATED -- spent, not chosen)
r4  imply      light/daylight => sound/echo        (chosen)
r6  imply      light/daylight => sound/none        (ALLOCATED)
r7  feed       sound/echo -> sound/wind            rank-scored, on the NEW axis
r8  feed       sound/wind -> sound/echo            second circuit closes            rank 2
```

Final verdict:

```
CIRCULATES: 2 strongly connected component(s), independent circuits 2
  {yields sound/echo, yields sound/wind}   rank 1
  {has rubble/fallen, has water/running}   rank 1
This world SUSTAINS, and sustains ITSELF.
```

A world that began with four unbuilt ways and a sleeping keeper — **no matter, no flow
layer, `check-fertility` structurally silent on the whole rhizome half** — ended with two
independent circuits on two different predicates, one of which did not exist as a
substance axis until a new transition shape made it readable.

Every layer built over two days is load-bearing in that chain: the substance generator
opens `has`, the rank measure closes it, the shape generator opens `yields`, breeding
propagates the atom, and the rank measure closes that too.

## What the allocation actually bought, stated honestly

**Both allocated implies targeted `sound/none`, which ended the run as a sink.** The two
implies that fed the second circuit — `sound/wind` in r2 and `sound/echo` in r4 — were
both **chosen by interest**, not allocated.

So in this run: the allocation guaranteed *that* shapes kept opening; interest was
better at *which*. That is the division of labour the design intends (interest breaks the
tie, the allocation decides whether to spend at all) — but it means the tie-break is
doing more work than I credited when I built it, and the allocation's value here is purely
as a floor. On this evidence it bought insurance, not the circuit.

Worth saying plainly because the tempting write-up is "the allocation built the second
axis," and that is not what happened.

## Also visible, not fixed

- **`sound/none` is a bad target and got picked three times of five.** A transformation
  *into silence* has nothing downstream by construction. The allocation picks the
  highest-interest opener, and twice that was a `sound/none` one. Nothing in the proposer
  knows that some objects are terminal by meaning rather than by graph position.
- **9 of 33 machines never fired**, mostly late breeds that ran out of budget rather than
  being unfireable. The run stopped on `max_total_firings`, not a fixpoint.
- **The growth allowance bit twice** — "4 relation(s) wanted but this round's growth
  allowance (4) is already spent." Frontier growth is taken first, so relations lose ties
  to edges systematically. Not obviously wrong, but it is a priority nobody chose.
