# Descoping sync/substrate; refocusing on DMML as a clean local language

Closes out the git-broker / Radicle / Tangled thread from earlier today
(`2026-08-31-git-radicle-broker-substrate.md`) with a scoping decision,
not a technical answer to it.

## The decision

Jason: return to the original framing of DMML as a base-layer DSL other
projects (written-world foremost) build on, and treat sync/distribution
as a genuinely separate toolchain concern to solve later -- not
something that has to be settled before DMML itself is clean. For now,
persistence can be anything trivial and centralized: plain git, or even
flat S3 objects. No multi-writer conflict resolution needed yet, because
there's no second writer yet.

## Why this isn't a retreat from today's earlier research

It's the direct consequence of a fact already true in the code, not a
new constraint being imposed on it: `engine::graph::WorldGraph::
apply_commit` never inspects how a commit arrived -- it reads commit
content and compares opaque `{uri, cid: String}` pairs. Git-vs-S3-vs-
Radicle-vs-per-player-broker was always a question about what sits
*around* `dmml`/`dmml-runtime`, never a question the language or
interpreter themselves had an answer bound up in. So today's SSH/
Tangled/Radicle research isn't wasted -- it's a real, still-valid answer
to a question that turns out not to gate anything right now. Parked, not
discarded.

## What "focus on the language" means next

Not yet scoped in detail -- next session's real first task is either:

- Jason names specific known rough edges in `dmml/src` (grammar
  quirks, `machine.rs` interpreter warts, `identity.rs`'s atproto-
  specific half the `transient-conjuring-flask.md` extraction plan
  already flagged as not substrate-blind), or
- a first-pass read of the crate to report back what looks least clean,
  if nothing specific is already in mind.

Either way: the `Substrate` trait, the broker, and the git/Radicle/
Tangled evaluation all stay exactly where today's journal entries left
them -- real, recorded, not acted on in code -- until sync becomes the
actual next thing being built.
