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

## Description, not just naming — and what happens when there isn't one

Jason, after seeing the first version's bare `- Tamsin`: "is there
anything more to say about Tamsin? who is she, what does she look like?
or are those facts not minted?" Real answer at the time: not minted —
the original fixture asserted only her `name`/`location`/type, nothing
else. Fixed on both sides: `world.dmml` now gives `npc/smith` (and
`npc/herbalist`) the same `role`/`state`/`worksAt` facts
`template-compose-fresh-world-demo` already proved renderable, and
`browse` now runs `eligibleTemplates`/`renderTemplateWith` over every
co-located subject — reusing `DMML.TemplateBank` exactly as that other
demo does, no new rendering mechanism.

This catalog is deliberately narrower than that demo's, though — it
covers an active oresmith and an active herbalist, but not an
apprentice-in-training — so `npc/apprentice` ("Bram," `state/training`,
also co-located in `room/forge`) has no eligible template. That's a
real, disclosed content gap, not a bug, and the browser doesn't
silently say nothing about him: it emits SPEC.md §19.4's governed
catalog-expansion pipeline's own first stage for real — a structured
`requests`/`about` fact, in the same closed-vocabulary, effect-as-fact
shape §19.4 already designed (a gap declares a need, it never contains
free text). Only stage 1 (gap detection → structured request) is
exercised here; offline generation, the review gate, and the approved-
citation step are still real, separate, unbuilt work — nothing
consumes the emitted request yet. The emitted commit is real,
independently-verified DMML, not just plausible-looking text: piping it
through `validate-commit` exits 0.

## Real output

```
the smithy
==========

Also here:
  - Bram (no description on file)
  - Tamsin works the forge at the Ninefathom seam, master of the seam.

-- content gap(s) logged as real, unconsumed DMML requests:
commit contentgap1
  declare relation requests
  declare relation about

  request/npcgap1 `requests` type/descriptiontemplate
  request/npcgap1 `about` npc/apprentice

You can:
  * greet
```

`npc/smith` ("Tamsin") is co-located in `room/forge`, has an eligible
template (`state/active`, `role/oresmith`), and is rendered as a real
sentence resolving through `worksAt`/`role` the same way that demo's
fixture does. `npc/apprentice` ("Bram") is co-located but has no
eligible template in this narrower catalog, so he gets the fallback
line AND a logged gap request. `npc/herbalist` ("Onn"), located in
`forest/oldroot`, is correctly excluded from "also here" entirely —
she's not co-located, so she's neither described nor gapped.

`greet()`'s guard holds (`player/one` is in `room/forge`); `descend()`'s
guard does not (`player/one` holds no `tool/lantern`) — so only `greet`
is offered, proving guard evaluation, not a hardcoded action list, is
what's actually gating the menu.

Rendered as a CYOA-style page on purpose — Jason: "it may be that we
don't need a chat surface at all — we can format this just like an old
school choose your own adventure novel." One deterministic pass over a
materialized snapshot, zero generation, zero LLM calls, same discipline
as every other demo `written-world/SPEC.md` §19.4/§19.5 records this
session. Open question Jason flagged and this demo doesn't yet
address: if a world surfaces many available actions at once, they may
need to be embedded into the description text itself rather than
listed separately — not exercised here since this fixture only ever has
one legal action regardless of how many subjects are on stage.

## Real hyphen bug, hit again

Generating the gap-request commit's identifiers as
`content-gap-1`/`npc-gap-1` failed to parse (`validate-commit` REJECTED
with `unexpected "-g"`) — the same "hyphens are invalid inside a DMML
node-ref/identifier segment" constraint this session already hit twice
authoring `.dmml` fixtures by hand (`metal-object`, `early-days`), now
hit a third time by *generated* content. Fixed the same way: no hyphens
in any generated segment (`contentgap1`, `npcgap1`,
`descriptiontemplate`).
