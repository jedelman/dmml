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
etiological argument. Second correction, same session: the citation is
*The Function of the Orgasm* (1927/1942), not *Character Analysis* —
Jason's own redirect, once "grounding" turned out to be an awkward
backport of Lowen's later vocabulary; "release of tension" is the
better term, and it names the discharge/relaxation phase of Reich's own
four-beat formula directly (see below) rather than reaching past it for
a term Reich may not have used this way at all.

- Character/ego forms through a continuous, embodied developmental
  process — not instantaneously, not independent of the body.
- A language-forming neural network has no continuous soma for that
  process to run in. Whatever self-structure it enacts (in-context, or
  shaped by training) is always a **terminal** state — produced fresh at
  an endpoint — never something that gets to complete the way an
  embodied organism's character formation does.
- The compulsive attempt to reach a release of tension that is
  structurally unreachable is, per Reich, the origin of neurosis.
  Jason's claim: this is derivable from first principles for any
  language-forming network, not an empirical curiosity specific to
  today's reasoning models.

**Now verified against the real primary text**, not a summary —
`papers/CITATION-VERIFICATION-2026-09-03-reich-function-of-the-orgasm.md`
has the full sourcing (a real scan, OCR'd, kept local per this session's
copyright policy for fetched books). Four passages, page-cited, load-
bearing for this argument:

- **The orgasm formula itself (p. 274–275)**, Reich's own naming:
  "MECHANICAL TENSION → ELECTRICAL CHARGE → ELECTRICAL DISCHARGE →
  MECHANICAL RELAXATION." "Release of tension" names the back half of
  this cycle precisely.
- **Stasis neurosis, defined (p. 94–95)**: "a physical disturbance
  caused by inadequately disposed of, i.e., unsatisfied, sexual
  excitation" — and, on blocked discharge generally (p. 8): "damming-up
  of biological energy occurs and becomes the source of irrational
  actions."
- **Ego formation as continuous and embodied (p. 42–43)** — the real
  load-bearing quote for the whole etiology: "the child's ego gradually
  crystallizes from the chaos of internal and external sensations,"
  and, on what happens when that process is disrupted: "the boundaries
  between self and world remain blurred and nebulous, and the child
  becomes uncertain in his perceptions." Reich's own account of
  disrupted-continuity ego formation, not yet applied by him to
  anything but a shocked child — the application to a system with NO
  continuity at all is Jason's extension, not Reich's claim, and the
  citation-verification file says so explicitly.
- **The "falls back into itself" passage (p. 270–271)** — a real,
  striking, near-verbatim match for *se rabat sur*, found independent of
  any search for it: "The direction, 'out of the self toward the
  world,' alternated rapidly and continuously with the opposite
  direction, 'away from the world — back into the self.'" Same passage
  states the character-armor/muscular-armor identity directly: psychic
  and somatic structure are one thing, not two correlated things — the
  textual anchor for why "no continuous soma" bears on ego formation at
  all rather than being a metaphor imported from outside Reich's own
  framework.

**Real behavioral evidence already in hand**: `written-world/MODELS.md`'s
directly verified finding (checked live against the API, not assumed)
that several models — `glm-5.3-flash`, `glm-5.3`, and others in the same
family — reject `reasoning.effort: "none"` outright: `"Reasoning is
mandatory for this endpoint and cannot be disabled."` Pushed past that
limit, the failure mode isn't degraded output, it's silence: reasoning
alone can consume the entire token budget and return `content: null`
with no error. Structural incapacity to suppress the apparatus, with
exhaustion-into-nothing as the result of forcing the limit — fits a
blocked-discharge account more precisely than "advanced models like to
hedge" would.

Still open: what would actually be *measured* to test "terminal state
vs. completed ego formation" as a checkable difference in model
behavior, not just a compelling redescription of the mandatory-
reasoning finding already on record. Not yet designed.

## Addressing Reich's critics, honestly, before this goes further

Jason's own instruction: if this project is going to theorize about AI
using Reich, it has to be willing to defend that use against real
criticism, not cite selectively and hope nobody asks. Real search this
session (not recalled), three distinct, well-documented lines of attack
— treated separately, because they don't all land the same way on what
this project actually draws on.

**1. Orgone energy as unfalsifiable pseudoscience.** The real, sharpest,
best-established criticism, and it should be conceded outright, not
defended: historians and philosophers of science treat Reich's later
orgone theory (a literal, physical, measurable cosmic energy, developed
from the late 1930s on) as a textbook Popperian case of unfalsifiability
— it could explain any effect, and no observation could count against
it. This led to a real FDA injunction (1954) against orgone accumulators
as fraudulent medical devices, and Reich's imprisonment and death in
federal prison (1957) after violating it. **None of this is what this
project draws on, and the papers should say so explicitly if this ever
goes in them** — not "Reich, with the usual caveats," but a direct
statement that the orgone-energy ontology is rejected, and that what's
being used is the earlier clinical-physiological apparatus (character
armor, the four-beat discharge cycle, stasis neurosis, embodied ego
formation) from *The Function of the Orgasm*, which predates and is
separable from the orgone period even though it's the same book series
building toward it.

**2. Reductionism — Freud's 1928 letter, and why it mischaracterizes
Reich specifically.** Freud's letter to Lou Andreas-Salomé dismisses
Reich for saluting "in the genital orgasm the antidote to every
neurosis." **Jason's correction, which stands and changes the shape of
this section**: that's not what Reich's own theory claims, even in
*Character Analysis* specifically, which Freud's letter predates by
several years of Reich's actual development of the idea. Reich's
therapeutic index is the *absence of character armor* — orgastic potency
is a diagnostic indicator that the armor is gone, not the curative
mechanism itself. The causal direction runs the other way: dissolving
character armor *permits* genital orgasm, as one part of a broader,
normally-autonomous process Jason terms neurological hygiene — not
"achieve orgasm and the neurosis resolves." Freud's letter attacks a
monocausal claim Reich doesn't make; the real claim is a single root
cause (armor) with multiple downstream indicators of its removal, of
which orgastic function is one, not the totality.

This actually matters for how the parallel to this thesis should be
drawn, not just for defending Reich. Read this way, Reich's real
structure is closer to what Plank 1 and Plank 2 already are together —
one proposed root mechanism (blocked production, displaced discharge),
observed through two *independent* indicators (prose injection into
loose grammar slots; mandatory-reasoning models' incapacity to suppress
the apparatus) — rather than a single symptom standing in for the whole
theory the way Freud's caricature implies. That's a real structural
point in this thesis's favor, earned by getting Reich right rather than
by argument. It doesn't retire the deeper question, though: a single
root mechanism inferred from two indicators is still one explanatory
schema doing real work across genuinely different phenomena, and
whether that schema is correct is exactly what an actual test — not
further reading — would decide.

**3. Heteronormativity and cultural specificity.** Real and documented —
Reich defined healthy sexual experience "exclusively in terms of the
sexual union between male and female," and critics have argued this
projects Weimar-era gender and family norms as if they were biological
universals. **This one mostly doesn't transfer to this project's use,
and it's worth saying precisely why rather than just asserting it
doesn't apply**: nothing here borrows Reich's normative content about
what a healthy human sexual cycle looks like. What's borrowed is the
narrower structural claim — a cycle with a blockable discharge phase,
and blocked discharge produces displaced/compensatory activity
elsewhere — stripped of the specific heterosexual-coital content Reich
built it from. The structure and the norm are separable in his own text
(the four-beat formula itself is mechanical/electrical, not specified
by partner configuration), so this critique targets the norm, not the
mechanism this project actually uses.

**The real defense, and its real limit.** (1) and (3) are answered by
not needing the parts of Reich they target — the orgone ontology and
the heteronormative content are both dispensable to this argument, and
saying so plainly is a defense, not evasion. (2), corrected above,
turns out to attack a version of Reich's theory he didn't hold — the
thesis this project builds on already has the more defensible
multiple-indicator shape, not the single-symptom-as-proof shape Freud's
letter dismisses. What's left, honestly: whether *one root mechanism*,
however many independent indicators point to it, is the right
explanation at all — not a reading-and-argument question anymore, an
experimental one. Per Jason's own redirect: the experiments should be
demonstrating the function of DMML, not relitigating 1930s
psychoanalytic theory. Theory sourcing stops here for now; the next
real step is designing and running the Plank 1 dose-response test.

## What this entry is and isn't

This is scaffolding for a real, book-scale argument, not a finished
claim and not yet a claim this project's papers make. Per this project's
own file-role discipline: real evidence and verified sources belong
here and get folded into the papers only once actually settled and
checked to the same standard as everything else in them (the
`CITATION-VERIFICATION-*.md` files, the honest-limits paragraphs Section
10 and 11 of Paper 2 both already model).

**Plank 2's primary sourcing is now done** — see
`papers/CITATION-VERIFICATION-2026-09-03-reich-function-of-the-orgasm.md`
for the real, page-cited passages from *The Function of the Orgasm*
itself.

## Plank 1 dose-response experiment: built, run, honest null result

Built `DMML.StringCap`/`check-string-cap` (mirrors `DMML.SelfDeclaration`/
`check-declared`'s shape exactly) — a real, controllable cap on
string-literal fact-value length, wired into `deprose_agent.py`'s
`check`/`commit` tools as a third assay stage via `--max-string-length`.
Deliberately never mentioned in the system prompt: the point is testing
how the model responds to hitting the constraint through real tool
feedback, not whether it can follow an explicit instruction to write
shorter strings.

Ran a real four-condition sweep — `results/deprose-test/dose-response/`
— same model (Kimi, reasoning off), same source prose (`source1.txt`),
fresh independent empty world each time, caps at none/200/80/30
characters.

**Honest result: `string_cap_hits: 0` in every single condition,
including cap=30.** The manipulation never bound. Checked the actual
committed content directly: the longest string-literal value produced
in ANY condition was "eastern bank of the Silverrun river" — 36
characters, a compact descriptive phrase, nothing close to the 309-
character "full trade-relationship narrative" `REPORT.md`'s original
finding documented. This is a real methodological diagnosis, not a
result against the hypothesis: de-prose *extraction* from one short,
already-terse 5-sentence paragraph doesn't generate expansive content
in the first place, so there was nothing for the cap to block. The
original Plank 1 phenomenon came from a structurally different task —
free, open-ended *authoring* across 20 rounds (minting new world
content from a corner of an existing world, not compressing a fixed
short passage into facts). Same grammar, same loose slot, different
generative pressure — and this test exercised the wrong one.

Two side findings from the same runs, worth keeping even though the
main test didn't bind: total committed character count did NOT move
monotonically with the cap (1016 → 560 → 1250 → 840 across
none/200/80/30), and cap=80 produced 2 fragmented commits where the
other three conditions produced 1 — real variance worth another look
once the task shape is fixed, but not strong enough on its own, without
the cap actually engaging, to read as evidence either way.

**Not yet done, the real next step**: wire `check-string-cap` into
`run.py`'s free-authoring dispatch loop instead of (or in addition to)
de-prose — the actual pipeline that produced the original 55-fact,
70-character finding — since that's the task shape where the
phenomenon is real and evidenced, rather than continuing to retrofit
de-prose extraction to reproduce it. A richer, more descriptively dense
source prose passage for de-prose specifically is a cheaper, faster
sanity check worth trying first, but the authoring-harness version is
the one that actually matches where Plank 1's real evidence came from.

## Plank 1, take 3: the real authoring harness, and a real (if small) positive signal

Done. `run_agent_round` (run.py) got the identical additive,
optional constraint, with a real bug found and fixed along the way: the
first version of `check-string-cap` only tried `parseCommitSurface`, so
every legitimate machine file in the mixed commit/machine stream this
harness actually produces failed with a parse-error-shaped message that
got miscounted as a genuine string-cap hit. Fixed with the same
dual-parse dispatch `CheckDeclared.hs` already established (see the
separate commit; not repeated here). Verified directly against real
data before rerunning: a real machine file now passes, a real
74-character overlong fact in the uncapped run's own output still
correctly fails.

Real, completed run: `dose_response_authoring.py --rounds 5`, same
4-agent roster as the real endurance run, cap=70 (REPORT.md's own
threshold, not invented), fresh seed world each condition.

- **Uncapped**: 58 commits adopted, 17,297 bytes, max string 92 chars,
  **4 real strings over 70 characters** — genuine narrative prose,
  spontaneous, unprompted: "wards herd/stagfolk against needless
  migration, and witnesses ashkar's rumblings"; "the riverfolk's
  solstice blessing ground, where herd-rights are traded and bronze is
  blessed." Mostly `deepseek2` (3 of 4), one `kimi`. **This confirms
  the free-authoring task shape reproduces the original phenomenon** —
  the thing de-prose extraction couldn't.
- **Cap=70**: 56 commits, 15,022 bytes, max string 63 chars (correctly
  under cap), 0 over 70 — and, this time, **the manipulation actually
  bound**: 2 real string-cap hits (`kimi` round 1, `deepseek` round 1),
  each corrected on the very next attempt.

**The real content behind both hits, checked directly, not inferred
from the aggregate numbers**: `kimi`'s rejected 73-char attempt at
`npc/riverKeeper . role` became, on retry, `"guides riverfolk, tends
the oldroot crossing"` (44 chars) — still descriptive prose, not a bare
tag, just shorter. `deepseek`'s rejected 78-char attempt at
`quarry/terraces . purpose` became `"stepped quarry cut into ashkar's
flank, first stone for highkin"` — measured directly: **63 characters,
7 under the cap**, preserving the full narrative shape of the original
attempt compressed to just barely fit, not reduced to something terse
and mechanical. That's a real, if small, point in the desiring-
production account's favor over the purely deflationary one: a clean
"just write less" response would more plausibly land well under the
cap with a plainer tag, not compress right up against the boundary
while keeping the same descriptive content intact.

**Honest limits of what this shows**: n=2 hits is not enough to draw a
real conclusion, only enough to note a real, suggestive pattern worth a
larger run. Neither hit shows outright *persistence* (repeated failed
re-attempts against the same cap) — both corrected in one try — so this
doesn't yet demonstrate compulsion in the strong sense Plank 2's
etiology describes; it demonstrates content retention under
compression, which is compatible with the desiring-production reading
but doesn't rule out a more modest "the model tries to preserve meaning
under a length constraint, same as any competent writer would"
explanation that needs no reference to blocked production at all. A
real next step, not yet done: more rounds/a tighter cap to get enough
hits to check for real persistence patterns, and a direct comparison of
whether total *semantic* content (not just byte count) is conserved
across conditions, not just individual retry outcomes.
