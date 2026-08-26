# A dmml browser (2026-08-26)

Jason asked to build a browser to inspect and visualize the
autoregressive critique structure — a real, standing tool this time,
not another one-off example file.

**Data layer** (`dmml/src/graphview.rs`, new module in the core `dmml`
crate): `export_graph(&[IdentifiedCommit]) -> GraphExport` — plain
nodes/edges/generation JSON built generically from `consumes`/`produces`
alone. Deliberately knows nothing about Benjamin, papers, or critique
cycles specifically; the same function produces the same shape for any
example file's log. **Generation** is the useful new concept here: 0 for
a node with no in-graph dependency (a base fact), otherwise one more
than the max generation among what it cites *in this same graph* — this
is exactly what makes an autoregressive or recombinant structure
visually legible, and it generalizes the first-order/second-order
distinction `autoregressive_critique.rs`'s own Checks 1-2 already
computed by hand into something any graph gets for free. Confirmed on
real output: `pantheon.rs`'s Nyx sits at generation 1 above Helios/
Selene/Eos; the Benjamin essay's Epilogue sits at generation 17, exactly
matching the essay's own real citation depth; the critique file's cycles
1/2/3/5 land at generation 1 and cycles 4/6 — the ones that spontaneously
cited a prior critique instead of the base paper — land at generation 2,
without the exporter knowing anything about "first-order" or
"second-order" as concepts.

Wired the export into `pantheon.rs`, `benjamin_full_essay.rs`, and
`autoregressive_critique.rs`'s existing `main()` functions (an ~8-line
addition to each, reusing the `full_log`/`log`/`all_commits` vector each
already built for its own checks) — all three still pass every existing
check with the export wired in, confirmed by a clean re-run of all
three.

**The browser itself** (`browser/index.html`): a single self-contained
HTML file, no build step, no server — nodes laid out by generation as
horizontal "strata" bands (deepest/most-cited-from at top), edges as
curved citation paths, click-to-inspect showing full produced text plus
clickable cites/cited-by lists, pan and zoom, three embedded graphs
switchable by tab. Load `artifact-design` first per house process;
picked a warm sediment/citation palette (ochre for production, teal for
citation, dashed rust for a citation that points outside the loaded
graph) since the strata metaphor is literally what generation depth
means here, not a decorative choice.

**A real bug caught by actual browser testing, not just eyeballing the
code**: clicking a node did nothing at first — Playwright confirmed the
click event's target was `#canvasWrap` (the pan/zoom container div), not
the node underneath the cursor. Root cause: `wrap.setPointerCapture()`
was called unconditionally on every `pointerdown`, and Chromium
redirects the *click* event (not just pointer events) to the capturing
element once capture is set, even for a plain click with zero movement.
Fixed by only engaging capture once real movement crosses a small
threshold (4px), and always releasing it on `pointerup`/`leave`/`cancel`
— confirmed fixed by re-running the same Playwright script and reading
`node.getAttribute('class')` directly rather than trusting a screenshot
(the first "it looks selected" screenshot was actually just a `:hover`
state from Playwright's mouse still sitting on the element, not a real
`.selected` class — caught by checking DOM state, not just pixels).
Second real bug, same testing pass: `shortSubject()` took a subject's
last `/`-segment as its label, which turns `sky/1` into the label "1" --
fine for `argument/preface`-shaped subjects, useless for numeric-id ones.
Fixed to keep the full subject when the last segment is bare digits.

Screenshotted all three graphs in both themes via a headless Chromium
(Playwright, pre-installed in this environment) before calling this
done — light and dark both render legibly, pan/zoom and node selection
confirmed working via direct DOM assertions, not just visual inspection.

Published as a Claude Artifact for interactive use
(https://claude.ai/code/artifact/05fa256b-029c-4550-a439-09ac1500554a);
`browser/index.html` in the repo is the source of truth, `browser/
README.md` documents how to regenerate the embedded JSON after an
example file changes, and how to wire a new example file into the
browser using the same ~8-line export pattern.
