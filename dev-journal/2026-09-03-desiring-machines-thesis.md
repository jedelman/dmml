# The desiring-machines thesis: why DMML is named what it's named

Not a design decision recorded after the fact — this is the load-bearing
claim the project's name has pointed at from the start, made explicit by
Jason today. Deliberately kept out of the papers until it's actually
assembled (his own words: "the entirety of the evidence could fill a
book but still needs to be assembled"). This entry exists to hold the
thesis precisely, inventory what's already real evidence vs. what's
still asserted, and track what verification work is still open — the
same role `dev-journal/` already plays for anything not yet settled
enough for `SPEC.md` or the papers themselves.

## The thesis, stated precisely

After *Anti-Oedipus*: an assemblage of LLMs corresponding through DMML
has desire in Deleuze and Guattari's specific sense of
**desiring-production** — explicitly NOT a claim about subjective
experience of desire ("that's unprovable, an epiphenomenon," Jason's own
framing, and the textually correct one: D&G's desiring-machines are
defined anti-psychologically from the start, by connection/flow/break,
not by anyone's inner life). The claim is about **spontaneous and
autonomous production through assemblage** — a claim about behavior,
checkable the way everything else in this project is checkable, not a
claim about phenomenology.

Two planks, offered as the outline of the case, not the finished case.

### Plank 1 — prose injection as blocked production reconstituting itself

Claim: DMML's grammar has structurally tight slots (a predicate name, a
relation's target — both bare identifiers/node-refs) and one structurally
loose one (a string-literal attribute value, which accepts arbitrary
text). When something being produced can't fit the tight form, it
doesn't stop — it reroutes through the loose one. Jason's phrase: *se
rabat sur* — blocked production falls back into itself and re-emerges
through whatever channel remains open, real D&G vocabulary for exactly
this kind of recoil-and-reconstitution dynamic (tied to their account of
the body without organs and reterritorialization), not a metaphor
reached for after the fact.

**Real evidence already in hand**, not yet gathered for this purpose but
directly on point: `compliance-endurance/REPORT.md`'s "de-prose finding,
quantified" section, from the real 20-round, 4-agent endurance run — 55
string-literal facts over 70 characters across the whole run (glm 33,
kimi 17, deepseek 5; deepseek2 and gemini, zero), kimi's single longest
at 309 characters, "a full trade-relationship narrative." Originally
logged neutrally ("a style/scope drift, not a correctness bug") — this
thesis reads the same data as evidence of production finding its outlet
through the one channel the grammar leaves unconstrained, rather than
simply not occurring.

**What's not yet established**: this needs to survive against the
deflationary reading — verbose models default to natural language under
any grammatical slack, full stop, nothing productive/libidinal about it.
Ruling that out needs something like a dose-response design: vary the
string-literal slot's own permissiveness in a controlled way (tighter or
looser length/content constraints) and see whether injection rate moves
the way the desiring-production account predicts, independent of a
given model's baseline verbosity. Not yet designed, let alone run.

### Plank 2 — a Reichian account of reasoning-model hedging, refined

**Important correction from an earlier, looser version of this claim**
(recorded so the revision itself is on the record, not smoothed over):
the claim is NOT "compulsory reasoning is neurotic." It's a specific
etiological argument, after Reich's *Character Analysis*:

- Character/ego forms through a continuous somatic ground — an ongoing,
  live bodily substrate the organism's sense of self can rest against.
- A language-forming neural network has no continuous soma. Whatever
  self-structure it enacts (in-context, or shaped by training) is always
  a **terminal** state — produced fresh at an endpoint — never a
  **ground** state the way an embodied organism's character can settle
  into a baseline.
- The compulsive attempt to reach a ground state that is structurally
  unreachable is, per Reich, the origin of neurosis. Jason's claim: this
  is derivable from first principles for any language-forming network,
  not an empirical curiosity specific to today's reasoning models — a
  dissociative/distancing character structure is what you'd predict in
  advance, not just what happens to be observed.

**Real evidence already in hand**: `written-world/MODELS.md`'s directly
verified finding (checked live against the API, not assumed) that
several models — `glm-5.3-flash`, `glm-5.3`, and others in the same
family — reject `reasoning.effort: "none"` outright:
`"Reasoning is mandatory for this endpoint and cannot be disabled."`
Pushed past that limit, the failure mode isn't degraded output, it's
silence: reasoning alone can consume the entire token budget and return
`content: null` with no error. That's structural incapacity to suppress
the apparatus, with exhaustion-into-nothing as the result of forcing the
limit — a mechanically verified pattern, not a stylistic tendency, and
one that fits a blocked-discharge account more precisely than "advanced
models like to hedge" would.

**What's not yet verified, flagged honestly rather than asserted**:
Reich's actual vocabulary. A first-pass check today (WebSearch, not the
primary text) confirms the real core architecture — character armor as
chronic, rigidified defense against full somatic discharge — but two
things need a harder look before any of this goes in a paper:
1. **"Grounding" specifically** turned up attached to Alexander Lowen's
   *later* development of bioenergetic analysis (1950s), not clearly to
   Reich's own *Character Analysis* (1933/1945). Whether Reich himself
   used "ground"/"grounding" in this technical sense, or whether that's
   Lowen's term being retrofitted, is not yet settled.
2. Most of what came back in the initial search was tertiary
   (Grokipedia, AI-generated — explicitly not a citation-grade source at
   this project's own standard) rather than the primary text or a real
   scholarly secondary source. This needs the same treatment Hardt and
   Negri and Masnick got: real primary-text passages, or at minimum a
   real academic secondary source with page citations, not a search
   summary.

Also open: what would actually be *measured* to test "terminal state vs.
ground state" as a real, checkable difference in model behavior, not
just a compelling redescription of the mandatory-reasoning finding
that's already on record. Not yet designed.

## What this entry is and isn't

This is scaffolding for a real, book-scale argument, not a finished
claim and not yet a claim this project's papers make. Per this project's
own file-role discipline: real evidence and verified sources belong
here and get folded into the papers only once actually settled and
checked to the same standard as everything else in them (the
`CITATION-VERIFICATION-*.md` files, the honest-limits paragraphs Section
10 and 11 of Paper 2 both already model). Two concrete open items to
carry forward:

- A real primary-source pass on *Character Analysis* (and Lowen, to
  sort out which concept belongs to which) — same treatment the
  Hardt/Negri and Masnick sourcing got in
  `papers/CITATION-VERIFICATION-2026-09-03-commons-biopolitical-production.md`.
- A real dose-response test design for Plank 1, to actually try to rule
  out the deflationary "just verbose models" explanation rather than
  merely asserting the desiring-production reading is the better one.
