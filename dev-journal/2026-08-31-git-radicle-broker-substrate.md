# Git-as-substrate, per-player brokers, and whether Radicle solves discovery

A design conversation, not yet acted on in code -- no `Substrate` trait
work landed from this, deliberately. Recorded because it's a real
candidate pivot away from the `transient-conjuring-flask.md` plan's
atproto/iroh framing, not a settled decision.

## The proposal (Jason's)

Since the project has already committed to data sovereignty, use git's
own primitives -- branch, add, commit, push, pull -- as the actual
persistence/sync substrate, instead of atproto or iroh. Concretely:

- Each player runs their own **broker**: a process, on their own server,
  that receives incoming commits from remote peers, runs an integrity
  check, and auto-merges on pass. It's open source; a player can modify
  their own broker's rules and accepts the consequences of doing so.
- `dmml` becomes purely the authoring DSL -- the thing that keeps
  content internally consistent (grammar, guards, the commit model).
  Git+broker becomes the persistence/authority layer. Jason's framing:
  these two concerns are orthogonal.
- atproto and iroh get downgraded from "the substrate" to "a peer
  discovery channel" -- i.e., how does peer B learn peer A's git remote
  exists at all, not how does the data move or get validated.

## Why the orthogonality claim actually holds, checked against the code

Not just asserted -- `engine::graph::WorldGraph::apply_commit` already
never inspects *how* a commit arrived. It reads commit content and
compares opaque `{uri, cid: String}` pairs it's handed. That's the same
boundary this proposal draws: authoring/coherence (dmml) genuinely has
zero opinion about persistence/authority (git+broker), because the
current code already doesn't let those concerns touch. A git-broker
`Substrate` impl would in fact be a *simpler* first concrete
implementation of the trait sketched in `transient-conjuring-flask.md`
than either atproto or iroh: the CID collapses to a single format (git's
own blob/commit SHA -- no more reconciling atproto's CIDv1/dag-cbor
against iroh's raw BLAKE3, which was the one real gap that plan had to
name up front), the admission gate is literally the broker's integrity
check, and the sovereignty root is each player owning their own remote
and its signing key.

## Correcting my own first read

I initially took "the broker" as a single CI process -- a centralizing
chokepoint, federation-with-one-gatekeeper. Wrong: it's N brokers, one
per player, each independently forkable. Two brokers can legitimately
reach different conclusions about the same incoming commit. That's not
a bug in the design, it's the same shape as the `mergeable`/`arbitrated`
self-declaration mechanism already named (but not yet built) in the
iroh spike's dev-journal entries -- git+broker just gives it real
mechanics (fork, branch, diff, merge, PR-shaped review) instead of a
bespoke conflict-resolution protocol that would have to be invented from
scratch.

## The real question this session actually researched: can you avoid running a server?

Jason's objection to the naive version of this: "you can't push to a
peer if it's not running." Requiring every player to stand up and keep
online their own always-on box is a real adoption cost the atproto/iroh
framing didn't have (a PDS host, or an iroh relay, already handles
uptime for you).

Checked against Radicle (radicle.xyz), a real, existing peer-to-peer
git collaboration stack, live-verified via web search rather than
recalled:

- **Radicle already separates "who has to be online" from "who
  replicates the data."** Every user runs a lightweight node, but
  availability comes from **seed nodes** -- always-on peers that
  gossip-discover repos and replicate them over git's own protocol. A
  push doesn't need the specific target peer online; it needs to reach
  *some* seed holding or wanting the repo, and the repo then stays
  available while the original peer is offline.
- **Public seed nodes already exist and are actively used** -- roughly
  600 nodes and 8000 repos seen weekly as of the most recent data
  found. So a player doesn't have to run their own server just to be
  reachable; they can rely on existing public seed infrastructure, the
  same way most atproto users don't self-host a PDS today.
- **This is a real, honest sovereignty tradeoff, not a free lunch**:
  "no server" really means "no server *for availability*" -- you're
  still trusting third-party seed infrastructure for replication.
  Reasonable, and probably a better default than mandating every player
  run their own always-on box, but it shouldn't get described as more
  self-sovereign than it is.
- **Seed nodes solve replication/discovery/availability. They do not
  do the broker's job.** Radicle's seeding policy controls *which
  repos* a seed stores, not content validation -- there's no
  integrity-check-then-merge semantics built into seeding itself. The
  per-player broker doesn't disappear; it becomes the thing each player
  runs (or points at) that watches their Radicle-hosted repo and
  actually does the admission-gate logic, while Radicle handles "can I
  even reach a copy of this repo" underneath it.
- **Real, existing prior art for exactly the broker shape described**:
  a `radicle-ci-broker` crate exists today, runs alongside a Radicle
  node, and triggers a CI run as soon as a new patch lands -- the
  "receive an incoming change, run a check, act automatically on the
  result" pattern this proposal wants, already built and named
  "broker" independently. Worth reading its actual source before
  designing DMML's own version from scratch; it may cover more of the
  admission-gate mechanics than assumed (webhook adapter, encrypted
  webhook settings keyed to permitted users, patch-event triggering).
  Not yet read -- next step, not done this session.

## What's still open, not resolved here

- Whether DMML's broker should be built as a Radicle CI-broker
  integration (reusing `radicle-ci-broker`'s event/webhook plumbing) or
  as a from-scratch process watching a plain git remote -- Radicle buys
  discovery/replication/patch-event-triggering for free, at the cost of
  depending on Radicle's own node software and network being reachable
  and maintained; a plain-git version depends on nothing but git itself
  but has to solve discovery some other way (this is the piece that
  gets deferred to "future work" either way for now).
- Whether this replaces the `transient-conjuring-flask.md` plan's
  atproto/iroh-as-substrate framing outright, or sits alongside it as a
  third `Substrate` candidate -- not decided. Given how much smaller
  the CID/identity story becomes under git, it's the strongest
  candidate for the first *concrete* `Substrate` impl once that trait
  actually gets designed, ahead of atproto or iroh.
- `ARCHITECTURE.md` and the `Substrate` trait itself still don't exist
  in code yet, on purpose -- per the original plan, that trait design
  is real work for the session that actually scaffolds `dmml-runtime`
  against a chosen first substrate, informed by whichever of these
  gets picked, not guessed ahead of that.
