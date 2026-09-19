# 2026-09-19 — The missing pairing axis, and measuring predicate diversity

Jason, on bridges being 81 of 112 machines built: *"We don't have any other pairing
predicates, huh. We should (a) make bridges cost something and (b) apply topological
diversity metrics to our predicates as well? Examples would be loves, admires,
inspiredBy, adaptsTo, even Desires."*

Both right, and the diagnosis is sharper than "the mix is lopsided": **bridge won by
volume of offers, not by being wanted.** It was the only operator proposable
N-squared — any two unrelated cleared places — while `feed` and `imply` are gated on far
scarcer structure. Raising `proposals_per_kind` did not rebalance that; it scaled it.

## (b) first, because it turns an instinct into a number

A world is a **multigraph**: every predicate carries its own edge set over the same
nodes, and those sets have completely different shapes. `at` is a forest. `feeds` is a
chain. `admires` can have cycles, reciprocity, triangles. One number for "the world"
hides all of it.

`check-fertility` now reports each predicate's own subgraph, plus **evenness** — Shannon
entropy over the predicate distribution, normalized so 1.00 is every relation equally
used and near 0 is one relation doing everything.

The maximal run, measured:

```
cleared             364 edges 123 nodes acyclic
overlooks             9 edges  16 nodes acyclic
berth                 6 edges   5 nodes acyclic
... 16 more, nearly all singletons ...
evenness across 19 predicate(s): 0.21
```

**0.21.** And *every predicate acyclic* — not one reciprocal relation in a 134-machine
world. The run looked dense and was monotonous, and nothing measured it until now.

## The missing axis

`vista` was the general case hiding in plain sight: it relates two things through the
fixed predicate `overlooks`. Make the predicate a parameter and let the subject be
something other than the machine, and the same operator writes `loves`, `admires`,
`inspiredBy`, `adaptsTo`, `desires` — relations between **agents**, which this corpus
could not previously express at all.

```
cannon regard bond/b0 keeper/orrin admires wright/callum role

  transition comeTo()
    guard keeper/orrin `role` ?sw
    guard wright/callum `role` ?ow
    assert keeper/orrin `admires` wright/callum
  transition part()
    retract keeper/orrin `admires` wright/callum
```

Two decisions worth naming. It asserts about `subj`, not `self` — **a vista *is* the
thing that overlooks; a matchmaker is not the thing that loves.** And it has a `part`
transition, because a relation nothing can undo is a fact of geometry rather than a
relation — and without a way back the machine fires once and is spent, which is the
ratchet `cannon-grow` already measured.

The guard is the honest part. "Does this node exist" is not expressible — guards walk
facts, not existence — so both ends must instead **carry the same kind of property**,
named by a `witness` predicate the driver reads off the world. Two things that both have
a `role` may come to admire each other; a corridor and a role may not. That is what stops
this becoming a *second* N-squared flood.

## (a) Bridges cost

Not a smaller quota — a price. A corridor has to be **cut**, out of something:

```
cannon bridge corridor/c0 room/a room/b is stone/rough

    guard room/a `cleared` mark/yes
    guard ?spent `is` stone/rough
    retract ?spent `is` stone/rough
```

Scarcity as an external relation rather than a knob in the driver — the same move as
depletion-by-guard-failure. `Nothing` keeps the free bridge, because the pure-reachability
demos have no matter to spend and a bridge that can never be dug is worse than a cheap one.

A consequence worth flagging: the cost guard is a `?binder`, so a bridge now also costs a
**decision** — *which* stone it is cut from — and routes through the ambiguous-binding
question. That is arguably correct and it is definitely slower.

## Measured

Same seed, same config, the only changes being a costed bridge and five regard predicates:

| | before | after |
|---|---|---|
| **predicate evenness** | **0.21** | **0.50** |
| predicates in use | 19 | 24 |
| bridges, share of machines built | **72%** (81/112) | **18%** (14/75) |
| operator mix | bridge 81, feed 9, vista 9, imply 8, breed 4 | **regard 25, imply 23, bridge 14**, feed 5, vista 4 |

And the new relations are live: `desires` 9 edges, `loves` 5, `inspiredBy` 4, `admires` 4,
`adaptsTo` 3. `comeTo` and `part` both fire, so bonds form and lapse.

**Honest caveat:** the after-run is 12 rounds against 30, stopped early because rounds got
slow. Absolute counts are therefore not comparable — the *proportions* and the evenness
figure are, and those are what is claimed.

## Still open

- **Every predicate is still acyclic, including the new ones.** `regard` *can* produce
  reciprocity — A admires B and B admires A is a cycle — but in 12 rounds it did not,
  because the reverse pair is a separate proposal competing against everything else.
  Reciprocity may need to be proposed as such rather than waited for.
- **Round cost is driven by world FILES, not machines.** 327 accumulated commit files by
  round 12, each re-parsed per scan. That, not machine count, is what made the run slow,
  and it is the thing to fix before a longer horizon.
- The bridge cost makes every bridge an ambiguous-binding question. Correct, and a real
  new load on the question budget that already binds.
