# jev-driver-demo

A standalone Python script (`driver.py`) that lets Jev choose which
legal transition to fire, round after round, instead of a human or a
fixed script order deciding — with hard budget caps so "let it rip"
can't mean "run forever" or "run until it gets boring."

This is orchestration only. It calls the existing `fire-transition`
binary exactly the way `cascade-demo/run.sh` does (same dedup-by-hash
fix for the same reason — see that script's own comments) and reuses
`cascade-demo`'s `furnace`/`anvil`/`world.dmml` as its test bed, so it
isn't introducing a new world to reason about on top of a new driver.

## Running the first real test (needs a real network + a Jev key)

Everything in this directory has been written and tested as far as
possible without either. Two things block a genuinely live run, and
neither is fixable from a sandbox that lacks them:

1. **A real `fire-transition` binary.** `cabal build` needs Hackage
   (megaparsec, aeson) — every previous sandbox this was developed in
   had no working path to it (a cabal-install TUF signature mismatch,
   confirmed unrelated to the egress proxy; disabling that
   verification to route around it was correctly refused as
   security-weakening and not attempted). If cabal/Hackage work
   normally where you're running this, it's a non-issue.
2. **`TYPESAFE_API_KEY`.**

Once both are available, in order:

```sh
# 1. Build for real, confirm the toolchain actually works
cd dmml-hs
cabal build fire-transition
FIRE_TRANSITION="$(cabal list-bin fire-transition)"
export FIRE_TRANSITION

# 2. Sanity-check the binary itself against cascade-demo BEFORE
#    involving Jev at all -- proves the CLI arg shape this driver
#    assumes (see driver.py's own module doc) is actually right
cd examples/cascade-demo
"$FIRE_TRANSITION" furnace.dmml smelt smelts --world world.dmml --param ore=ore/raw1
# expect: a real, printed DMML commit (smithy/furnace `refinedInto`...) --
# if this fails or errors, stop here and fix the mismatch before touching
# the driver; it's a smaller surface to debug in isolation.

# 3. Dry-run the driver against the real binary (still no Jev call)
cd ../jev-driver-demo
python3 driver.py candidates.json --dry-run
# expect: the same "round 1 / round 2 / fixpoint" shape the
# fake-fire-transition.py shim already produces -- if the shape
# differs, the real binary's output doesn't match what driver.py
# assumed and needs reconciling before step 4.

# 4. The actual first live run
export TYPESAFE_API_KEY=...
python3 driver.py candidates.json
# read audit.jsonl afterward -- it has the full Jev response (choice,
# probabilities, confidence) for every round, not just which candidate won.
```

Expect to find at least one real mismatch at step 2 or 3 before step 4
works — every piece of this has been written carefully but none of it
has touched a real binary yet, and "read the spec/code correctly" and
"actually works" have been two different claims all the way through
this project. `fake-fire-transition.py`'s own doc comment says the
same thing; delete it once step 3 passes for real.

## What this actually is

Per round:
1. Dry-fire every candidate in `candidates.json` against the current
   world snapshot (`fire-transition ... `, exit 0 = legal).
2. Drop anything identical to its own last firing (a guard that's
   still satisfied after firing "succeeds" with nothing new — the
   real bug cascade-demo's dedup exists to catch).
3. Nothing legal and new → stop. This is the *healthy* stopping case.
4. A budget cap hit first → stop and print exactly which cap and why.
   Caps never auto-raise themselves.
5. Otherwise: legal candidates go to Jev as a Choice question, the
   winner gets applied for real (written into the world dir as the
   next fact file), and the full round — including Jev's raw
   response with its probabilities/confidence, not just the winner —
   gets appended to `audit.jsonl`.

## Budget knobs (`candidates.json` → `budget`)

- `max_rounds` — hard stop regardless of anything else.
- `max_total_firings` — across all candidates.
- `max_firings_per_candidate` — the guard against Jev picking the
  same well-formed, valid, boring action over and over. The included
  config sets this to 1 for both candidates, so this particular demo
  can only ever run 2 rounds — intentional, it's a smoke test, not a
  world.
- `max_minted_nodes` — counted by a text-scan heuristic over commit
  output (anything token-shaped like `foo/bar`), not a real
  `DMML.Ast` parse. Good enough to bound a run; treat it as
  approximate, not authoritative.

## Running it

```
export TYPESAFE_API_KEY=...   # or pass --api-key
export FIRE_TRANSITION=/path/to/fire-transition   # or put it on PATH

python3 driver.py candidates.json
```

`--dry-run` skips the Jev call entirely and deterministically picks
the first legal candidate each round — use this to check the loop's
mechanics (legality, dedup, budgets, audit log) before spending a
single Jev call or having a key at all.

## Machines that spawn machines, and where they land

`DMML.Ast.EffectSpawn`/`spawn <term> from <node_ref>` lets a transition
mint a whole new machine instance, not just a fact. As of
`DMML.Fire.renderFiredCommits`, a spawned machine renders as real DMML
Surface **commits** — `DMML.MachineFacts.encodeMachine`'s output, one
commit per fact (never two facts sharing a (subject, predicate) key in
one commit — the one hard constraint the whole machines-as-facts design
fits inside, see `dev-journal/2026-09-17-machine-facts-unification-phase1.md`)
— not a Surface `machine` block needing a parser round-trip before it's
usable again. Applied straight into the world dir as ordinary `--world`
files, a spawned machine is immediately fireable via
`DMML.Fire.fireTransitionFromFacts`, and immediately visible to
`list-candidates`/`DMML.MachineFacts.candidateTransitions` below —
verified end to end by `examples/jev-driver-demo/spawn-facts-pipeline-selftest.hs`:
a machine fired a transition that had never existed as anything but
facts a PRIOR firing produced. (`DMML.Fire.renderFiredMachine`'s Surface
`machine` block still exists too, printed alongside, purely for human
inspection or a tool that specifically wants Surface text.)

`candidates.json` in this demo doesn't exercise spawn yet (its two
candidates are plain asserts, inherited from `cascade-demo`), but the
driver's own dry-fire/dedup/budget loop applies to a spawn-firing
candidate exactly the same way — `fire-transition` exiting 0 is still
the legality oracle.

Before turning a driver loop loose on spawn-capable machines: run
`check-spawn-cycles` over the full candidate machine set first. It
flags (never blocks) a template whose own spawn effects loop back to
itself, directly or through others' — the "reproductive system for
universes" risk a spawn capability raises that plain assert/retract
never did. A flagged cycle isn't automatically a bug (a guard could
make the loop's next iteration unreachable in practice), but it's
exactly the kind of thing this driver's `max_firings_per_candidate`
budget cap exists to backstop if a real run turns out to hit it.

## Automatic candidate discovery — real, for fact-native machines only

`list-candidates <world.dmml>...` (new binary, `app/ListCandidates.hs`)
prints every `(machine, transition, params)` triple it can find by
querying a built snapshot directly — `DMML.MachineFacts.candidateTransitions`,
compiled and tested for real (`examples/jev-driver-demo/candidate-discovery-selftest.hs`):
built a snapshot from two real fact-native machines plus a THIRD
machine that was only ever a Haskell value, never applied — discovery
found exactly the first two's transitions and correctly never saw the
third.

**This is real automatic enumeration, not the same "hand-author
`candidates.json`" limitation from before — but only for machines that
exist AS FACTS.** A machine spawned via `EffectSpawn` (now that it
renders as commits, see above) qualifies automatically. A hand-authored
`cascade-demo`-style Surface-text machine does NOT, unless something
also runs it through `DMML.MachineFacts.encodeMachine` — `furnace.dmml`/
`anvil.dmml` are still invisible to `list-candidates` as written. Wiring
this driver's own `candidates.json` to call `list-candidates` instead
of a hand-written list is the natural next step for a seed world whose
producer machines spawn their own successors, but it isn't done here —
this README documents the capability existing, not the Python driver
having been rewired to use it.

## Known, disclosed scope limits

- **Candidates are hand-authored, not auto-enumerated.** DMML has no
  CLI-exposed "every node of type X, what's legal" query yet —
  `DMML.Guard.availableTransitions` takes one already-bound
  `EvalContext`, it doesn't generate candidate bindings itself
  (`WorldBrowserDemo.hs`'s own doc comment flags the identical gap).
  Growing `candidates.json` is what stands in for "more possible
  content" until that enumeration exists as real tooling.
- **The `state` text handed to Jev is a thin per-round summary**
  (`build_state_summary` in `driver.py`), not the full world
  snapshot. This is the first thing to make richer once you can see
  what Jev actually needs to choose well — right now it's a
  placeholder proportional to what a smoke test needs, not a
  considered design.
- **Minted-node counting is a regex heuristic**, described above —
  fine as a budget signal, not fine as an audit-grade fact.

## Testing status — read before trusting this

This was written in a sandbox with no network access to Hackage (a
TUF signature-verification mismatch with the available cabal-install
blocked fetching megaparsec/aeson; disabling that verification to
route around it was correctly refused as security-weakening, not
attempted) and no Jev API key. `fire-transition` was never actually
invoked against real `.dmml` content, and no live call to
`api.typesafe.ai` was made. The CLI argument shape comes straight from
`FireTransition.hs`'s own `parseArgs`/usage string; the wire format
(`POST /v1/systemone`, `Authorization: Bearer`, `{state, model,
questions}` request, `{answers, usage}` response) comes from
`docs.typesafe.ai`, both read directly rather than guessed. But "read
the spec correctly" and "actually works" are different claims — run
`--dry-run` first once `fire-transition` is built and on `PATH`, then a
real run once the key is available, and expect to find at least one
wiring mistake before either does.

**What WAS actually compiled and run for real**, once a bare
GHC/cabal-install was installed (`apt-get install ghc cabal-install`
worked without any network dependency): every module in `dmml-hs/src`
that doesn't transitively need megaparsec or aeson — `DMML.Ast`,
`DMML.Guard`, `DMML.Materialize`, `DMML.Fire`, and the new
`DMML.SpawnCycles` — was compiled against the REAL, unmodified source
(only `DMML.Surface`/`DMML.Retroconsistency` were stubbed, with their
real exported type signatures copied verbatim, to satisfy `Fire.hs`'s
imports without a real parser). `spawn-cycles-selftest.hs` and
`spawn-fire-selftest.hs` in this directory are real, executed test
programs (not just `ghc -fno-code` type-checks) — the former caught
and fixed a genuine off-by-one in the cycle-detection walk before it
shipped; the latter fires a real `EffectSpawn` through the real
`fireTransition`/`renderFiredMachine` and checks the output byte for
byte. What's still unverified is anything touching the real
`DMML.Surface` parser or `DMML.Json`/`DMML.FromJson`'s aeson instances
— the new `spawn` grammar in `Surface.hs` and the new
`EffectSpawnInput` wire shape in `Json.hs`/`FromJson.hs` were written
by mirroring the existing `assert`/`retract` code paths exactly, but
neither has been compiled, let alone run.

**Same session, next wave** (`renderFiredCommits`, `decodeMachineFromSnapshot`,
`fireTransitionFromFacts`, `candidateTransitions`): compiled and
actually run against the real, unmodified source the identical way —
`spawn-facts-pipeline-selftest.hs` fires a spawner, checks
`renderFiredCommits` produces one commit per encoded fact with no
duplicate-key violations, then applies the SPAWNED machine's own facts
and fires ITS transition too, end to end. `candidate-discovery-selftest.hs`
builds a snapshot from two real fact-native machines and confirms
`candidateTransitions` finds exactly their transitions, correctly
missing a third machine that was never applied. `app/ListCandidates.hs`
(new CLI) and `app/FireTransition.hs`'s updated `main` are, like every
other CLI in this project, UNCOMPILED — both need `DMML.Surface` to
parse real files, and megaparsec still isn't available here. Their own
logic (`candidateTransitions`, `renderFiredCommits`) is the part that's
actually verified; the thin CLI wrapper around it is not.
