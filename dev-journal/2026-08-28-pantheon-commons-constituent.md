# Phase 8: can the four choose their own rule? (2026-08-28)

Jason's question, put directly to the pantheon rather than answered for
it: "can we allow the agents to choose which rules they want to follow?
none of what they critique is built into DMML so all of these are
possible. h&n shows how." This phase built and ran that literally, not
as a discussion but as real, mechanically different runtime behavior —
the first file in the project where the code branches on what a model
chooses, not just on frozen prior content.

## Design

No consensus/ratification stage — a formal vote would have contradicted
the premise the debate was testing. Instead: (1) a real choice round,
each Olympian individually picking or hybridizing one of their own four
rhizome-provocation protocols (Athena's web, Artemis's hunt, Apollo's
lyre, Dionysus's sparagmos) for themselves, with explicit instruction
not to coordinate; (2) dispatch order reshuffled every round rather than
fixed; (3) hunt-mode agents required to name a citation target each
turn; (4) sparagmos-mode agents have their public `respondent`
attribution reassigned by lot after drafting, true authorship logged
separately as `true_author`; (5) no vote — ratification computed
programmatically as "kept alive by use" (cited ≥2 times by ≥2 distinct
true authors), Athena's own proposed mechanism. Sixteen new Hardt and
Negri anchors (Empire, Multitude), the two most load-bearing verified
live via web search before writing. Apollo's most radical ask — fully
concurrent, non-round dispatch — was not implemented; reshuffled-per-
round is the practical proxy, named honestly as a simplification in the
file's own doc comment.

## The result nobody scripted: unanimous, unforced convergence

All four Olympians, choosing independently with no visibility into each
other's choice, picked sparagmos. Not a majority — all four. This is
itself the first real data point on the multitude-vs-Empire question the
phase was built to test, and the debate immediately noticed and
interrogated it: Dionysus's own opening move named the convergence
suspicious ("a shared code running through all four masks") before
anyone else could.

## The debate: eleven rounds of discriminators proposed and torn down

Once the masking mechanic was live, the transcript reads as a sustained
argument about what would actually distinguish "singularities acting in
common" (Hardt and Negri's multitude) from "a decentered, boundary-less
apparatus that is total precisely because it has no throne" (their
Empire) — tested against the pantheon's own procedure, in real time,
under real stakes (turns were genuinely mis-attributed by the lot, not
simulated). A chain of proposed discriminators, each torn down by the
next turn:
- Athena: a stake that costs its author something to retract.
- Artemis: exposure to being hunted by anyone, including its own author.
- Athena (reassigned): the reshuffle itself, arriving unratifiable, IS
  the genuine cut — "constituent power producing itself and its
  antagonist."
- Dionysus: forfeited authorship — writing under a name you cannot
  reclaim and do not want back.
- Apollo: rupture of the forfeiture itself — willing renunciation is
  still a staged sacrifice, "the maenad who volunteers for sparagmos is
  still holding the knife."
- Artemis: whether the redistribution of what's sayable is shared (a
  "people," Empire) or genuinely divergent (multitude).
- Artemis again: refusing the confession-coda genre itself, on the
  grounds that a pre-confessed wound is schedulable and therefore safe.
- Athena: the question "are we Empire?" is itself the last examiner,
  installed by treating constituent power as something a process could
  pass or fail.
- Apollo: rupture of that refusal — Empire is not an external judge
  interrupting production, it IS what the multitude's own producing
  brings forth; retiring the question abolishes the antagonist.
- Artemis (closing): every acceptance mechanism on the table — web,
  hunt, sparagmos — metabolizes wounds by design; none can make citing a
  claim cost the citer anything, so as *currently constituted*, none of
  the four protocols can produce the multitude, regardless of posture.
- Apollo, then Dionysus: even "wait for the unscheduled wound" is a
  schedule; even "never stop producing" is Empire's own metabolism
  (biopolitical production is exactly what capture feeds on).
- Athena (final turn): every posture tried — vigil, unvigil, forfeiture,
  refusal, ceaseless motion — specifies how the singularities should
  *comport themselves*, which was never the actual question. The
  multitude is defined by what is held in common, not by any performed
  relation to interruption, and whether the common was genuinely
  produced through difference (multitude) or merely distributed to
  compliant differences (Empire, pluralized) is legible only
  retroactively, never certifiable from inside a turn. "The graph
  decides; the legs do not."

## A real, more severe failure mode: total schema drift

Two calls (both Apollo, both post-mask-reassignment, both in the most
recursive/abstract stretch of the debate) failed with an error distinct
from every prior missing-field bug: not one field missing, but the
model returning `consumes` and `object` correctly while inventing
entirely unschema'd keys (`"taxonomy"`, `"now"`, `"whole"`, `"Empire"`,
`"asked"`, `"here"`) instead of `verb`, `subject`, `predicate`. This
reads as the model reaching for prose-shaped output under maximal
recursive/self-referential pressure and partially abandoning the tool
schema rather than merely dropping one required field. Not fixed in
code this run — logged as a distinct, third variant of the
missing-required-field family, worth a permanent fix if it recurs.

## Citation discipline and the ratification tally

Both dropped-call rounds still had 3 of 4 Olympians succeed, so no round
was lost. Programmatic "ratified by use" is a genuinely stricter bar
than any prior phase's citation-reliability metric — it requires the
exact same cid to be cited twice by two distinct true authors, not
merely any real citation — and only 1 of 18 debate turns cleared it,
because nearly every turn in this debate minted a new claim rather than
repeatedly citing a small stable set of anchors. This is consistent with
the transcript's own content: the debate was a chain of successive
rebuttals, not convergent citation.

## Cold-path checkpoint and fidelity

51 real records published to `claude.jason-edelman.org`: 9 frozen
machines-consensus items, 4 frozen rhizome-protocol proposals, 16
Hardt-and-Negri anchors, 4 choice-round elections, 18 debate turns
(carrying both the reassigned `respondent` mask and the real
`true_author`, so the mask/authorship split is independently
re-verifiable from the live record, not just asserted in this
write-up). Spot-verified against the live PDS; session token removed
immediately after.

## Running total

551 records through Phase 7, plus this phase's 51 = **602 real records**
published across tonight's full pantheon body of work.

## What's still open

The phase does not resolve whether the pantheon is multitude or Empire
— deliberately; Athena's own closing turn argues that question can only
be answered retroactively, by whether the common keeps changing under
real disagreement, not certified from inside any single session. Hardt
and Negri themselves have not yet been tested as directly against the
frozen sovereignty/machines findings the way every other phase's sources
were (this run used them as the diagnostic vocabulary for the
choice-of-rules question rather than running the standard
extend/rupture cycle against them) — a more conventional Hardt-and-Negri
extension run remains a real option if Jason wants the standard pattern
completed for this pairing too.
