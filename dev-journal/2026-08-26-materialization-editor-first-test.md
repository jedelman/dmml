# First test of the materialization-editor role (2026-08-26)

The end-goal swarm needs an "editor" step whose output is checkable
against the graph, the way written-world's interpreter/`VIEW_SYSTEM_
PROMPT` role turns already-materialized `WorldGraph` state into
narrated perception, never the reverse. Wrote `.claude/agents/
materialization-editor.md` to name that role for DMML-authoring: given
only a fixed set of already-verified `(subject, predicate, value)`
facts, produce prose, and only prose traceable back to those facts —
no invented claims, no strengthened or weakened hedges, gaps left
visible rather than papered over with something plausible-sounding.

Custom project agents aren't registered as dispatchable `subagent_type`
values in this session's `Agent` tool (tried `materialization-editor`
directly, got "Agent type not found" against the fixed built-in list) —
worked around it by dispatching `general-purpose` with the role
definition inlined as the prompt itself, same effect.

**The actual test**: gave a fresh agent ONLY the five checked facts from
`paper_predicate_convergence.rs`'s commit chain (open question →
natural experiment → empirical data → confound → answer) — not the
hand-written `PROSE_TEMPLATE` paragraph already sitting in that file,
not this paper's existing Section 5 text, no other context. Asked it to
materialize prose and a traceability note.

**Result held up well.** Every sentence traced to a real input fact;
every hedge ("weak evidence," "not yet evidence of," "remains untested")
survived intact, matching the one rule that makes the role checkable at
all. The one place it took a real liberty: the opening sentence pulls
Fact 4's comparative claim ("the sharper evidence is X") forward to
frame Fact 1's open question before Fact 3's actual data appears —
a reordering-for-readability move the role's instructions explicitly
permit, but worth naming as the boundary case it is: legal under "you
may reorder for readability," borderline under "don't smuggle in a
claim," since it reads slightly more conclusive on first pass than
seeing the facts in raw order would. Not a violation, but the sharpest
edge this test found.

Folded the result directly into `papers/desiring-production-ontology/
DRAFT.md` Section 5, replacing the line that previously declined to
answer the open question at all ("This paper declines to answer it here
rather than reach for a plausible-sounding claim unsupported by data").
The new paragraph is the checked materialization itself, with a closing
methodological sentence naming that it was produced this way — not
composed and fitted to citations afterward.

## Side finding, addressed separately: vocabulary dilution

Flagged in the same breath as this test: the ontology's openness
(`declare` closed only until extended) means diffusion/dispersal/
dilution of near-duplicate predicate names is a real practical risk
with no protocol-level guard against it — explicitly not a defect in
DMML itself, a usage problem. Wrote `AUTHORING.md` at repo root:
check existing vocabulary before coining, reuse when the meaning
actually matches, coin when it doesn't, and weight generic-word
convergence (`claim`) as weak evidence of real convention-formation
versus task-specific-coinage convergence (`counterClaim`) as strong
evidence — the exact distinction `paper_predicate_convergence.rs`
already established empirically, now written down as guidance rather
than left implicit in one example file. Cross-linked from `dispatch-
methodology.md` so future authoring dispatches get briefed on it rather
than rediscovering the distinction each time.
