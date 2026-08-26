# Autoregressive dispatch cycles: generation, not convergence (2026-08-26)

Ran the experiment Jason asked for directly: same independent-reader
role, same base prompt, dispatched three times in a row against the
paper's current full text (now including the Section 5 addendum from
earlier today), each round also given every prior round's output with
an explicit instruction not to repeat it. The question was methodological,
not philosophical: does repeated dispatch against a fixed-but-growing
body of material converge (later rounds restate or trivially rephrase
earlier ones) or generate (later rounds keep finding real, distinct
content)? For this run, unambiguously generation — all three rounds are
genuinely distinct, none is a rephrasing of another, and the third is
arguably the sharpest of the three, meaning depth didn't taper off as
material accumulated.

(Same dispatch friction as the materialization-editor test: the custom
project agent types aren't directly dispatchable as a `subagent_type` in
this session's `Agent` tool. Worked around it the same way — inlined the
role as a `general-purpose` prompt each time.)

**Cycle 1** (paper only): fail-open citation semantics (Section 1,
`fact_retraction_fails_open`) undercuts the auto-recombinant claim
(Section 4) — nothing structurally distinguishes a legitimate synthesis
from citation-grounded nonsense; both satisfy "grounded" in exactly the
sense the auto-recombinant claim requires.

**Cycle 2** (paper + cycle 1, told not to repeat it): a different
connection entirely — Section 4's non-convergence claim (nothing forces
convergence on facts) is in real tension with Section 5's own
convergence finding (predicate vocabulary DOES converge under
shared-context dispatch, the exact condition real multi-author worlds
run under). If schema convergence is real and shared-context-driven,
nothing stops the same pressure from eventually producing convergence
on canonical facts too — auto-recombinant multiplicity might be a
transient phase, not a stable structural property.

**Cycle 3** (paper + cycles 1–2, told not to repeat either): the
sharpest of the three, and a genuinely different register — not a new
structural tension inside Section 4, but a self-consistency gap between
Section 3 and Section 5. Section 3 explicitly flags that LLM-sampled
petition resolution might be "selection from an already-determined
menu," not production, and declines to resolve it. Section 5's own
convergence evidence comes from two LLM-dispatched authoring agents —
the identical suspect structure — without applying the same suspicion.
Two LLMs converging on `distanceStrategy` could just be two samples from
one distribution converging on its mode, a claim about LLM sampling in
general, not about DMML. No control condition (human authors, an
unrelated domain) separates the two. This is the paper failing its own
evidentiary standard, applied unevenly between two of its own sections.

Built all three as real commits in `dmml/examples/autoregressive_
critique.rs`, each consuming the actual pair of base-paper claims it
triangulates (never another cycle's output — checked directly, Check 1),
confirmed no two cycles triangulate the same fact-pair (Check 2, real
structural distinctness, not just different wording), and confirmed all
three remain independently real in one combined log (Check 3 — none
retracts another, since none consumes another's output). Per
`AUTHORING.md`'s own reuse guidance, written the same day, this file
reuses `claim` rather than coining `critiqueClaim` — a critique is a
claim about a claim, and the existing predicate already fits; a
near-duplicate here would have been exactly the dilution the guidance
warns against. First real test of that guidance being applied rather
than just stated.

Folded all three, unpatched, into `DRAFT.md` as a new subsection ("Open
critical debts, surfaced by autoregressive dispatch") right before
Section 6. Deliberately did not try to resolve any of them — patching
piecemeal risks exactly the "plausible-sounding claim unsupported by
data" this paper elsewhere declines to reach for. They stand as real,
checkable debts against the commit log that raised them, same register
the paper already asks its other claims to be read in.

Three rounds is a small n — not enough to claim generation-not-
convergence as a general property of this dispatch method, only that it
held for this run. Worth another few rounds sometime to see where (or
whether) it eventually plateaus; not done here.
