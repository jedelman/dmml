# Staleness closed: non-issue generally, content-level fix for gaming (2026-08-26)

Closes the one remaining question from the conflict-check finding
earlier today. Jason's read: `getResolved`'s Jetstream-driven staleness
window doesn't matter outside gaming applications, and gaming
applications have their own fix — specialized DMML TTL triples — not a
change to the conflict-check design itself.

Both halves hold up:

**General case**: most DMML applications, including this project's own
paper-authoring and critique work, have no tight interactive loop where
a few seconds of read-model lag is observable or consequential. A
checkpoint that resolves a moment stale just disputes a moment later —
already the accepted outcome from the conflict-check design regardless
of *why* a conflict surfaced late, so staleness adds no new failure
mode, just a slightly later trigger of one already designed for.

**Gaming case**: the one real place staleness could bite (two players
racing to the same item, a race experienced directly and immediately)
doesn't need a stronger infrastructure guarantee either. It's the exact
same move Section 3 of the desiring-production paper already makes for
lack/desire — DMML's grammar has no opinion about tight-consistency
needs, so an author who needs one writes it as ordinary, self-declared
content (a TTL-shaped predicate on the specific facts that actually
need a tight bound) rather than the grammar or the conflict-check
infrastructure growing a new primitive to solve a domain-specific
problem most DMML worlds don't have. Consistent with `AUTHORING.md`'s
own standing guidance: coin new vocabulary when the existing primitives
would misstate what's needed, don't reach for infrastructure when
content will do.

Nothing designed here — TTL triples are named as a real direction for
whoever builds a real-time game on this substrate, not specified.
Updated `ARCHITECTURE.md`'s "Live deployment shape" section with this
resolution immediately after the staleness caveat it closes, so the
thread reads as actually finished rather than left hanging on an open
caveat.

This closes every item that was open in the live-deployment design
thread as of this session: client split, conflict detection and
resolution, cross-substrate identity binding, Android's and CLI's auth
paths, the conflict check's mechanism, and now its one remaining
caveat. What's left in `ARCHITECTURE.md`'s "Open design work" is
implementation from here, not further design.
