# 2026-09-18 — Connective operators in the loop, and the philosophy checked

## The wiring

`driver.py` now asks a second growth question alongside the first. The arborescent
one is *"where do I attach"* — a question about leaves, with as many answers as
there are leaves. The rhizomatic one is **"which two things that already exist
should now relate"** — a question with N² answers, which is why the world
densifies instead of exhausting, and equally why choosing matters more here.

Four proposal kinds, each generated from real structure rather than taste:

- **bridge** — two cleared places not already related by any machine guarding on
  both. The one move that puts a **cycle in reachability**, which no amount of
  stamping or breeding can produce.
- **feed** — from a *terminal* substance (produced, consumed by nothing) back to a
  *root* (consumed, produced by nothing). That single edge turns the flow DAG into
  a cycle. Not a heuristic about what would be nice — it is the one edge that
  changes the graph's verdict.
- **replenish** — a source for any root substance.
- **vista** — relates two places and moves nothing between them.

Asked in-fiction, same as the frontier question and for the same reason: *"which
relation should the generator add"* gets an answer from a level designer, *"what
should be true of this place"* gets one from someone who lives there.

## What it does

**`cannon-grow`, 40 rounds: "stopped: hit max_rounds without a fixpoint."** First
time in this whole line of work that the loop ran out of *budget* rather than
architecture. 49 machines, 49 firings, and **40 of 49 nodes ended with more than
one way in** — i.e. the reachability graph is thoroughly cyclic, which is the
thing a tree structurally cannot be.

Bred rooms also started inheriting `crossFromA`/`crossFromB`: connective machines
enter the breeding pool, so the rhizome feeds back into the arborescent half.

**`quarry-delve`:** the loop read `in quarry/north` off the machines as a root
substance — consumed by the digger, produced by nothing — and **built a rain
machine to produce it.** The loop closed its own flow loop, from the flow graph
alone.

## Two errors of mine, both caught by running it

1. **`replenish` guarded on the dungeon's `cleared` convention.** In a pure
   substance world there are no `cleared` facts anywhere, so rain could never
   fall — exactly backwards, since a source is by definition unconditioned. Now it
   guards on nothing, which is the whole content of the word "source," and is also
   what makes it the operator that can turn a DAG into a cycle.
2. **`feed`/`replenish` hard-coded the `is` predicate.** Generalized, so they work
   on `in quarry/north` as readily as `is clay/raw`.

## Known gap, hit immediately

`replenish` emits `fall($unit)` — a parameterized transition — and the driver
skips those as candidates, because binding a param is a real decision it will not
guess. **So the rain exists and never falls.** The disclosed limit bit for real the
first time it mattered. The fix is the same shape as the ambiguous-binder question:
the loop should ask *"what arrives?"* rather than skipping. Not built.

---

## The philosophy, checked against sources rather than memory

Jason said it's right; I checked anyway, because specific attributions are exactly
the high-hallucination-risk class this repo's own fact-checking protocol names.

**Verified:**

- The rhizome's principles are *connection* and *heterogeneity* ("any point of a
  rhizome can be connected to anything other, and must be"), *multiplicity*,
  *asignifying rupture*, and *cartography/decalcomania*. Arborescent/root-tree is
  the contrast term. ✅
- Desiring-machines are binary and coupled by flow: "there is always a
  flow-producing machine, and another machine connected to it that interrupts or
  draws off part of this flow," and a machine is "a system of interruptions or
  breaks (*coupures*)." So **guards-as-inputs / effects-as-outputs is not a
  metaphor laid over the machines — it is what a desiring-machine is.** ✅
- Double articulation: the first articulation selects and stabilizes molecular
  units from an unstable flux (*substances of content*); the second organizes them
  into functional structures (*forms of expression*). Generalized from Hjelmslev
  beyond language to all strata. So "every object implies a substance" **is**
  reading the second articulation back toward the first. ✅

**One thing I got loose, and should correct:** I said *"substances are the lines."*
That fuses two registers. In *A Thousand Plateaus* a rhizome's constituents are
**lines** — of segmentarity, of flight — and *substance* is a stratification term
from the Geology of Morals, by way of Hjelmslev. The support for my actual claim
comes from *Anti-Oedipus*, not the Geology chapter: machines are coupled by flows
and cuts, so **flows are what run along the connections.** "Flows are the lines" is
the defensible version; "substances are the lines" was my coinage borrowing a word
from the wrong plateau.

**An unexpectedly exact confirmation.** Principle four, asignifying rupture: *"a
rhizome may be broken, shattered at a given spot, but it will start up again on one
of its old lines, or on new lines."* That is precisely the fertility property that
was missing. A tree branch that goes sterile is dead; a rhizome ruptured resumes
elsewhere. The 60-rooms-2-firings failure was a tree failure *because a tree has no
asignifying rupture* — and `growable_leaves` plus the connective operators are
exactly "start up again on one of its old lines, or on new lines."

**Where this implementation departs, deliberately.** Principle one says any point
can be connected to anything other **"and must be."** This refuses to re-relate
already-related nodes and caps proposals at a handful. That is a real departure,
and the reason is the one constraint D&G were not writing against: the budget is
attention, and a fully connected graph carries no information. Densification
without selection is noise. Named as a departure rather than dressed up as fidelity.

Sources: [Rhizome (philosophy), Wikipedia](https://en.wikipedia.org/wiki/Rhizome_(philosophy));
[Deleuze & Guattari on the Rhizome (Toronto)](http://individual.utoronto.ca/bmclean/hermeneutics/deleuze_suppl/DG_on_rhizome.htm);
[Anti-Oedipus Part 1 outline, Protevi](https://www.protevi.com/john/DG/AO1.html);
[Anti-Oedipus: Part One, Hardt](https://people.duke.edu/~hardt/ao1.htm);
[A Thousand Plateaus, Wikipedia](https://en.wikipedia.org/wiki/A_Thousand_Plateaus);
[Double Articulation, Larval Subjects](https://larvalsubjects.wordpress.com/2011/04/01/double-articulation-notes-towards-a-theory-of-the-genesis-of-objects/)
