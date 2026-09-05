# template-compose-demo

Real, no-LLM proof of `SPEC.md` §19.4's "text as closed-set selection,
not open generation" design (`written-world#139`) — `DMML.TemplateBank`.

`world.dmml` declares two entities with mutually exclusive conditions:

- `npc/watcher` — `condition "corroded"`, `material "metal"`.
- `npc/keeper` — `condition "pristine"`, `material "metal"`.

The catalog (`app/TemplateComposeDemo.hs`) has three hand-vetted
templates, two of which cover mutually exclusive tags on purpose —
`worn-corroded` requires `condition = corroded`; `gleaming-pristine`
requires `condition = pristine`. This is the actual test: not just "does
the right template get picked," but "does the wrong one ever get picked
when its required fact isn't true."

```sh
template-compose-demo examples/template-compose-demo/world.dmml
```

Real output:

```
=== npc/watcher ===
eligible templates: ["worn-corroded","metal-generic"]
  -> npc/watcher looks worn, its surface corroded with age.
  -> npc/watcher is built of metal.

=== npc/keeper ===
eligible templates: ["gleaming-pristine","metal-generic"]
  -> npc/keeper gleams, freshly forged and untouched.
  -> npc/keeper is built of metal.
```

`npc/watcher` never gets `gleaming-pristine`; `npc/keeper` never gets
`worn-corroded` — the closed-set membership check
(`DMML.TemplateBank.eligibleTemplates`) actually excludes an ineligible
template, not just happens to pick the right one. Zero model calls
anywhere in this path: `eligibleTemplates` is a pure filter over
`DMML.Materialize.currentValue`, `renderTemplate` is a literal string
substitution of `{subject}`, nothing else.

**What this proves**: the runtime selection/slot-fill half of §19.4's
design holds for real, not just on paper. **What it doesn't prove**:
the governed catalog-expansion pipeline that would grow this catalog
(gap-detection guard → request effect → offline generation → review
gate → approved template) — `catalog` here is a fixed, hand-written
Haskell list, not sourced from any such process. That's real, separate,
unbuilt work, same as it was before this demo.
