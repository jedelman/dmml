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

Real output:

```
=== npc/smith ===
eligible templates: ["smith-at-work","any-active-worker"]
  -> npc/smith works the forge at mine/ninefathom, a master role/oresmith.
  -> npc/smith is active and at work.

=== npc/apprentice ===
eligible templates: ["smith-in-training"]
  -> npc/apprentice still learns the trade, apprenticed at mine/ninefathom.

=== npc/herbalist ===
eligible templates: ["herbalist-active","any-active-worker"]
  -> npc/herbalist tends forest/oldroot as role/blightreader.
  -> npc/herbalist is active and at work.
```

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
