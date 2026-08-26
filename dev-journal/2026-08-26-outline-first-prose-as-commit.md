# Outline-first authoring, and prose as its own commit (2026-08-26)

Every prior Benjamin file TRANSLATED existing prose into DMML — the
essay, or the paper's own draft — which made the work checkable against
an external ground truth. This pilot inverts that: there is no prior
prose for Section 5's flagged, unanswered question ("do self-declared
predicates converge into fixed convention across independent authors?
cannot be answered without evidence... that does not yet exist"). So
`dmml/examples/paper_predicate_convergence.rs` authors the argument's
dependency graph first — open question → natural experiment → empirical
data → confound → tentative answer → prose — and only the *last* commit
produces prose, as its own fact, consuming the answer that licenses it.
Less checkable than translation (there's no ground truth for a genuinely
novel argument), but the dependency structure itself is real and
checked.

The "empirical data" isn't asserted, it's computed: `count_declared_
predicates()` does real `std::fs` reads over every file in `dmml/
examples/`, counting actual `declare attribute <name>` occurrences at
runtime. Re-running after more examples land changes the numbers and
the produced prose changes with them — confirmed directly this session
(271 declarations, 24 files, 84 distinct predicates, `claim` × 91 —
these were computed, not guessed, and the file panics if a later
`assert!` catches the prose text *not* containing the freshly-computed
number). `counterClaim` and `distanceStrategy` are checked (Check 0) as
actually present in both genuinely independent-author files
(`benjamin_rival_reading.rs`, `benjamin_second_reader.rs`) before the
argument leaning on that fact gets built at all.

The argument itself is deliberately narrower than the open question as
posed: `claim` converging is weak evidence (any author models an
assertion with an ordinary English word regardless of convention
pressure); `counterClaim` converging is sharper (a task-specific
coinage, absent from ordinary vocabulary, that two uncoordinated authors
both reached for in the same role). The confound commit says this out
loud rather than smoothing past it: most of the corpus's raw convergence
reflects one continuous author's own consistency over time, not
independent multi-author agreement — only the two dispatched files are
real independent-author evidence. The tentative-answer commit narrows
accordingly: what's shown is convergence *under dispatch conditions*
(shared context, an existing graph in view to cite from), not
spontaneous convergence from a blank slate. The stronger claim DRAFT.md
originally flagged stays open — this pilot doesn't oversell what six
data points support.

Hit the same "cite-and-spend" interpreter behavior from Section VI
again, in a new spot: Check 2 originally queried `paper/section5_data`/
`claim` inside the FULL combined six-commit log, right after `confound`
consumes and retracts that exact key downstream — got `None` even
though the fact is real. Fixed the same way as every time before:
isolated re-materialization of the empirical-data commit alone
(`Materialized::from_identified_commits(&[empirical_data.clone()])`),
not a query into the log past the point something cites it. Worth
noting this is now the second file in a row (after several correctly
written from the start) where this exact mistake got made fresh before
being caught — it's not yet fully internalized as a reflex, still
requires the honest-check discipline to actually catch it, not
memorization.

All 4 checks pass, plus Check 0's independent-file sanity check. The
deliverable is the last `println!`: the actual materialized prose
paragraph, produced by walking the graph's `current_value`, not
composed separately and cited afterward.

Not yet done, and not requested for this file specifically: folding
this pilot's materialized prose into `papers/desiring-production-
ontology/DRAFT.md` Section 5 itself (replacing or supplementing "this
paper declines to answer it here"). Natural next step, not yet taken.
