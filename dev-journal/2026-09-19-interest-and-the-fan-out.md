# 2026-09-19 — Interest, the fan-out, and Jev for real

Jason: *"Let's get Jev on it. And - let's add an 'interest' score to each choice each
pass. Are we getting Jev's massive parallelism - send the whole frontier, grow all
valid and interesting in one step?"*

Short answer to the question: **no, we weren't** — and the reason is worth more than
the fix.

## The serialization was in the primitive, not in the budget

Actions were already parallel: one winner per conflict-free group, several groups a
round. **Growth was strictly serial** — one frontier edge, one relation, one machine
per round, however many stood open.

That was not a scarcity decision. It was a consequence of asking a `choice`.

> A `choice` is a softmax over alternatives. Its probabilities sum to 1, so it
> measures *relative* preference and structurally cannot say "all of these are worth
> building" or "none of these are." Asking "which one edge?" forces exactly one
> answer even when six are interesting and even when none are.

Jev's own docs say *"all questions are evaluated in parallel, so adding more
questions to a call typically doesn't add any latency."* The parallelism was there
the whole time. The loop was spending it on actions and not on growth.

`noul` is the primitive that fits: an independent 0–1 per option, no competition
between them. Six edges can all come back 0.8. Six can all come back 0.1.

## Three live runs, and the first two built nothing

**Run 1.** Every score came back 0.43–0.50. Flat.

**Run 2**, after two fixes — the run's persona wasn't being passed to interest
questions (only `choice` got it), and every unbuilt way was described identically —
still flat, 0.44–0.50.

So: stop guessing, probe the primitive.

```
"You are extremely thirsty. There is a well to the east... to the north is solid rock."
  obvious_yes  (go east to the well)   0.95
  obvious_no   (go north into rock)    0.02
  neutral      (go west)               0.15
```

It discriminates *sharply*. It had never been given anything to discriminate on.

**The real bug was `state`.** The summary sent with every question was bookkeeping
about the RUN — *"1 committed fact files, 0 prior firings, 7 known nodes"* — not a
description of the world. That was survivable while every question was a `choice`,
because a choice carries its alternatives in its own criteria and can be answered by
comparing them. **An independent yes/no cannot.** It needs a situation, and there
wasn't one.

Rewritten to say what is true — what has been done, what stands open, what the world
says about it, all read off committed facts:

```
Round 1. You have done nothing yet. 4 way(s) stand open with nothing built beyond
them: way/north (has draft/cold; yields sound/none), way/east (has water/running;
yields sound/echo), way/south (has rubble/fallen; yields sound/none), way/west
(has light/daylight; yields sound/wind). 0 action(s) are legal right now.
```

Next probe: **0.48 / 0.47 / 0.27 / 0.25.** Running water and daylight over a cold
draft and fallen rubble. A real preference, read off real facts.

## The threshold was the wrong instrument

The top score was 0.48 and the bar was 0.6, so the world still wouldn't grow.

Across **122 interest scores in two live runs the range was 0.21–0.59, mean 0.41.
Jev never once exceeded 0.6.** A fixed 0.6 threshold would have built nothing, ever,
in either run — while the ranking underneath it was perfectly clear.

An absolute threshold assumes the 0–1 scale means the same thing to the chooser as
to this file. It doesn't. Nobody is desperate to see an unbuilt corridor.

So, two numbers:

- **`floor` (0.35), absolute** — "is anything here worth it at all?" This is what
  preserves the capability a `choice` never had: the ability to build *nothing*.
- **`band` (0.15), relative** — "which of them?", by reach from the best. Ranking is
  what the signal is good for; an absolute reading of it is what it is not.

## Run 3: the fan-out, live

`examples/jev-driver-demo/cannon-fanout/` — a great hall with four ways out, each
carrying real distinguishing facts.

Round 1, real Jev:

```
interest: 2 of 4 open way(s) within 0.15 of the best -- way/west 0.55, way/east 0.51
          (passed over: way/north 0.27, way/south 0.25)
```

Daylight and running water taken; cold draft and fallen rubble left standing open.
By round 3 all four remaining edges were within band and **four machines were built
from one call**.

| | cannon-fanout (live) | quarry-delve (live) |
|---|---|---|
| rounds / Jev calls | 4 / 4 | 20 / 20 |
| firings | 20 | 38 |
| machines minted | **16** | 12 |
| machines per growing round | **4.0 mean, 4 max** | 1.0 |
| rounds that grew nothing | 0 | **8** |

Those 8 rounds are the other half of the feature, and quarry-delve shows it live:

```
interest: nothing reaches the floor 0.35 (best weather/w0 at 0.26)
          -- the world does not grow this round
```

The world declined to grow. No `choice` over that same option could have said so.

quarry-delve also ran the rest of the stack end to end under real Jev: the rain fell
eight times, and Jev bound `room_g7-take`'s `?rock` to `rock/fall5` — a rock that
existed because it had rained earlier in the same run.

## Still selection by attention, not fitness

Nothing here scores a machine's *structure*. It asks whether a reader wants to go and
look. The scarcity stays immanent — the token budget, the reader — which is the line
this project has held since fitness was first deferred. Interest is not a fitness
function wearing a hat.

## Caught in passing

- The log lied once: *"no relation reaches the floor 0.35 (best at 0.48)"* when 0.48
  clears 0.35 easily and the real reason was that the round's growth allowance was
  already spent. Two different causes printed as one. Fixed.
- The dry-run interest stand-in was calibrated wrong at first — the naive
  `(drift + 1) / 2` maps a zero-mean field with sd 0.306 into a narrow band around
  0.5, so nothing cleared any bar and dry runs rehearsed a world that never grew.
  Now pushed through its own measured CDF to a near-uniform [0, 1]. It is a stand-in
  for the *shape* of an interest distribution, explicitly not a prediction of Jev.
- A `--world` file holds exactly one commit. Three `commit` blocks in one seed file
  is a parse error at the second.

## Still open

- **`max_questions` (40) has never bitten.** The biggest live round asked 21. On a
  100-candidate world it would, and the truncation is reported but untested.
- **`score_actions` is observability, not control** — the group's `choice` still
  decides what happens. Worth asking whether interest should break ties, or whether
  that would double-count the same signal.
- **The breeding policy is still a 2-cycle** (yesterday's finding, untouched).
- `cannon-fanout` is a seed I wrote to exercise the fan-out. `cannon-grow`'s frontier
  is one edge wide by construction, so it never showed the behaviour at all.
