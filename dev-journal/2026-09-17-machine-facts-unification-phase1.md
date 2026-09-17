# 2026-09-17 — machines-as-facts, phase 1: a real, tested encode/decode pair

Jason's framing: "I'd like machines to have the same guards as facts —
desiring machines, remember?" Real question underneath: does unifying
machines and facts break closure? Answer, worked out precisely rather
than asserted: the shallow type-closure question is trivial (Haskell
sum types are closed by construction — add a constructor, done), but
that's not real unification, it just moves the two-kinds-of-thing split
from two Haskell types into one. The real version — a machine's states
and transitions expressed entirely as facts, retractable/citable
through the exact same provenance machinery any other fact already
has — is a genuine, bigger redesign, and this session only attempted
phase 1 of it: prove the encoding itself is sound before touching
`DMML.Guard`/`DMML.Fire`'s actual dispatch.

## The one hard constraint the design had to fit inside

Confirmed by actually parsing real content (`.claude/skills/dmml-authoring`):
a single commit can never assert the same (subject, predicate) key
twice, even with different objects — the second occurrence silently
overwrites the first. A machine has MANY states, MANY transitions, MANY
guards per transition — genuinely multi-valued relations. Not a problem
requiring a linked-list encoding: DMML already has a first-class
concept for exactly this shape (multiple LIVE ALTERNATIVES for one key,
the same thing `DMML.Fire.FireRetractAmbiguous` already reasons about)
so long as each element lands in a SEPARATE commit. `DMML.MachineFacts`
doesn't emit commits yet (real, separate, later work) — it just emits
one `hasState`/`hasTransition`/`hasGuard`/`hasEffect` fact per element
and leaves "these need separate commits" as the constraint on whatever
renders this to Surface text next.

## `DMML.MachineFacts` — encode/decode, `[FactStmt]` only, no snapshot

`encodeMachine :: MachineStmt -> [FactStmt]` / `decodeMachine ::
NodeRef -> [FactStmt] -> Either DecodeError MachineStmt`. Every
sub-structure (transition, guard, hop, effect) gets its own minted node
under the machine's own node (open-world minting, same mechanism an
ordinary assert already uses). Order is preserved where it's
semantically real (a guard or retract's hop sequence — explicit numeric
`hopIndex` fact) and NOT preserved where it never was (state order,
transition order, which guard comes first, which effect comes first —
none of that changes what fires or what a guard checks).

## Two real bugs the round-trip test caught before this shipped

1. **Off-by-nothing type error, real one**: the first term-encoding
   reused the lossy SURFACE-TEXT spelling (`self`, `$param`, bare node
   text) instead of an explicit tag. That's fine for
   `DMML.Fire.renderPatternTerm` (which has to match what the real
   parser can re-read) but wrong here, because a single-segment
   `TermNode` is lexically indistinguishable from a `TermVar` once
   spelled as bare text — exactly the ambiguity `dmml-authoring`
   already documents for the real grammar. Concretely: the state-sugar
   desugaring (`assert unlocked` → `EffectValueTerm (TermNode
   "unlocked")`, per `DMML.Ast.Effect`'s own doc comment) produces
   exactly this shape for real, and the lossy encoding silently turned
   it into a `TermVar` on decode. Fixed by tagging explicitly
   (`"node:" <> n` / `"var:" <> v`) instead of reusing surface spelling
   — this module never goes through the real parser, so it owes real
   grammar text nothing.
2. **Test bug, not an encoding bug**: `transitionParams` order isn't
   preserved (never semantically real — params bind by name via a
   `Map`, `EvalContext`, never by position), but the round-trip test's
   first version compared raw `==` including that list's order. Fixed
   by sorting params before comparing, not by changing the encoding.

Both caught by actually running a real round-trip test
(`examples/jev-driver-demo/machine-facts-selftest.hs`) against a
machine exercising every branch — multiple states, a negated multi-hop
guard, an assert, a retract with hops AND a value, and a spawn effect —
not by inspection.

## What phase 1 deliberately does NOT do

`DMML.MachineFacts` knows nothing about `DMML.Materialize.WorldSnapshot`,
multi-commit assertion, or firing. `DMML.Guard.mayFire`/`DMML.Fire.fireTransition`
still read a machine's structure from an already-parsed `MachineStmt`
record, not from live facts in a snapshot. Wiring that — either
reconstructing a `MachineStmt`-shaped view from scattered facts on
every `mayFire` call, or rewriting the engine to walk the fact graph
directly and never materialize `MachineStmt` at all — is real, larger,
separate work. Also not attempted: rendering the encoded facts to real
Surface commit text (grouped into separate commits per the one hard
constraint above) — this phase proves the STRUCTURE is soundly
representable as facts, nothing about the render pipeline yet.

## Phase 2, same day: `decodeMachineFromSnapshot` + `fireTransitionFromFacts`

Before writing a line of this phase: checked whether this had prior
art rather than assuming it didn't. `Materialize.hs`'s own header
comment cites `written-world/dev-journal/2026-09-02-machines-as-facts-
generic-guard-evaluator.md` — that exact path doesn't exist in the
current `written-world` checkout (reorganized since, most likely), but
`written-world`'s own `README.md` (which its `CLAUDE.md` explicitly
names as the real architecture doc) does: `engine::machine` already
runs a machine-as-graph-node model in production, on Oxigraph, with
the identical core bet this module makes, verbatim from that crate's
own module doc: "nothing here is a different *kind* of fact from
anything in `graph.rs` — a requirement or effect is just another node
with its own triples, read back the same way a room or item is."

Real difference worth recording: `written-world`'s `Requirement`/
`Effect` are three closed, hardcoded variants apiece — no general
multi-hop pattern guard, so DMML's guard language is strictly richer
and this module's encoding is correspondingly bigger. And
`written-world` decodes ONE requirement/effect node at a time, on
demand, as a live evaluation walks the graph (`requirement_met` takes
one `Requirement` and evaluates it directly) — it never reconstructs a
whole "Machine" struct. `decodeMachineFromSnapshot` deliberately takes
the other real option: decode the WHOLE machine once, then hand it to
`DMML.Guard`/`DMML.Fire`'s existing, already-tested functions
unchanged. Lower risk (zero changes to load-bearing dispatch code) at
the cost of being coarser-grained than a true piece-by-piece live
evaluator — a real tradeoff, not a free lunch, and worth revisiting
once the actual cost of decoding a whole machine per `mayFire` call is
measured against something that matters.

**`snapshotToFacts`**: flattens a real `WorldSnapshot`'s
`Map (Text, Text) Alternatives` into ordinary `FactStmt`s — every live
alternative becomes its own fact, so a key with several live
alternatives (exactly `encodeMachine`'s multi-valued `hasState`/
`hasTransition`/`hasGuard`/`hasEffect` shape) becomes exactly that many
facts, matching what `decodeMachine` already expected from the phase-1
test.

**`fireTransitionFromFacts`**: decodes the acting machine from `snap`
via `decodeMachineFromSnapshot`, then delegates straight to the
ordinary, UNCHANGED `fireTransition`. New `FireMachineDecodeError`
`FireError` case for when decode itself fails.

## Two real bugs this phase's own test caught — recorded because both are informative, not just "it works now"

1. **First version forgot the machine's own initial `state` fact.**
   `fireTransitionFromFacts` refused with `FireBlocked`. Before
   assuming a decode bug, fired the ORIGINAL, non-decoded `furnace`
   against the identical (incomplete) snapshot first — it refused
   identically, which proved the bug was in the test's snapshot setup,
   not in decode. The actual cause: exactly the landmine
   `.claude/skills/dmml-authoring` already documents — "A freshly
   minted machine needs its own initial `state` fact... asserted in
   the SAME commit that equips it." `encodeMachine` correctly does NOT
   emit a `state` fact (a machine's current state is ordinary, mutable
   WORLD data, not part of its structural definition — the same
   distinction the skill draws), so the test had to assert one itself,
   same as any real caller would.
2. **Second version then hit `FireRetractNoProvenance`.** The
   transition's `retract self \`state\`` effect needs a real
   `StrongRef` to cite, which plain `applyCommit`/`applyCommits` never
   provide (`DMML.Fire`'s own documented behavior, not new). Fixed by
   switching the test to `applyIdentifiedCommits` with a synthetic-but-
   real `StrongRef` per fact (same contract `DMML.LocalIdentity.localFileRef`
   satisfies for a real file, minted deterministically here since the
   test has no file to hash).

Once both were fixed: firing the fact-sourced machine produces the
BYTE-IDENTICAL rendered commit to firing the hand-authored machine
directly, checked by actual string equality, not by inspection. That's
the real closure claim proven, not asserted.

## What's still not done

`Guard.hs`/`Fire.hs`'s own source is completely unchanged — every new
capability lives in the new functions (`decodeMachineFromSnapshot`,
`fireTransitionFromFacts`), additively. Not attempted: rendering
`encodeMachine`'s output to real Surface commit text (grouped into
separate commits, per phase 1's own constraint) — everything in phase
2's test builds `CommitStmt`/`IdentifiedCommit` values directly in
Haskell, never through `DMML.Surface`'s parser (unavailable in this
sandbox regardless, see the PR's own testing notes). Also not
attempted: a piece-by-piece live evaluator in `written-world`'s own
style, noted above as a real, deliberate scope choice, not an
oversight.

## Phase 3, same day: wiring it into the Jev driver's actual problem

Jason's ask, directly: "how does this affect our Jev evaluation?" then
"wire it up." Two concrete closures against the driver's own disclosed
gaps (`examples/jev-driver-demo/README.md`'s "Known, disclosed scope
limits" section):

1. **`DMML.Fire.renderFiredCommits`**: a spawned machine now renders as
   real DMML Surface COMMITS (`DMML.MachineFacts.encodeMachine`'s
   output, one commit per fact — the one hard constraint again) instead
   of only a Surface `machine` block needing a parser round-trip.
   Applied as `--world` files, a spawned machine is immediately
   fireable (`fireTransitionFromFacts`) and immediately visible to
   discovery (below) with zero re-parse. `renderFiredMachine`'s Surface
   block is kept too, for inspection — not replaced, `renderFiredCommits`
   is additive.
2. **`DMML.MachineFacts.candidateTransitions`** (+ `machineNodesInSnapshot`):
   real automatic candidate enumeration — every `(machine, transition,
   params)` triple discoverable by querying a snapshot's own
   `hasState`/`hasTransition` facts, no hand-authored `candidates.json`
   entry needed. New `list-candidates` CLI exposes it. Real, disclosed
   limit stated plainly in the driver's README: this only sees
   FACT-NATIVE machines — a spawned one, or one deliberately authored
   via `encodeMachine` — never a hand-authored Surface-text machine
   (`cascade-demo`'s `furnace`/`anvil` stay invisible to it) unless that
   too gets encoded.

One real bug in the first draft of `machineNodesInSnapshot`, caught
before compiling rather than at runtime this time: an over-engineered
attempt at deduplication via a self-referential list comprehension with
a repeated, nonsensical guard clause. Simplified to `nub` over the
`NodeRef`s directly — `NodeRef` already derives `Eq`, nothing clever
needed.

Both new functions compiled and actually run for real
(`spawn-facts-pipeline-selftest.hs`, `candidate-discovery-selftest.hs`)
— the former proves a machine can fire a transition that never existed
as anything but facts a PRIOR firing produced, closing the loop all the
way from "machines and facts unified" (phases 1-2) through to "a Jev
driver loop can discover and fire what a prior round's spawn produced,
automatically" (this phase). `app/ListCandidates.hs` and
`app/FireTransition.hs`'s updated output path remain UNCOMPILED, same
disclosed reason as every other CLI touched this session (no megaparsec
in this sandbox) — their own underlying logic is what's verified, not
the thin CLI wrapper.

Not done: the Python driver (`driver.py`) itself is not rewired to call
`list-candidates` instead of reading `candidates.json` by hand. The
capability exists and is documented; the driver's own candidate-loading
code is untouched.
