# Datalog as the commit validator -- a real spike, real corpus

Jason's prompt: looking at tonight's checkpointed commits and finding them
too prose-heavy, then asking directly whether written-world's hand-rolled
guard/state-machine interpreter (`engine::graph::apply_commit`,
`machine.rs`) should just be a real Datalog engine instead. This is that
spike -- run against the actual 311-entry corpus from tonight's
sensemaker/Child swarm, not synthetic data.

## What was built

`dmml-agent-nucleus/spikes/datalog_validator.py`: a real, small, stdlib-only
Datalog engine (ground facts, Horn-clause rules with variable unification,
manually-stratified negation -- one fixpoint pass for recursive positive
rules, then a second pass for non-recursive rules that may negate them).
Two limitations named directly in the file rather than hidden: naive
(non-incremental) evaluation, and stratification that's manual rather than
automatically checked.

It extracts only the STRUCTURAL envelope every commit already carries --
`kind`, `consumes`, `responds_to` -- never the prose in `object_text`.
Extracting facts out of narrative text is a real, separate, much harder
problem this spike doesn't attempt.

## Real results, from the actual corpus

```
loaded 239 parsed real commits (72 raw/unparsed, skipped)
ground facts:  commit=239  consumes=309  respondsTo=50
derived:       restsOn (transitive citation edges) = 8393
               root commits (cite nothing) = 22
               uncited leaves (nothing cites or responds to them) = 80
               legalAccept = 0  illegalAccept = 0
               deepest single commit's total lineage = 105
```

Independently verified against the raw JSON: zero `accepts` commits exist
anywhere in the corpus (`kinds seen: {replies, critiques, raises,
repairs}`) -- so `legalAccept=0`/`illegalAccept=0` isn't an engine bug,
it's an honest, previously-unnoticed fact about tonight's run: the
raises -> replies -> accepts petition chain modeled at the DMML grammar
level has never actually closed once, across two full swarm runs.

Also newly visible, and not computed by anything earlier tonight: roughly
a third of all parsed commits (80/239) are uncited leaves -- nobody built
on them and they don't respond to anything either. And the citation graph
is far denser than the raw edge count (309) suggests once you take the
transitive closure (8393 restsOn edges, one commit with 105 total
ancestors) -- confirming there IS a real, substantial structural graph
under the prose, even though no petition has ever resolved.

## Verdict on "just use a Datalog interpreter as the validator"

Genuinely the right direction, not a novelty -- this is the same shape as
Berkeley's Bloom/Dedalus language (Datalog specifically to get the CALM
theorem's coordination-free guarantee in practice). Concretely, for
written-world:

- The hand-rolled Rust guard/state-machine code (`machine.rs`) is already,
  structurally, a bespoke mini Datalog engine. Replacing it with a real one
  (`ascent`/`crepe` for an embedded Rust engine, keeping semantics native
  rather than shelling out) would turn "a rule" into literal data instead
  of interpreter code -- making the repo's own DMML-first / A/C razor
  (SPEC.md #18) enforceable rather than just a discipline to remember.
- It does NOT resolve the open reflexivity question from earlier tonight:
  if two territories are allowed to commit conflicting rules for the same
  predicate, Datalog gives you a precise failure mode (the union may not
  stratify) rather than an answer to whose rule set governs. That's a real
  improvement (detectable vs. silently ambiguous) but not a solved problem.
- It does not make the prose more structured. The envelope (kind, consumes,
  responds_to) is exactly what a Datalog validator can check today, and
  this spike's real numbers show that envelope is not trivial (8393
  derived edges) -- but object_text stays exactly as narrative and
  unparsed as it already was. That's a distinct, harder problem: making
  the CONTENT of a claim fact-shaped, not just its citations.

## Immediate, real, unresolved fact this spike surfaced

Zero petitions have ever been accepted across the entire written-world
worldbuilding corpus. Worth deciding whether that's a swarm-instruction gap
(nobody's ever been told a petition needs a closing move) or a genuine,
honest feature of how this swarm builds -- proliferation without
resolution, which is exactly the "post-dialectics" instinct from earlier
tonight, just now visible as a hard structural number instead of a vibe.

## Resolved (Jason, same session): it's an instruction gap, and that's fine

It's a gap -- no persona has ever been told a raises/critiques chain can or
should close via `accepts` -- but not a bug worth fixing. Explicit call:
**for this worldbuilding swarm, expressivity is the goal, not closure.**
Zero `accepts` isn't the corpus failing to converge; it's the corpus doing
exactly what it was actually asked to do (proliferate, leave gaps, never
resolve). Don't add `accepts`-seeking instructions to any persona on the
strength of this finding alone -- if a real reason to want closed petitions
shows up later (e.g. an actual player-facing petition flow, not
worldbuilding), revisit then, as a deliberate choice, not a reflexive fix
for a number that looked like an error but wasn't one.
