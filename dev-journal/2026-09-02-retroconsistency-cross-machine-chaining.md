# Cross-machine chaining: the fixpoint over retroconsistency

Jason: "cross machine chaining is the way to go next. let's get that
working before we hit this with a swarm." Follows `2026-09-02-
retroconsistency.md` (the primitive) and `2026-09-02-retroconsistency-
gate.md` (gating one candidate against the whole tree) — this closes
the disclosed follow-up both of those left open: Jason's quarry
example, "someone quarried it, and the stone went somewhere — where?"

## Correcting the earlier framing before building the wrong thing

The prior dev-journal entry described the follow-up as: "run this
module again, recursively, on whatever machine (if any) governs each
freshly-implied node." That doesn't survive contact with how
governance actually works here. A freshly-minted node — an existential
hop's placeholder, invented on the spot because nothing else in the
pattern says which node satisfies it — has NO facts about it
whatsoever. Nothing has asserted it equips a machine, named it,
anything. There is no governance edge to discover for a node nobody
has said anything about yet, and there structurally can't be, by
construction, since only fact assertions ever create such edges.

**The real chaining case is a BOUND target** — a guard hop resolving to
a real, already-existing node (`self \`deliversTo\` warehouse/central`,
a literal multi-segment reference, not a variable). And per this
project's own convention throughout every real example built this
session, a node's governing machine is simply whichever `MachineStmt`
was declared with that same node as its own name — `Map.lookup
targetText machines` — not an `equips`/`trigger` lookup
(`DMML.Governance.findGoverningMachine`, which exists specifically to
arbitrate a DISPUTED multi-valued pair, a different concern entirely
from "which machine plainly governs this node").

## The mechanism

`fixpointRetroconsistency machines rootMachine rootTransition snapshot`:
a worklist algorithm, starting from one `(machine, transition)`. For
each item: run the existing single-step `retroconsistency`; if it
implies facts, apply them (through a real render + parse round-trip,
same discipline as everywhere else — every step of a chain is provably
real DMML the whole way through, not internal state that only becomes
real at the end), **gate the result against the full machine set**
(Jason's own earlier instruction — every step, not just the whole
chain once at the end, since a later step could break something an
earlier one already relied on), and for every implied fact whose
target matches a declared machine's own node, queue that machine's
every transition too. A visited `(machine, transition)` set makes this
provably terminating even against a real cycle (A's transition needs
B, B's needs A) — not assumed safe, tested (Scenario 4 below).

## Verified for real — the actual quarry example, plus the honesty checks

`app/RetroChainDemo.hs`, four scenarios:

1. **The real quarry chain**: `quarry/east`'s `extract` transition
   needs both `quarriedBy` (existential — gets a fresh actor) and
   `deliversTo warehouse/central` (bound — resolves to the real,
   already-named node). `warehouse/central` is a real, separately
   declared machine with its own unrelated precondition (`staffedBy` a
   clerk, needed before it can legitimately be `receiving`). One call
   surfaces BOTH gaps, in order — `warehouse/central`'s own precondition
   was never asked about directly, only discovered by chaining through
   the quarry's own implied `deliversTo` fact.
2. **Scope matters, proven**: the identical scenario with only
   `quarry/east` declared (no `warehouse/central` machine in the
   machine map at all) correctly stops after one step — chaining can
   only discover what's actually in scope, not conjure a machine that
   isn't there.
3. **The corrected framing, proven**: the forest-depletion example's
   fresh `harvestedBy` target — even with an unrelated machine present
   in scope — correctly chains no further. Confirms the "fresh nodes
   can't chain" conclusion is actually true of the real implementation,
   not just argued in a comment.
4. **Cycle safety, proven, not assumed**: two machines each requiring
   the other (`loop/a` needs `linkedTo loop/b`; `loop/b` needs
   `linkedTo loop/a`) terminates cleanly at 2 steps instead of looping
   forever.

## What's still open

- **Not wired into `retro-gate` or any real pipeline** — `fixpoint
  Retroconsistency` is a library function plus a demo; there's no CLI
  yet that takes a root transition and a world, runs the fixpoint, and
  emits the combined commit(s) for real use. Real, natural next step
  before "hitting this with a swarm," not done here.
- **"Every declared transition on a newly-reached machine" is a real,
  disclosed breadth choice** — there's no predicate/context to narrow
  which of a chained machine's transitions is "the relevant one," so
  all of them get checked. For a machine with many transitions, only
  some domain-relevant to the chain, this could surface real noise
  (facts implied for an unrelated transition just because the machine
  happened to get reached). Not a correctness problem — everything
  still gets gated — but a real usability question once this runs
  against a larger, denser machine set than this demo's two-machine
  example.
- **`ChainFailed` loses partial progress** — a chain that breaks
  partway through returns nothing usable, not the steps that DID
  resolve before the break. Matches `broker.sh`'s own all-or-nothing
  posture deliberately, but whether that's the right call for a long
  chain (vs. applying what resolved and reporting only the tail as
  blocked) is a real, undecided tradeoff, not settled here.
