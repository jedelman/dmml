# Section 4's road to the transindividual argument (2026-08-24)

Working record of what was tried, in order, while drafting the
desiring-production-ontology paper's Section 4 (Simondon's
transindividual applied to a DMML petition). The paper itself states
its current position without narrating this history — this is where
that history actually lives, per the "content shouldn't reference its
own previous versions" rule adopted this session; git log and this
file are the record, not the paper's own prose.

## Attempt 1: constraint propagation via append-only history

First cut: DMML's referential-integrity check (issue #53, in
written-world) means later commits must cite genuine prior facts, so
a petition's resolution reshapes both parties' future citable claims
— read as Simondon's individuation-through-relation. Adversarial
review (`stealth/ox-alpha`) found this factually wrong about the
software: the check verifies a citation is genuine, not that later
commits stay *consistent* with it. Also found the underlying move —
path-dependence via an append-only log — is satisfied by any
event-sourced system (git, a blockchain); nothing distinguished
DMML's case. Verdict at the time: "GitHub is not a transindividual
collective."

## Attempt 2: rhizome vs. arborescent (real progress, kept)

Jason's pushback: git and blockchain are arborescent in Deleuze and
Guattari's own specific sense (*A Thousand Plateaus*); DMML's actual
connectivity (N-ary `consumes`, `FactRef` addressing any triple in
any prior commit, cross-DID quotation, mergeable-by-default
semantics) isn't. Checked via a real citation pass before drafting
(git is a DAG with a strong convention, not a tree; blockchain is the
fair arborescent case; D&G name six rhizome principles, the paper
draws on three, labeled as such). A second review found this still
didn't reach far enough — the same criteria describe the open web,
RDF, and citation networks, all of which DMML's own graph resembles.
Answered with a further property: DMML's connections are verified
productions (issue #53 again, read correctly this time), not claims
that can be figurative the way a hyperlink or citation can misrepresent
its target — tied back to the paper's own Section 2 ("real ones, not
figurative ones"). This survived review and is still in the paper.

## Attempt 3: agrammaticality and the runtime event (tried, rejected)

Jason's further pushback: relocate "charge" from the graph's state
(risking a stasis/charge confusion already flagged) to desire's own
movement through the grammar — D&G's real, ATP-native concept of
*agrammaticality* (verified: a genuine, D&G-stated link to
deterritorialization, "Postulates of Linguistics" plateau, the
Cummings "he danced his did" example), applied by analogy to a new
self-declared predicate entering DMML's open vocabulary. And relocate
"relation as site of individuation, not registered afterward" from
the persistent graph to the runtime event of `apply_commit` itself,
grounded in Simondon's own physical-register individuation
(crystallization needs no consciousness or co-presence — also
verified).

Both citations checked out as real. Both applications didn't survive
a third review round:

- Agrammaticality runs backwards. It works because an agrammatical
  expression violates a constraint the speaker is otherwise bound by.
  DMML's self-declaration is a *sanctioned* extensibility feature the
  grammar defines and expects — no resistance, no transgression. Worse,
  if routine self-declaration counted, so would any new URI minted in
  ordinary linked data, reopening the exact problem attempt 2 had just
  closed.
- The runtime-event move rests on a factual error about the software,
  same category as attempt 1's mistake: a commit's `produces` content
  is authored *before* `apply_commit` runs. The interpreter checks and
  applies, it doesn't generate. Indeterminacy resolves at authoring
  time, not execution time — and relocating to authoring time instead
  runs straight into Section 3's own concession about the resolver's
  internal process (possibly "selection from a menu" after all).

## Attempt 4 (in progress): multiplicity, haecceity, speed — a different mode of argument, not just a different location

Jason's sharpest correction, after three rounds of checklist-style
verification: the whole approach was mis-strata'd. Checking whether
DMML "satisfies" Simondon's individuation criteria like a theorem to
be proven is itself an arborescent move — exactly what Section 4's
own rhizome material argues against doing. D&G's own opening move in
*A Thousand Plateaus* ("since each of us was several, there was
already quite a crowd") starts from multiplicity, not from a stable
self later split apart. Individuation-as-a-permanent-self is, on this
reading, closer to a territorialization — a coding effect — than to
what's actually real, which is always stranger and more mobile than
that. Charge, on this view, isn't a state (reconciled or not) or a
discrete event (a runtime execution) — it inheres in the *fact of
change itself*, which D&G call speed. This points toward haecceity
(individuation without a subject or substance — an event, a season, a
"this," individuated by its own configuration of speed/affect
relations, not by persisting as a self) as a better-fitting D&G
concept than a checklist run against Simondon's criteria one at a
time. Citation-checked (`CITATION-VERIFICATION-2026-08-24-
multiplicity-haecceity.md`) before the rewrite that replaces Section
4's checklist structure with an argument tracing this movement
directly, rather than verdict-by-verdict against a fixed criterion
list.

## Attempt 4, adversarial review round: haecceity alone proves too much (fixed, not abandoned)

A `stealth/ox-alpha` adversarial review of the multiplicity/haecceity version
above (real review, not a hypothetical stress test) found the section's
actual climax — "a petition's resolution demonstrably is... a haecceity" —
was a satisfaction-verdict wearing tracing vocabulary: strip the predicates
(singular, unrepeatable, positioned in a citation network, irreversible) and
every one holds of any git commit, any HTTP request, any database
transaction. The section had already made exactly this "proves too much"
move against the web/RDF/citation-network comparison in its rhizome half,
and committed the identical error one paragraph later without noticing.
Verdict: DO NOT SHIP as drafted. Three smaller findings came with it: the
"every edge is non-figurative" claim overclaimed what the referential-
integrity check actually guarantees (grounding, not semantic fidelity — a
commit can cite something real and still misrepresent it, and nothing
catches that); the Simondon-to-D&G handoff read as a target substitution
executed mid-argument rather than a thesis announced up front, with an
unattributed quote and un-named scholarship; and the anti-checklist
methodology was violated by its own final paragraph's "demonstrably is...
without further argument needed" verdict language, plus a minor Simondon
publication-date ambiguity and an overstrong claim about git being
unable to reference across repository boundaries at all (submodules do,
by copy).

Fixed, not abandoned, unlike attempt 3 — the review's own suggested repair
worked: rebuild around what actually distinguishes DMML structurally rather
than around bare eventhood. The *mergeable*-default-vs-*arbitrated*-narrow-
exception split (already in the paper's rhizome material as a blockchain
contrast) is read as deterritorialization-as-default against a declared,
local act of territorialization — a structural distinction most event-
recording systems don't make available at all, unlike singular/irreversible/
positioned-in-a-graph, which every one of them shares. Cross-DID quotation
across a sovereign identity boundary is read as connection across a molar
boundary that isn't erased by the connection — again a specific grammar
fact, not a generic one. Haecceity is now used only once specified by these
two structural facts, with explicit defeasibility conditions stated (if
arbitrated were the default, or cross-DID quotation collapsed repositories
into one record, the argument would fail on its own terms) — answering the
"unfalsifiable escape hatch" objection the same review raised. The
"non-figurative edges" claim was narrowed to grounding, with semantic
infidelity conceded explicitly and the mergeable default reframed as what
keeps a misreading from being silently laundered into canonical status
rather than as a repair of the fidelity gap. Simondon's dates were
corrected (main thesis 1964, complementary part posthumous 1989); the git
submodule overclaim was fixed; the D&G-radicalization thesis now opens the
section as the substantive claim rather than arriving as a fallback.

## Attempt 5: strata/deterritorialization replaces Simondon-individuation as the primary register; a real pantheon simulation grounds it (2026-08-25)

Jason's direct feedback on the fixed multiplicity/haecceity version: still leaning
too heavily on Simondon and individuation, when the actual target is D&G's own
apparatus — differentiation and deterritorialization/reterritorialization on,
against, and off the plane of consistency through strata — and DMML should be
read as producing a novel *auto-recombinant* form of the rhizome. Also asked for
concrete simulations (e.g. a pantheon) rather than prose claims alone.

A fresh citation-verification pass read the actual Massumi-translation text of
"10,000 B.C.: The Geology of Morals" and the "Rhizome" introduction directly
(not secondary paraphrase), confirming: strata as double articulation
(content/expression is Hjelmslev's vocabulary, not D&G's own coinage);
deterritorialization/reterritorialization as a coupled pair, with an
absolute/relative distinction turning on *nature*, not speed; the plane of
consistency paired against "strata/Ecumenon" specifically in this chapter, not
"plane of organization" (that belongs to a different plateau — a real
correction to an assumption in the initial draft of this reframe). Full report:
`papers/CITATION-VERIFICATION-2026-08-25-strata.md`.

Building the concrete simulation surfaced a real, previously-uncaught error:
attempt 4's rebuild (above) claimed DMML's grammar has a declared `mergeable`/
`arbitrated` consume-kind distinction. That distinction is real but is a
*substrate-layer design stub* for a not-yet-built iroh backend
(`dmml-runtime/src/substrate.rs`'s own doc comment says so explicitly) — it has
never existed as a grammar primitive, and no adversarial review round had
caught this because none had been asked to check that specific claim against
the actual crate. `dmml/examples/pantheon.rs` was written specifically to
avoid relying on unbuilt primitives: three independently-authored commits
(Helios, Selene asserting rival, uncoordinated claims about the same fact),
one recombining commit (Nyx, consuming both via real `FactRef`s and producing
a genuine synthesis neither input contained), and one stratifying commit (a
council declaring a `canonicalOrigin` on top of, not instead of, the
underlying multiplicity). Four checks run and assert on real interpreter
output: (1) the *current* materialized view is honest last-write-wins, not
coexistence — stated plainly rather than papered over; (2) the log itself
preserves both original claims, independently re-materializable and citable
forever; (3) Nyx's multi-fact `consumes` really does fold two divergent prior
facts into a new production; (4) the council's declaration is a separate,
additive predicate that doesn't touch the underlying `origin` facts. Building
this also surfaced a second real error: Section 1's "referential integrity"
framing overstated what the portable `dmml` crate itself guarantees — a
dangling `FactRef` is a documented, formally-certified no-op
(`fact_retraction_fails_open`), not a rejected commit; the actual admission-
time check the paper had in mind is written-world's atproto-specific,
substrate-layer gate (issue #53), a different layer entirely. Both errors
fixed in the same pass, not just the one the user's feedback pointed at.

Section 4 rebuilt around the verified strata/deterritorialization apparatus,
Simondon demoted to a brief opening/closing gesture (real, but the wrong
primary vocabulary), and the "auto-recombinant rhizome" claim given a precise,
checkable referent: a commit's `consumes`/`produces` pair is literally D&G's
own double articulation, applied recursively to the graph's own accumulated
multiplicity as an ordinary act of the grammar, demonstrated by Nyx's commit
rather than asserted about DMML in the abstract. An honest limit was kept
rather than oversold: nothing in this round claims DMML exhibits *absolute*
deterritorialization in D&G's stronger sense — only the more modest, verified,
relative kind, checkable and shown.

## Attempt 5, adversarial review round: double articulation overgeneralized, fixed with a stated triad (2026-08-25)

A fresh `stealth/ox-alpha` adversarial review of the strata/deterritorialization
version (attempt 5 above) found it strongest yet — the mergeable/arbitrated
and referential-integrity fixes held, the primary-text work was judged real —
but found one recurrence of the exact haecceity-era failure in a new
subsection: "Nyx's commit is a real, literal act of double articulation, not
a metaphor reaching for one" is an unblocked overgeneralization, since a SQL
`JOIN`, a git merge, or a SPARQL `CONSTRUCT` query satisfies the identical
bare description (read several records, write one back), and the text didn't
notice. The review also flagged that "auto-recombinant" as stated required
nothing beyond ordinary relative deterritorialization plus an append-only
store — true of Datomic, event-sourcing, and SPARQL-over-RDF generally — and
that a stray "a submodule copies; it does not live-reference" was a factual
error (a submodule pins a SHA reference, not a copy), a category of mistake
this paper cannot afford a fourth time.

Fixed, not abandoned: double articulation demoted from literal identity claim
to explicit structural analogy, with the SQL/git-merge/SPARQL-CONSTRUCT
concession stated in the text itself (mirroring the rhizome subsection's own
earlier concession to the web/RDF comparison). The actual differentiator
named is a triad — grounded citation (Section 1), cross-sovereign
connectivity (no shared authority required), and a default of
non-convergence — argued to be what distinguishes DMML's case, not double
articulation alone; the same triad is used to answer the Datomic/event-
sourcing comparison directly rather than leaving it unaddressed. The git
submodule sentence was corrected (pins a reference, doesn't copy, still needs
a separate resolve step). The Simondon opening paragraph was trimmed per the
review's "throat-clearing with a bibliography" finding. A logically inverted
closing inference ("needed no bespoke primitive, which is itself evidence the
reading is right") was corrected — cheapness of fit is not evidence of
correctness, only of non-special-casing, and the text now says so. `pantheon.rs`
was extended from two rival prior facts to three (adding a third deity, Eos)
so the "or, in principle, arbitrarily many" recombination claim isn't resting
on the smallest case that could look coincidental — a real code change, not
just a hedge in prose, matching this project's "try it first" discipline.

## Correction: strata are relative, never singular (2026-08-25)

Jason's direct catch: both papers had drifted into treating DMML (or its
representational form) as "a stratum," full stop — exactly the kind of
flattening ATP's own text explicitly warns against. Verified directly
against the Massumi translation: "Each stratum serves as the substratum for
another stratum" (p. 73), with "no fixed order" to that relation (p. 64) —
strata come in a nested, relative stack, never as one freestanding layer.
A single stratum also decomposes internally into **epistrata** and
**parastrata** (pp. 50–52), "strata in their own right." One real
terminological correction surfaced by checking rather than assuming: the
author's own working term "superstratum" does not appear anywhere in the
Massumi translation (confirmed by full-text search) — D&G's own word for
the relation is just "another stratum," not a coined counterpart.

Fixed in both papers: paper 1's Section 4 now states the relativity
principle up front, with real page citations, and explicitly maps
`pantheon.rs`'s own chain onto it — Helios/Selene/Eos's commits each serve
as substratum for Nyx's synthesis, which itself serves as substratum for
the council's declaration, a real, checkable substratum chain rather than
a metaphorical "layer." Paper 2's Section 7 (and its abstract line) were
retitled and reworded from "symbolic representation is itself a stratum"
to "is a further stratum, not the only one" — DMML's representational form
serves as substratum for the commit content built on top of it, and a
continuous latent representation starts a different stack, not a
less-coded single layer. Full findings in
`CITATION-VERIFICATION-2026-08-25-strata.md`'s new §5.

## Standing methodological note for future sessions

Three rounds of "propose a move → verify citations → adversarial
review → find a real flaw → report it honestly" is real, working
discipline and should continue. What changed this round: the
*form* of the argument (a checklist of criteria to satisfy) was
itself part of the problem, not just its content — worth noticing
earlier next time a philosophical argument starts resembling a
spec-compliance check.
