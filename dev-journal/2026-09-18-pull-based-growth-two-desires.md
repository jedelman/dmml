# 2026-09-18 — Growth pulled, not pushed; Jev picks where; two desires

Third pass over the same loop, from a conversation rather than a spec. Jason:
*"What if we let Jev pick the next node to expand from? Decisions is what he does."*
Then: *"Build it all."*

## What was wrong with what shipped this morning

The self-extending loop fired the cannon once per round off a clock. That is
backwards, and the reason matters more than the fix.

**written-world** — this project's sibling — generates a room when a player
*arrives at an unmapped frontier point*, and once made it is never remade. That
was never a lazy-evaluation optimization. It is what a world made of attention
looks like when you implement it: attention is the only genuinely scarce
resource in this whole stack (tokens, interest, the reader), and building where
nobody went spends it on nothing.

So this morning's cannon was **an eager cannon on a lazy ontology**. `room/g3` in
the first live run came out a bare hall, minted, described and offered before
anything needed somewhere to go.

## Growth is now pulled

The demand signal is an **unmapped edge**: a node the world records as `cleared`
that nothing is built on yet (`unmapped_frontier` — frontier nodes minus every
node any machine currently guards on). Such an edge is not exhaustion, it is a
request.

The stopping condition inverted accordingly. The old loop stopped the moment
nothing was legal; that treats the frontier running dry as the end of the run
when it is precisely the signal to build. **The run now ends only when there is
nothing legal AND no opened way left unentered.**

This is written-world's principle one articulation down: that generates a room's
DESCRIPTION on arrival, this generates a room's MECHANISM.

## Jev picks where

When several edges stand open, "where do you press on?" rides in the **same
batched call** as that round's action choices — one more question, no extra call.

The anchor used to be `frontier[seq % len(frontier)]`. That modulo was the single
most consequential knob in the file, because it decided the world's shape, and it
was mine. It belongs to whoever makes decisions.

**Phrased in-fiction, deliberately.** Asking "which node should the generator
expand" invites an answer from a level designer, with taste. Asking a delver where
they are going gets an answer from desire. A world built where a character wants
to go is producing in response to wanting; a world built where a reader's eye
lingers is a slot machine. The frame is what holds those apart — and it is the
answer to a worry raised earlier in the same conversation, that attention-as-
resource risks a world that courts attention rather than spending it.

## Two desires

Pull-only growth has a real cost: nothing ever arrives unbidden, nothing in the
dungeon appears to want anything, and the world is summoned rather than inhabited.

`unbidden_every: N` buys that back — every Nth round the cannon also builds
somewhere the delve did *not* choose. It is an explicit line item because it is
not free: it spends the scarce resource on what nobody asked for, which is exactly
what makes a world have weather. Logged as `bidden: true/false` so the difference
between a world that answers you and one that also wants things can be read back
out of an audit log.

## The leak this found in its own output

Dry-running the new loop showed it alternating: fire-rounds (legal non-empty,
no unmapped edge) and build-rounds (nothing legal, an edge open). A build-only
round was still **calling Jev with zero questions in it** — burning the one
genuinely scarce resource to ask nothing. Exactly the failure mode the whole
redesign is about, committed by the redesign.

Fixed: when there is no legal action and at most one opened way, there is no
decision — build and move on. 7 of 9 rounds needed a call instead of 9.

## Live run

New minimal seed (`examples/jev-driver-demo/cannon-grow/`): a world containing
`room/entry cleared` and one fork, nothing else. Beyond the mouth, nothing exists.
The dungeon is built entirely by the cannon, as the delve presses into it.

**9 rounds, 8 firings, 7 machines minted, run ended at a true fixpoint** (nothing
legal, no unmapped edge) rather than on a cap. All minted machines pass
`check-guard-literals`.

The frontier question fired once, over `['room/g1', 'room/g2']`, and produced a
genuine split: **0.75 / 0.25 at 0.49 confidence**. A real, uncertain decision about
where the world should take shape — not a formality. The other four growth events
had exactly one open edge, so no question was asked and no call was spent.

**And the fork hole became productive.** Round 1 went LEFT, sealing `path/right`
forever. Round 8 fired `room_g4-goRight` — a bred transition carrying the fork's
`assert path/right cleared` without the fork's lock — and the delve then **pressed
into `path/right` and the cannon built there.** The recombination-defeats-
exclusivity finding from this morning is no longer just a curiosity in a haddock:
it is a generator of territory. The dungeon routes around its own foreclosure and
then grows into the space it reopened.

## Still open

- **Fitness: still none, and increasingly it looks like there shouldn't be one.**
  The economy is the token budget and the reader's attention; boring architecture
  is literally expensive, and selection is what gets pressed into again. A fitness
  function would be transcendent. Scarcity is immanent.
- **The budget is still denominated in counts** (`max_rounds`,
  `max_minted_machines`) when the real currency is calls and reading. Known
  mis-denomination, not a claim that counts are right.
- **Parent and mode selection are still deterministic cycles** — two knobs that
  are still mine. Anchor was the one worth handing over first; these could follow.
- **`guarded_nodes` decides "built on" by guard anchors only.** A machine that
  relates to a place without guarding on it is invisible to it, so such a place
  reads as unmapped and can be built on twice.
- **Substances** — the conversation that produced this went rocks → footpads →
  "every object implies a substance" → double articulation → tokens. None of the
  substance layer is built. `retro-imply` is the machinery for reading an object
  back to what it implies, and it can only add, never retract, which means
  retroaction *manufactures* resource. Finitude has to come from somewhere else,
  and the answer arrived at was: it already does, from the token budget.
