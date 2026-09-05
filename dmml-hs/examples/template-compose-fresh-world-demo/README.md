# template-compose-fresh-world-demo

Real test against a world authored FRESH, from scratch, with the
explicit discipline Jason called for after `template-compose-e1-demo`
found the E1 corpus's descriptive content (`state`, `role`, `purpose`)
stored as string literals — un-guardable by design (`DMML.Guard`
structurally excludes literal-valued facts from a walk). `world.dmml`
here has **zero string literals anywhere**: `role`/`state` are declared
node references (`role/oresmith`, `state/active`, ...), same discipline
as `worksAt`/`:: a Type` always had.

```sh
template-compose-fresh-world-demo examples/template-compose-fresh-world-demo/world.dmml
```

(See the bottom of this README for real output — evolved twice since
this section was first written, see the two sections below.)

## What this proves that the E1 run couldn't

Because `role`/`state` are node-valued now, they can be **guard
conditions**, not just slot-fill content — `smith-at-work`'s guard
includes `guard self \`role\` role/oresmith` directly. The mutual-
exclusion case this world was built to test: `npc/apprentice` shares
`:: a type/smith` with `npc/smith` but has `state/training` instead of
`state/active` — it correctly gets ONLY `smith-in-training`, never
`smith-at-work` or the cross-type `any-active-worker`, proving state
alone (not just type) can gate selection now that it's structural.
`any-active-worker`'s single guard (`state/active`, no type clause at
all) correctly picks up both `npc/smith` AND `npc/herbalist` — a real,
deliberately type-agnostic guard, proving guards compose across
whatever granularity the author actually wants, not just the type-plus-
attribute shape every other demo so far has used.

## The real cost this trades away — resolved, not just flagged

First pass rendered raw node paths (`"a master role/oresmith"`,
`"at mine/ninefathom"`) and called the fix a genuinely open question.
Jason, laughing at that: "things can have names! they can have many
names for many reasons!" — right: a display name isn't a mechanism to
design, it's just another fact, asserted on the node the same way any
attribute is. `DMML.TemplateBank.resolvePath` (new) walks a dotted
`{attr:role.name}` marker through the node `role` resolves to and
renders THAT node's own `name` fact. `world.dmml` gives `role/oresmith`
**two** differently-purposed name facts — `name` = `"ore-smith"`,
`epithet` = `"master of the seam"` — and the catalog below picks
whichever fits: `smith-at-work`'s more formal sentence uses
`{attr:role.epithet}`, `smith-in-training`'s plainer one uses
`{attr:role.name}` on a different role node entirely. Same underlying
mechanism (`DMML.Materialize.currentValue`, walked one hop further),
same node carrying multiple names for different reasons, exactly as
described.

Real output now, full sentences, resolved entirely through facts:

```
=== npc/smith ===
  -> npc/smith works the forge at the Ninefathom seam, master of the seam.
=== npc/apprentice ===
  -> npc/apprentice still learns the trade, apprenticed at the Ninefathom seam as an apprentice.
=== npc/herbalist ===
  -> npc/herbalist tends Oldroot as blight-reader.
```

Nothing here is prose generation — `mine/ninefathom`'s `name` fact
(`"the Ninefathom seam"`) is exactly as hand-authored and exactly as
guard-invisible as any other literal content always was; what changed
is that rendering now resolves THROUGH a node reference to find it,
instead of stopping at the raw path.

## Singularities get names; relations and processes get described through their governing machine

Jason's next distinction: "a singularity needs a name [...] but for the
more general case — relations and processes — the template should not
be read directly from the node itself but from the machine that
produced it." `resolvePath`'s dotted-hop mechanic already had the right
shape (walk from a subject, through a relation, to a fact about what it
points at) — `DMML.TemplateBank.resolveViaGoverningMachine` reuses it
with a different first lookup: `DMML.Governance.findGoverningMachine`
(already real, load-bearing machinery from `jedelman/dmml#1`'s own
governed-arbitration design, keyed on the same `equips`/`trigger`
facts a governed catalog entry would use) finds which machine governs
`(subject, predicate)`, reads THAT MACHINE's own current `state`, and
resolves a description off the state node — a `{via:<predicate>.
<path>}` marker, alongside `{attr:...}`.

`world.dmml` and `world-later.dmml` are identical except for one
thing: `npc/apprentice`'s `worksAt` relation is governed by
`machine/apprenticeship` (`equips`/`trigger`), whose own `state` is
`state/earlydays` in one file and `state/seasoned` in the other — real,
separate description facts on each state node. Same template
(`smith-in-training`), same subject, same relation — the rendered text
changes completely because the underlying PROCESS moved, not because
anything about `npc/apprentice` or `mine/ninefathom` changed:

```sh
template-compose-fresh-world-demo \
  examples/template-compose-fresh-world-demo/world.dmml \
  examples/template-compose-fresh-world-demo/world-later.dmml
```

```
##### world.dmml #####
=== npc/apprentice ===
  -> npc/apprentice still learns the trade, apprenticed at the Ninefathom seam
     as an apprentice -- just started, still finding their footing in the dark.

##### world-later.dmml #####
=== npc/apprentice ===
  -> npc/apprentice still learns the trade, apprenticed at the Ninefathom seam
     as an apprentice -- has worked the seam long enough to know every tunnel by feel.
```

Still zero generation, zero LLM calls, and no new grammar — `{via:...}`
composes two already-real primitives (`findGoverningMachine`,
`currentValue`) the exact same way `{attr:...}`'s dotted path already
did.
