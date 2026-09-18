# rhizome-demo — connective operators, and flow that closes on itself

Everything in `app/Cannon.hs`'s original set is arborescent, and it is
visible in the signature:

```
cannon <hall|forge|vault|spur> <newNode> <parentNode>
```

One parent, one child. The word *parent* is in the API. A world built
from those is a tree: it has leaves, and "can it keep growing" reduces
to "do leaves keep appearing," which is why one sterile crossover ends a
lineage.

**A corridor has two ends.** It is not a child of a room; it is an edge
between two rooms that already exist, and nothing in the original set can
express one. These four can:

```sh
CANNON=cannon   # or: cabal list-bin cannon

$CANNON bridge    corridor/ab  room/a      room/b        # two ends
$CANNON feed      mill/kiln    clay/raw    brick/fired   # a transformer
$CANNON replenish weather/rain clay/raw    cliff/face    # rain: produces, consumes nothing
$CANNON vista     tower/watch  room/b      quarry/north  # relates without flowing
```

## The chain, fired for real

```sh
FIRE=fire-transition
$FIRE rain.dmml   fall      rains  --world world.dmml --param unit=clay/lump1 > f1.dmml
$FIRE kiln.dmml   transform fires  --world world.dmml --world f1.dmml         > f2.dmml
$FIRE mason.dmml  transform builds --world world.dmml --world f1.dmml --world f2.dmml
```

`clay/lump1` is produced by rain, becomes `brick/fired` in the kiln,
becomes `wall/section` at the mason — **the same unit**, transformed in
place, each step consuming the last with a real `consumes` citation. The
`?binder` guard variable is what makes that possible: the guard finds a
unit and the effects spend *that* unit.

The transform asserts under the **same predicate** it retracts (`is`),
deliberately. An earlier draft dodged that with a separate `becomes`
predicate and thereby broke the only thing that matters: a second mill
must be able to consume the first's output. A retract lowers into the
commit's `consumes` block and an assert into its facts, so there is no
duplicate-key collision — checked, not assumed.

## The cycle a tree cannot have

Build `room/a -> room/b -> room/c` with halls, then `bridge corridor/ac
room/a room/c`. Now `room/c` has **two independent ways in**: the tree
path through `room/b`, and the corridor directly. That is a cycle in
reachability, and it is precisely what an arborescent generator cannot
produce, because everything it makes descends from a single parent.

## Flow that closes

`check-fertility` reads the substance graph these induce, and the verdict
turns on one structural fact — **a DAG runs down, a cycle sustains**:

```
$ check-fertility kiln.dmml mason.dmml
  is clay/raw -> is brick/fired -> is wall/section
  no cycle and no source -- every substance is a finite stock. This world RUNS DOWN.

$ check-fertility kiln.dmml mason.dmml rain.dmml
  sources: is clay/raw
  no cycle, but is clay/raw is produced from nothing -- sustained by a source.

$ check-fertility kiln.dmml mason.dmml decay.dmml     # walls crumble back to clay
  is clay/raw -> is brick/fired -> is wall/section -> is clay/raw
  CYCLE -- this world SUSTAINS.
```

"Quarries that fill with rain" is not a metaphor for sustainability. It
is literally a cycle in that graph, and cycle-detection on a finite graph
is trivial — so the rhizomatic question is exactly as decidable as the
arborescent one. It is simply a question about something else.
