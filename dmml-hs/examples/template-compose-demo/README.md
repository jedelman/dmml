# template-compose-demo

Real, no-LLM proof of `SPEC.md` §19.4's "text as closed-set selection,
not open generation" design (`written-world#139`) — `DMML.TemplateBank`.

`world.dmml` declares three entities:

- `npc/watcher` — `:: a MetalObject`, `condition "corroded"`, `material
  "metal"`.
- `npc/keeper` — `:: a MetalObject`, `condition "pristine"`, `material
  "metal"`.
- `creature/wraith` — `:: a Spirit`, `condition "corroded"` — same
  attribute value as `npc/watcher`, deliberately a different type.

Every template in the catalog (`app/TemplateComposeDemo.hs`) is bound
to exactly one required type (2026-09-04, Jason: "templates should be
bound to types") in addition to whatever attribute tags it covers —
checked against the subject's own real `a`/rdf:type fact
(`DMML.Ast.RdfType`, already-shipped grammar, not a new construct).

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

=== creature/wraith ===
eligible templates: []
```

The `wraith` case is the actual point of this revision, not just an
extra fixture: it shares `condition = corroded` with `npc/watcher`, so
an attribute-tag-only check (this demo's original version) would have
incorrectly matched it to `worn-corroded` — a spirit narrated as
"corroded" makes no sense, but nothing in a flat tag check would have
caught that. Binding `worn-corroded` to `MetalObject` makes the
cross-match structurally impossible rather than merely unlikely: zero
eligible templates for `wraith`, confirmed by real output, not argued.

**What this proves**: type-binding closes a real gap in the original
design — attribute overlap alone was never a strong enough signal that
two entities are the same KIND of thing. **What it doesn't do**: define
type SCHEMAS (which attributes/values are even legal for a given type,
the thing that would fully subsume the separate "cross-decorator
compatibility matrix" SPEC.md §19.4 originally called out as needed
alongside catalog membership) — that's a real, larger, still-open
primitive, not built here. This demo only checks that a subject's
*already-asserted* type matches a template's required type; it doesn't
constrain what a type is allowed to assert in the first place.
