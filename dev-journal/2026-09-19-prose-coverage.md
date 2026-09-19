# 2026-09-19 — Prose coverage: which facts have no sentence

Jason: *"Do all of our predicates have prose templates so all of our facts are
deterministically describable?"* → **no, and not close** → *"build the coverage report,
then we can write canonical templates."*

## What existed, measured

```
distinct predicates across examples/ : 105
templates in existence               :  12   (inside four demo binaries)
predicates any template guards on    :   6
```

Three prose paths, none per-predicate:

- **`DMML.TemplateBank`** — the right mechanism. A template is `(guards, text)`;
  eligibility is an ordinary `GuardClause` run through the shipped `evalGuards`, so
  templates get negation and multi-hop patterns free. But the catalogs were Haskell
  literals inside demos, and its own haddock says the catalog-expansion pipeline is not
  built.
- **`check-describable`** — checks every **subject** has a *reachable* description
  mechanism. Its haddock is explicit: it "does NOT check that today's specific content
  already produced good prose." A world passes it with every subject named and not one
  relation sayable.
- **the Jev driver's describers** — which turn out to be the raw triple with the
  backticks stripped. Fed awkward predicates it produces *"apprenticesUnder master/iolo;
  holdsRelicOf saint/nonna; refinedInto flour/fine"*. That read fine all week only
  because the demo vocabulary happened to be English verbs in the right form.

**Facts were deterministically *renderable* — one textual form each, the triple. Not
deterministically *describable*.**

## `check-prose-coverage`

Per **fact**, not per subject and not per predicate. A fact `(subject, predicate)` is
covered iff some template **eligible for that subject** (real `evalGuards`) is **about
that predicate** — its guards test it, or its text renders it through an
`{attr:…}`/`{via:…}` marker. The report groups by predicate because that is how the
writing is best ordered; what it checks is facts.

Plus a catalog file format, so a catalog can leave a Haskell literal:

```
template at-place
  guard self `at` ?place
  text "{subject} stands at {attr:at}."
```

The container is new; the guards are not — they go through `guardsFromText` verbatim, so
a catalog cannot express a condition a machine could not. `guardsFromText` was copied in
all four demos; it moved into `TemplateBank` rather than becoming a fifth copy.

## Three things the tool found by being run

**The ignore list was wrong.** First run put `name` (54 facts) and `description` (30) at
the top of the gap — a ranking that would have sent someone off to write "a template for
the name predicate." Those *are* the description mechanism. Now ignored alongside
`state`/`cleared`. `a` is deliberately kept: "X is a Y" is real prose.

**An unresolved marker is not an error.** `renderTemplateWith` substitutes what it can
and leaves the rest verbatim, so `{attr:has.name}` against a world whose objects carry no
`name` fact ships the brace inside the sentence. The tool now renders every eligible
template and fails on any surviving `{`. It caught my own first catalog immediately.

**Literal-valued facts can never be covered by a guard on their own predicate.**
`DMML.Guard` structurally excludes them from a guard walk — TemplateBank's haddock says
so. I wrote templates for `count`, `purpose`, `quality`, `weight`, `year` and watched all
five stay uncovered *with a template each*. The report now splits node-valued from
literal-valued and says what each needs, because listing them together asks for one thing
that works and one that cannot.

Then the obvious fix for literals — make them eligible by `self \`a\` ?type` — produced
**"There are {attr:count} of Wren Aldy, Apprentice."** Eligibility for a literal has to
be a node-valued fact that reliably *co-occurs* with it, which is per-world authoring, not
something a canonical catalog can supply. Tried, caught by the leak check, removed, and
documented in the catalog rather than shipped.

## Where it stands

`examples/prose-catalog/core.catalog`, 20 templates:

| world | covered |
|---|---|
| the world the full live run built | **12 / 12 — every fact, every template clean** |
| all of `examples/` aggregated | 165 / 244, tail of 34 node-valued + 14 literal-valued |

CI pins the **closed** world (cannon-fanout), where the whole vocabulary is covered, so
any regression goes red. The wider corpus is deliberately not pinned — it has a long tail
and a known literal gap, and would be red on day one and stay red.

## Still open

- **34 node-valued predicates** in the tail, mostly 1–2 facts each (`drains`, `berth`,
  `grazesIn`, `downstreamOf`). Mechanical to write; nobody has.
- **14 literal-valued predicates** need per-world eligibility, or a change to
  `DMML.Guard` to admit literal guards — which its haddock says is faithful to the real
  crate, so that is a spec question, not a patch.
- **The driver still does not use any of this.** `describe_frontier_node` still
  concatenates triples. Wiring the catalog into the Jev loop is the obvious next step and
  is not done.
- A predicate minted at runtime (`imply`/`transmute` invent `(pred, obj)` pairs mid-run)
  can outrun any catalog. Coverage-as-measurement handles that; coverage-as-fixed-table
  never could.
