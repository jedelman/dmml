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

## Standing methodological note for future sessions

Three rounds of "propose a move → verify citations → adversarial
review → find a real flaw → report it honestly" is real, working
discipline and should continue. What changed this round: the
*form* of the argument (a checklist of criteria to satisfy) was
itself part of the problem, not just its content — worth noticing
earlier next time a philosophical argument starts resembling a
spec-compliance check.
