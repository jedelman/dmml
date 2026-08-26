# DMML Browser

A self-contained, single-file HTML viewer (`index.html`) for inspecting
and visualizing any DMML commit graph — nodes are commits, edges are
`consumes` citations, and vertical position is **generation**: how many
citation-hops back to the nearest base fact, computed structurally by
`dmml::graphview::export_graph` (`dmml/src/graphview.rs`), not asserted.
Deep strata (older, cited-from facts) sit at the top; later commits that
build on them stack below, the same register the desiring-production
paper's own strata/sedimentation language already uses.

Click any commit to see what it produced, what it cites, and what cites
it back; pan and zoom the graph itself. No build step, no server, no
external JS — open `index.html` directly in a browser.

## Regenerating the embedded data

The three graphs currently embedded (`pantheon.rs`, `benjamin_full_
essay.rs`, `autoregressive_critique.rs`) are static JSON snapshots taken
at the time this file was built, not live-loaded — the browser has no
server to fetch from. To refresh them after changing one of those
examples:

```sh
cargo run -p dmml --example pantheon
cargo run -p dmml --example benjamin_full_essay
cargo run -p dmml --example autoregressive_critique
```

Each prints its own `Graph exported to .../examples/output/<name>.graph.json`
path. Then splice the fresh JSON into the matching
`<script type="application/json" id="graph-...">` block in `index.html`
(the `<div class="app">`'s three data blocks, in the same order as the
tabs) — a small Python one-liner reading the JSON file and replacing the
block's `"graph": {...}` value is the least error-prone way to do this
for the larger graphs; see this file's own git history for the exact
substitution used when it was first built.

## Adding a new graph to the browser

Any other example file can export itself the same way — see the export
call at the end of `pantheon.rs`'s or `autoregressive_critique.rs`'s
`main()` for the ~8-line pattern (`dmml::graphview::export_graph(&log)`,
serialize with `serde_json`, write to `examples/output/`). The exporter
itself is generic: it knows nothing about any specific example's
content, only the `consumes`/`produces` shape every DMML commit already
has. Add a fourth `<script type="application/json" id="graph-...">`
block here with the new export's contents, and add its id to the
`GRAPHS` array near the top of the `<script>` block.
