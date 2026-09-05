# world-browser-demo

Direct answer to Jason's design question: "Build the linter, then build
a 'browser' for the world... The browser most likely can be built
entirely in dmml, and i'm wondering if valid actions can also be
surfaced in dmml or whether we need new Haskell code for that."

```sh
world-browser-demo examples/world-browser-demo/world.dmml examples/world-browser-demo/actions.dmml
```

## What's DMML content vs. what needed new Haskell

Everything the player perceives and can do is real, ordinary DMML
content — no bespoke Haskell types for "player," "room," or "action":

- **Embedding**: `player/one` is just a subject with a `location` fact
  (`room/forge`), same predicate any other node uses.
- **What's on stage**: co-location is a plain relational query — every
  other subject whose own `location` fact resolves to the same room.
  This is a deliberate v1 simplification, disclosed not hidden: it is
  NOT a full sense-machine firing pass (SPEC.md §19.2's richer
  perception-gating is real, separate future work) — here, "on stage"
  just means "everything asserted as co-located," which is enough to
  prove the browser's basic shape.
- **What's actionable**: `machine/miningactions` is an ordinary machine
  declaration, `player/one`'s `equips` fact names it, and each
  transition's guard is exactly the same `DMML.Guard` machinery every
  other demo this session has used — `greet()` guards on
  `self \`location\` room/forge`, `descend()` on
  `self \`holds\` tool/lantern` (a fact nothing in this world asserts,
  so it's correctly never offered).

**The one piece of new Haskell**: `DMML.Guard.availableTransitions`, a
small generic enumeration over "every declared machine's every
transition, which ones currently pass right now" — reusing `mayFire`
unchanged, called in a loop across a machine map instead of by one
ident. This is not a new interpreter capability so much as the same
"new evaluation mode" §19.2 already named as necessary for sense-
machines ("a forward pass reusing eval_guards/resolve_transition
unchanged, just called in a loop instead of by ident") — applied here
to surfacing actions instead of materializing perceived facts. DMML
itself has no "for all machines, for all transitions" construct, only
"is this one transition legal" — so yes, new Haskell was needed, but it
is exactly this one small, generic, already-precedented shape, not a
bespoke "actions system."

## Real output

```
the smithy
==========

Also here:
  - Tamsin

You can:
  * greet
```

`npc/smith` ("Tamsin") is co-located in `room/forge` and correctly
shown; `npc/herbalist` ("Onn"), located in `forest/oldroot`, is
correctly excluded. `greet()`'s guard holds (`player/one` is in
`room/forge`); `descend()`'s guard does not (`player/one` holds no
`tool/lantern`) — so only `greet` is offered, proving guard evaluation,
not a hardcoded action list, is what's actually gating the menu.

Rendered as a CYOA-style page on purpose — Jason: "it may be that we
don't need a chat surface at all — we can format this just like an old
school choose your own adventure novel." One deterministic pass over a
materialized snapshot, zero generation, zero LLM calls, same discipline
as every other demo `written-world/SPEC.md` §19.4 records this session.
Open question Jason flagged and this demo doesn't yet address: if a
world surfaces many available actions at once, they may need to be
embedded into the description text itself rather than listed
separately — not exercised here since this fixture only ever has one.
