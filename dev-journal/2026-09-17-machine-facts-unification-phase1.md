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
