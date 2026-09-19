# 2026-09-19 — Readiness for a maximal run: three measurements that changed the plan

Jason: *"our next step for the cannon is graph density and differentiation over a longer
horizon… hand them a BROAD range of machines (maybe 40-50 across a variety of domains)
and a lot of raw substance, then do a maximal run."*

The raw material is already here — **81 machines in `examples/`, 78 distinct shapes**,
more than the 40–50 proposed. But three measurements said the plan as stated would not
have produced a longer horizon, and each one pointed at a different fix.

## 1. Pool SIZE buys nothing. Pool SELECTION buys 5×.

`check-fertility`'s structural walk halts at the first repeated bred shape, so it is a
direct measure of horizon.

```
pool  2 -> walk  2      pool 16 -> walk 11
pool  4 -> walk 14      pool 32 -> walk 11
pool  8 -> walk 11      pool 81 -> walk 11
```

**81 machines with 78 distinct shapes give exactly the horizon of 4.** It saturates
immediately, because the breeding policy makes parent A the newest offspring — a single
chain that converges regardless of how many B's are available to cross with.

But different 8-machine *slices* of that same 81 give walks from **2 to 26**, and a
searched pool reaches **83**. Horizon is a property of *which* shapes are present, not
how many. And the search is free: `check-fertility` answers in milliseconds and needs no
Jev call, so a pool can be chosen before a single token is spent on it.

`examples/jev-driver-demo/select-pool.py` does that search, scoring three things that are
not the same question — walk (horizon), fertile (can the offspring still be built on),
rank (does matter circulate).

## 2. Diversity and anchorability were in direct tension. They are not any more.

The reason every high-horizon pool came back sterile:

```
22 of 81 machines assert self `cleared`  (27%)
-> 59 cannot be anchored on under the cannon convention
```

Any pool diverse enough to lengthen the walk pulls in the other 73%, and crossing one
sterilises the lineage from that generation on — the "growth without reachability"
failure this repo has already measured once as 60 rooms and 2 firings.

Caught in the act at **`g1 chimera x crew/digger -> STERILE`**. Chimera takes A's guards
with B's *world* effects, and `assert self \`cleared\`` is one of A's world effects, so it
was dropped. Splice loses it the same way by taking a tail of B that happens to contain no
clearing transition.

`preserveAnchor` gives it back — a machine-level repair, like `pruneOrphans`, covering all
three modes at once. **An offspring of two unclearing parents stays unclearable**, which is
correct: it is only ever given back something a parent already had.

Same 400-pool sample, same seed, before and after:

| | before | after |
|---|---|---|
| FERTILE pools | 16 / 400 (**4%**) | 194 / 400 (**48%**) |
| best FERTILE walk | 8 | **83** |
| FERTILE *and* circulating | 0 | **6** |
| median walk, all pools | 17 | 21 |

Twelvefold on the thing that was blocking, and the tension is simply gone.

## 3. Density was capped by me, not by Jev.

```
30 cleared nodes ->   435 possible bridges, 2 offered per round
100 cleared nodes -> 4,950 possible bridges, 2 offered per round
```

`unrelated[:2]`, `feeds[:2]`, `couplings[:2]`. N-squared opportunity, constant-sized menu.
And growth spent its allowance on frontier edges *before* relations, so relations were
starved systematically — the log had been saying so for two days ("4 relation(s) wanted but
this round's growth allowance is already spent").

Both are now policy: `proposals_per_kind` and `relations_first`. Measured on the same seed:

| | relations offered | relations built | machines |
|---|---|---|---|
| default (2/kind, edges first) | 59 | 17 | 36 |
| dense (6/kind, relations first) | **149** | **41** | 42 |

**2.4× the relations built**, from two knobs that were previously constants.

## A bug in my own tool, caught by its output being implausible

`select-pool.py` first reported **"0 of 300 fertile"** against a measured 48%. The probe
read a missing `generation(s)` line as *a walk of zero* rather than *the probe did not
run* — the paths were relative to `dmml-hs/` and I ran it from a subdirectory.

Worth recording because of what made it visible: **0% was implausible.** A silent failure
that produced a plausible number would have been believed. The probe now distinguishes
"no verdict" from "a verdict of zero", and warns when probes fail.

## Where that leaves the maximal run

Ready, with the pool chosen rather than collected. Two things remain unmeasured and I will
not guess at them:

- **Per-round cost at 80+ machines.** `scan-candidates` was measured at small scale only
  (0.53s/round). More machines means more candidates means a bigger JSON per round.
- **`max_questions: 40`.** It has never once bitten — the largest live round asked 21 —
  and at this scale it certainly will. The truncation path is reported but untested.

## Also, on names

Every name in the stack is a counter: `room/g{seq}`, `corridor/c{seq}_{i}`,
`works/i{seq}_{i}`. Exactly one site consults the world — `unit_kind`, which reads the
namespace off existing facts so that what falls into a quarry full of `rock/N` is called a
rock. So `rock/fall0` parses as "a rock, that arrived by falling, the first one":
structurally honest and linguistically nothing. A phoneme-combinator naming layer would sit
on top of this without changing what is true. Jason: not a priority.
