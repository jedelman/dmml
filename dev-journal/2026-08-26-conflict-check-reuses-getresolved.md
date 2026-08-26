# The conflict check needs no new primitive — it's `getResolved` (2026-08-26)

Jason's instinct on the last open item in the live-deployment thread:
"conflict check probably doesn't need to be a primitive if we have an
existing general repo traversal/query primitive." Checked rather than
assumed, and it's not just directionally right — the exact mechanism
already exists, live, built for a different reason.

**First checked the obvious candidate and it doesn't fit, by its own
admission.** `dmml-runtime`'s `WorldGraph::consume_state`/`is_retracted`
(`dmml-runtime/src/graph.rs`) look like they should answer "has this
fact already been retracted" — but their own doc comment rules this out
directly: they only track whole-node currency for a `ConsumeRef::Strong`
reference. A `FactRef` entry (the `(commit, subject, predicate[,
object])` shape the conflict check actually needs) is *deliberately*
never added to that bookkeeping, because — in the comment's own words —
"this in-memory graph has no notion of which specific commit produced
which triple," unlike "`appview`'s URI-indexed commit log." The comment
names its own limits and points at exactly who doesn't have them.

**`appview` turns out to already do exactly this job, live.**
`org.jason-edelman.writtenworld.getResolved` (`appview/src/main.rs`) is
a real, deployed, native (non-wasm32) service — indexes every commit to
the collection across every repo via Jetstream (`bluesky-social/
jetstream-legacy`, a real deployed relay, not written-world's own
infrastructure and not a spike), with a `Resolver::resolve` walk that
already computes, per `FactRef`, whether the cited `(commit, subject,
predicate[, object])` is still current or gets excluded as "retracted
or structurally invalid." That's the entire conflict check, already
built, for a completely different original purpose (resolving a
player's world view across cross-repo references).

So the "Substrate side" of this work shrinks again, same shape as the
identity-binding thread earlier today: the checkpointing client doesn't
need a new query designed against atproto's retraction history — it
calls the existing `getResolved` XRPC endpoint against the
`(subject, predicate)` key its pending commit is about to consume, and
treats a reported retraction from an unknown commit as the conflict
signal.

**One real caveat named, not smoothed over**: `getResolved`'s index is
Jetstream-driven — an eventually-consistent read model, not a
synchronous check at the instant of write. There's a genuine
staleness/TOCTOU window between resolving and the checkpoint commit
actually landing. Didn't try to resolve this by asserting it's fine;
named it as the one remaining real (and now much smaller) question,
with a note that even a raced checkpoint just becomes one more
`disputes`-flagged case rather than silent loss, since nothing else in
this design depends on the conflict check being infallible.

Updated `ARCHITECTURE.md`: added the `getResolved` finding as its own
paragraph in the "Live deployment shape" section, right after the
detection/three-way-merge-base paragraph it directly answers, and
replaced the "Open design work" bullet that previously described
"query shape design work" with the narrower staleness question that's
actually left.
