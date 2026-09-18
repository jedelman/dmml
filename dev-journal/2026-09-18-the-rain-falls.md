# 2026-09-18 — The rain falls

Jason: *"fix the rain - make it fall."*

## The gap, restated

`cannon replenish` builds a source: a machine whose `fall` transition is
guarded by nothing and asserts `$unit `in` quarry/north`. It was minted
correctly, wired into the loop correctly, and never fired once.

`driver.py` skipped every parameterized transition of a minted machine,
with an honest reason attached: *binding a param is a real decision, and
this file does not make decisions.* That reason is entirely correct about
one kind of param and entirely wrong about the other, and nothing in the
code could tell them apart, because the grammar does not either.

The cost was invisible everywhere it could have been caught. The machine
parsed, rendered, round-tripped, passed `check-guard-literals`. Worse,
`check-fertility` read the flow graph off the machine text and reported
`in quarry/north` *"sustained by a source"* — true of the architecture,
false of the run. The world still ran down while the analysis said it
would not.

## Two kinds of param

`DMML.Ast.Effect`'s haddock has said from the beginning that the world is
OPEN: an effect may name a node that does not exist, and asserting about
it is what brings it into being. So:

- **A param in a GUARD is a question about what is already there.** Which
  rock. The engine can enumerate the candidates (`GuardAmbiguousBinding`
  already does), and the answer belongs to whoever is choosing. Still
  skipped, still routed to the binding question. Unchanged.
- **A param that appears only as an effect SUBJECT is a name for
  something arriving.** There is nothing to enumerate, so there is no
  decision being taken away from anyone — only a name to supply. This is
  what the loop was refusing to do.
- **A param used nowhere at all** turns out to be common: breeding
  routinely takes one parent's signature over the other's effects, so
  `room/g5` inherits `fall(unit)` with no `$unit` in it. Skipped, but
  reported as its own thing — "needs a binding" was a false statement
  about a param nothing needs.

`classify_params` makes the split. `mint()` registers the middle case.

## Where the name comes from

Not from me. The *kind* is read off the world: two `rock/N `in`
quarry/north` facts say that what falls into that quarry is a rock. When
nothing yet stands in the relation, the fallback is the machine's own word
for it — the formal param name, `unit` — which is likewise not a guess.
The suffix is the transition that brought it, so `rock/fall0` carries its
own provenance.

Freshness is decided by `known_nodes`, not by a counter this file keeps,
because the world is the only thing that can say whether a name is taken.
Re-asserting `rock/fall0 `in` quarry/north` would resurrect the rock
already spent rather than bring another — that is not replenishment, it is
an undo. Names are also allocated against the ones handed out earlier in
the same pass, since several groups fire per round and two sources must
not mint the same node.

## What it does

`examples/jev-driver-demo/quarry-delve/`, same config, same 20-round dry
run, with the classification reverted and restored:

| | before | after |
|---|---|---|
| total firings | **6** | **30** |
| `weather_w0-fall` | never registered | 8 (its per-candidate cap) |
| `weather_w0-gather` | 0 | 8 |
| `room_g6-take` | 0 | 4 |
| the last 14 rounds | *"nothing to decide"*, every one | working |

Before, the world dug its three rocks and then sat there minting
architecture nothing could fire. After, the loop closes, and the commit
chain says so in its own provenance:

```
commit breaches
  room/g6 `took` rock/fall0
  consumes
    fact local:…/002-weather_w0-fall-c0.dmml#fnv1a64:bd84c4eb71cdb367
      rock/fall0 . in = quarry/north
```

A rock that fell, taken, citing the rain that made it.

`check-fertility` on `digger.dmml` alone:

```
sinks (consumed, never produced):     in quarry/north
no cycle and no source -- every substance is a finite stock. This world RUNS DOWN.
```

with the minted rain machine:

```
sources (produced consuming nothing): in quarry/north
no cycle, but in quarry/north is produced from nothing -- sustained by a source.
check-fertility: FERTILE
```

That verdict was already printable this morning. Today it is also true of
what happens.

## The driver gets a test

`minting-params-selftest.py` — the first executed check on the Python side
of this project, written because the failure was a classification mistake
and every existing check is Haskell. Ten assertions plus one against
the cannon's own real output. Reverting `classify_params` turns three of
them red; confirmed, not assumed. Wired into CI.

## Still open

- **Breeding produces vestigial params.** `splice1` and `chimera` can hand
  an offspring a formal param with no use, and `?rock` in effects with no
  guard to bind it. Reported now rather than silently misdescribed, but
  the crossover itself could drop a param its effects never mention.
- **`unit_kind` reads only world files.** A substance that exists solely
  in machine text has no observed kind, so it falls back to the param
  name. Correct, but it means the first rain into a never-yet-filled
  quarry is a `unit/fall0`.
- **Still no live Jev run** of any of this. Dry-run only, both the
  connective operators and now the rain.
- **Bridge proposals still dominate** the connective mix (38 of 39 in the
  last cannon-grow run). Unchanged from this morning.
