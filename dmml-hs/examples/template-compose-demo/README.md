# template-compose-demo

Real, no-LLM proof of `SPEC.md` §19.4's "text as closed-set selection,
not open generation" design (`written-world#139`) — `DMML.TemplateBank`.

**Collapsed to reuse the real interpreter directly (2026-09-04, Jason:
"the ENTIRE existing grammar is ALREADY our field/domain... attributes
are just facts are just DMML").** The first version of this module
invented its own eligibility mechanism — a bespoke `[(Text, Value)]` tag
list plus a special-cased `templateType` field, checked by hand-written
Haskell matching logic. That was exactly the mistake this repo's own
"DMML first" rule warns against: reaching for a narrower, parallel
construct instead of checking what the real grammar already says. A
template's eligibility condition is not a new kind of thing — it's a
guard, literally `DMML.Ast.GuardClause`, evaluated by the same,
already-shipped `DMML.Guard.evalGuards` every machine transition uses.
"Bound to type" isn't special either: `guard self \`a\` type/metalobject`
is just another guard clause, no different in kind from a `condition`
check. This isn't a simplification traded for expressiveness — it's a
real gain: templates now get negation (`guard not ...`) and arbitrary
multi-hop patterns for free, because that's what `DMML.Guard` already
supports and the old bespoke tag list never could.

## A real bug found building this, worth keeping visible

The first attempt at this version used single-segment values
(`corroded`, `pristine`, `MetalObject`) and got everything matching
everything — a real, load-bearing grammar rule, not a bug in the
interpreter: `DMML.Surface.pPatternTerm` parses a bare **single-segment**
identifier in guard position as `TermVar` (an existential variable,
matches anything), and only a **multi-segment** reference (containing
`/`) as `TermNode` (a concrete match target). Guard text written by hand
needs multi-segment node values (`type/metalobject`, `state/corroded`)
to actually constrain anything — `world.dmml` and the catalog below both
reflect this now.

## The world and the catalog

`world.dmml` declares three entities:

- `npc/watcher` — `:: a type/metalobject`, `condition state/corroded`,
  `material stuff/metal`.
- `npc/keeper` — `:: a type/metalobject`, `condition state/pristine`,
  `material stuff/metal`.
- `creature/wraith` — `:: a type/spirit`, `condition state/corroded` —
  same attribute value as `npc/watcher`, deliberately a different type.

Each catalog template's eligibility condition (`app/
TemplateComposeDemo.hs`) is literal DMML guard text, parsed by the real
`DMML.Surface.parseMachineSurface` (wrapped in a throwaway single-
transition machine purely so the real parser has something to parse —
the machine is never fired, only its transition's real, parsed guard
list is kept):

```
[ Template "worn-corroded"
    "guard self `a` type/metalobject\nguard self `condition` state/corroded"
    "{subject} looks worn, its surface corroded with age."
, Template "gleaming-pristine"
    "guard self `a` type/metalobject\nguard self `condition` state/pristine"
    "{subject} gleams, freshly forged and untouched."
, Template "metal-generic"
    "guard self `a` type/metalobject\nguard self `material` stuff/metal"
    "{subject} is built of metal."
, Template "not-pristine-metal"
    "guard self `a` type/metalobject\nguard not self `condition` state/pristine"
    "{subject} has clearly seen use."
]
```

```sh
template-compose-demo examples/template-compose-demo/world.dmml
```

Real output:

```
=== npc/watcher ===
eligible templates: ["worn-corroded","metal-generic","not-pristine-metal"]
  -> npc/watcher looks worn, its surface corroded with age.
  -> npc/watcher is built of metal.
  -> npc/watcher has clearly seen use.

=== npc/keeper ===
eligible templates: ["gleaming-pristine","metal-generic"]
  -> npc/keeper gleams, freshly forged and untouched.
  -> npc/keeper is built of metal.

=== creature/wraith ===
eligible templates: []
```

Three things proven at once, all by the SAME mechanism
(`DMML.Guard.evalGuards`, unmodified):

1. **Mutual exclusion** — `npc/watcher` never gets `gleaming-pristine`;
   `npc/keeper` never gets `worn-corroded`.
2. **Negation composes for free** — `not-pristine-metal` correctly
   matches `watcher` (metal, not pristine) and correctly excludes
   `keeper` (metal, but pristine).
3. **Type-binding needs no special field** — `wraith` shares
   `condition = state/corroded` with `watcher` but gets zero eligible
   templates, because `guard self \`a\` type/metalobject` is just
   another clause in the same conjunction, evaluated the same way.

**What this doesn't do**: the governed catalog-expansion pipeline (gap-
detection guard → request effect → offline generation → review gate →
approved template) that would grow this catalog — still a fixed
Haskell list here, not sourced from that process. Also doesn't define
type SCHEMAS (which attributes are even legal to assert for a type in
the first place) — per the same conversation's Deleuze-framed
correction: compatibility between DIFFERENT attributes on the SAME
entity (the "burning + submerged" case) is a property of whichever
machines GRANT those attributes having mutually-exclusive guards
(`guard not EXISTS(...)`, real grammar, already demonstrated above) —
not something this module needs its own mechanism for either, but not
yet proven with a real two-machine firing sequence.
