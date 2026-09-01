# Endurance/stability report — 20-round, 4-agent DMML authoring

Real evidence only: every number below is computed directly from
`results/stdout.log` and `results/commits/*.dmml` (committed alongside
this report), not estimated or recalled. The parsing script used to
produce the per-agent/per-round breakdowns is reproducible from the raw
log — see the categorization method noted inline.

## Setup, in brief

10 hand-authored, open-ended `machine`-governed nodes (a mountain, a
mine, a forest, two herds, three technical paths, two cultures — see
`dmml-hs/examples/endurance/`) seeded a small world by hand. Four
models — `kimi-k2.5`, `deepseek-v4-flash-0731`, `glm-5.3-flash`, and
`gemini-3.7-flash` for rounds 1–4 (swapped for a second `deepseek-v4-
flash-0731` instance from round 5 on, to cut token cost — see below)
— ran 20 stacked rounds. Each round, every agent got a freshly
materialized, randomly-sampled, slightly-overlapping *corner* of the
real world (not a shared static context, not the whole world) and
could author up to 4 commits/machines, blind to the other three agents'
same-round output. After each round, every pairwise combination of
agents' new output was checked for real divergence via `check-
divergence`; a genuine conflict minted a `Contest` (never silently
resolved, never blocked) instead of being merged over.

## Headline result

**All 20 rounds completed. Zero thrash**, against three explicit stop
conditions (an oscillating contest across rounds, 3+ simultaneous new
contests in one round, ≥50% authoring failure rate in a round) that
none of the 20 rounds ever tripped.

- 208 accepted commit/machine files.
- 3 genuine contests minted, all in rounds 1, 2, and 4 — **zero new
  contests in the 16 rounds from 5 through 20.**
- World grew from a 10-node seed to 52 nodes visible in a single
  agent's corner by round 20 (corners are a bounded sample of a larger
  whole, so total world size is larger still).
- Per-round authoring failure rate ranged 0.00–0.45 with no trend
  toward runaway failure.

## What the contest timing actually shows

The three contests weren't evenly spread — they clustered entirely in
the first third of the run, then stopped completely. This isn't
coincidence, and it's the most structurally interesting finding here:
**divergence risk was a function of world size relative to a fixed
corner size, not of round count or agent count.** Early on, the world
had only ~10–16 nodes total; a corner sampling ~6–16 of those (with a
deliberate overlap fraction) had a real chance of two different agents
both landing on the same "obviously next thing to do" node — and
sometimes disagreeing about what to do with it (contests in rounds 1,
2, 4 hit `path/greenlore`, `herbalist/onn`, and `cistern/ninefathom` —
exactly the small set of seeded, machine-governed, "interesting"
nodes). As the world grew past ~20–30 nodes, the same fixed corner
size and overlap fraction covered a shrinking *proportion* of the
total graph, so the odds of two agents' corners both touching the same
specific fact dropped off — without any change to the sampling logic
itself. The stability observed from round 5 onward is a property of
the corner-to-world-size ratio shrinking as content accumulates, not
evidence that 4 agents "learned" to avoid each other.

This has a direct implication for anyone building on this harness: a
small, sparse world is the highest-divergence-risk regime for this
sampling design, not a large one. A cold-start world (or a world that
resets/prunes content) would need either a smaller corner, less
overlap, or fewer simultaneous agents to avoid exactly the round-1
pattern seen here.

## Per-agent reliability — a real, sizable spread

| agent | valid | invalid | attempts | invalid rate | commits | machines |
|---|---|---|---|---|---|---|
| kimi | 58 | 3 | 61 | **5%** | 55 | 3 |
| glm | 49 | 3 | 52 | **6%** | 45 | 4 |
| deepseek | 48 | 17 | 65 | **26%** | 38 | 11 |
| deepseek2 | 35 | 16 | 51 | **31%** | 27 | 8 |
| gemini (rounds 1–4 only) | 12 | 0 | 12 | **0%** | 9 | 3 |

Two real things worth separating here. First, `kimi` and `glm` were
roughly 5x more reliable than either `deepseek` instance across this
run — a genuine, sizable gap, not noise (65 and 51 real attempts for
the two deepseek instances is enough to trust the rate). Second,
`gemini` had a *perfect* record in its 4 rounds (0/12 invalid) —
dropping it after round 4 was purely a token-cost decision (mandatory
reasoning, heavier per-call cost), not a quality one; nothing here
should be read as "gemini was unreliable."

## Failure-mode taxonomy (39 invalid attempts, categorized from the real parser errors)

| category | count | share |
|---|---|---|
| unexpected-token (misc syntax) | 16 | 41% |
| hyphenated commit verb (`commit river-rites` — `-` isn't a valid ident char) | 8 | 21% |
| duplicate fact within one commit | 4 | 10% |
| machine/commit keyword confusion (opened with `machine` where `commit` was needed, or vice versa) | 4 | 10% |
| bad indentation (Surface syntax's fixed-indent rule) | 2 | 5% |
| garbled/glitch-token output | 1 | 3% |
| other | 4 | 10% |

The single largest, most *fixable* category is the hyphenated-verb
one (21%): models keep reaching for `commit river-rites` or `commit
settle-expedition` — natural, readable verb phrasing that the grammar's
`isValidIdent` simply doesn't allow (`-` isn't a valid identifier
character; `SURFACE.md`'s grammar requires a bare, hyphen-free ident).
This is a real, cheap prompt-engineering fix (an explicit "commit verbs
are single identifiers, no hyphens" line) that would likely have
removed a fifth of all failures outright — not a model-quality problem,
a grammar-reference-completeness gap in the system prompt.

## Self-pacing: agents don't all "stop" the same way

Given up to 4 attempts per round, agents used the room differently:

- **`kimi` self-reported DONE explicitly 10 times** (out of ~15
  rounds it participated in under the new roster), averaging 2.35
  commits before stopping on its own — a real, consistent
  "I'm finished for this round" signal, not just hitting the cap.
- **`glm` never once said DONE** — it stopped by going silent (no
  fenced content in its reply) 7 times instead. Functionally
  equivalent in this harness (both end the round), but a distinct
  behavioral signature: glm doesn't verbalize completion, it just runs
  out of things to say.
- `gemini` (4 rounds) said DONE 4 times; `deepseek`/`deepseek2`
  self-stopped rarely (2 and 1 times respectively), more often either
  exhausting the 4-attempt cap or hitting 2 invalid attempts in a row
  (the harness's own early-abort-on-repeated-failure rule).

## The "de-prose" finding, quantified

Flagged earlier from one example; the real count: **55 string-literal
facts over 70 characters** across the whole run, i.e. genuine
sentence-or-paragraph text living inside what's supposed to be
world-state data. Breakdown by agent: **glm 33, kimi 17, deepseek 5**
(deepseek2 and gemini: none over the threshold). Notably, `kimi`
authored the single longest one in the entire corpus — 309 characters,
a full trade-relationship narrative — despite otherwise being the most
mechanically reliable agent; prose-drift and syntax-reliability are
independent problems, not the same failure. None of this breaks
anything (every one of these is still a valid quoted string literal,
still parses, still round-trips through `render-snapshot` correctly) —
it's a style/scope drift, not a correctness bug. Worth a tighter
system-prompt instruction ("a `purpose`/`role`/`description` value is a
short phrase, not a sentence") for any future run; not yet built.

## What this run does and doesn't prove

**Proves**: the mint-not-reject Contested primitive and the corner-
sampling authoring loop both hold up under real, sustained (20-round),
multi-agent (4-way), blind-to-each-other load — no thrash, no runaway
failure, no unbounded contest pileup, real divergence correctly
surfaced exactly 3 times and never silently resolved either time.

**Doesn't prove**: that this scales past 4 agents or past a ~50-node
corner-sampling window (the contest-clustering finding above suggests
those two variables interact, not just each alone); that any of this
holds when agents *are* told to resolve contests as part of their task
(none were, in this run — 2 of the 3 raised contests are still live,
unresolved, in `results/snapshot-final.txt`, which is the correct,
intended behavior for content nobody has yet witnessed a resolution
for, not a gap in this run); or that the real git-sync mechanism
(`run_git_sync.py`, validated separately on a 1-round dry run) holds up
under the same 20-round load — that's a distinct, not-yet-run test.
