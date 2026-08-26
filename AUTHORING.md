# Authoring guidance

Practical norms for anyone — human or dispatched agent — declaring new
DMML vocabulary in a shared world. Not protocol rules; `declare` is
deliberately open, closed only until extended, and nothing here changes
that. These are usage guidelines for the judgment call every author
already has to make at the moment they write `declare attribute` or
`declare relation`.

## Check before you coin

An open vocabulary makes diffusion, dispersal, and dilution real
practical risks: if every author reaches for a new predicate name for a
concept a prior author already declared, the graph accumulates near-
duplicate vocabulary (`claim`, `assertion`, `statement`, `finding` all
meaning roughly the same thing across different files) instead of a
shared, citable vocabulary that actually accrues meaning through reuse.
This is not a defect in the protocol — `declare`'s openness is what
lets a genuinely new concept get named at all — but it is a real cost if
nobody attends to it, and nothing in the grammar itself prevents it.

Before declaring a new predicate:

1. **Look at what's already declared in the world you're extending.**
   `grep -rhoE 'declare attribute [a-zA-Z]+' <examples-dir>` (or the
   equivalent for the actual target graph, not just this repo's example
   files) is cheap and real — do it before coining, not after.
2. **Reuse an existing predicate if its meaning genuinely fits.** Not
   "close enough" — the same claim, not a related one. Reusing `claim`
   for a genuinely different kind of assertion just because the word is
   already in the vocabulary is its own failure mode (see below); reuse
   only when the meaning is actually the same.
3. **Coin a new predicate when the existing vocabulary would misstate
   what you mean.** A new, more specific name is better than forcing an
   ill-fitting reuse. `counterClaim` earns its place next to `claim`
   because it names a distinct role (disputing a prior claim), not a
   synonym for it.

## Generic reuse is weak evidence of anything; specific reuse is strong

This distinction was confirmed empirically, not asserted:
`dmml/examples/paper_predicate_convergence.rs` found that ordinary
English words (`claim`) converge across authors constantly, for reasons
that have nothing to do with shared convention — any author modeling an
assertion reaches for the word "claim" whether or not anyone else's
vocabulary influenced them. Convergence on a **task-specific coinage**
(`counterClaim`, `distanceStrategy`) between authors who never
coordinated is a much sharper signal that a real, shared term has taken
hold. When you're deciding whether "someone already named this," weight
that decision the same way: a shared ordinary word tells you little; a
shared coined term, especially one with no obvious one-word alternative,
tells you the vocabulary is actually converging.

## This is judgment, not enforcement

There is no mechanism (and this document does not propose one) that
blocks a bad predicate name or forces reuse. The check above costs one
`grep` and a moment's thought before writing `declare`; the guideline is
that it's worth doing, not that anything requires it. A dispatched
agent's briefing should include a pointer here (see
`.claude/agents/dispatch-methodology.md`) the same way it includes the
DMML syntax itself — vocabulary discipline is part of the task, not an
afterthought to clean up later.
