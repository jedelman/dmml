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

## The real cost this trades away, not hidden

`{attr:role}`/`{attr:worksAt}` render the raw node path —
`"a master role/oresmith"`, `"at mine/ninefathom"` — not clean prose.
Going fully node-valued to make everything guard-walkable is a genuine
tradeoff against the E1 corpus's free-text literals, which read far
better out of the box (`"first down the ninefathom shaft"`) precisely
because they were never structured. This isn't solved here: a real
production template bank would need either a per-node "display name"
fact (itself node- or literal-valued, a separate design question) or
`renderTemplateWith` rendering just a node's last segment instead of
its full path — neither built, both genuinely open, and worth being
honest that "no narrative literals" has a real readability cost, not
just a structural win.
