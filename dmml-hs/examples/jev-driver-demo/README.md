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

This was written in a sandbox with no GHC/cabal toolchain and no Jev
API key. `fire-transition` was never actually invoked, and no live
call to `api.typesafe.ai` was made. The CLI argument shape comes
straight from `FireTransition.hs`'s own `parseArgs`/usage string; the
wire format (`POST /v1/systemone`, `Authorization: Bearer`,
`{state, model, questions}` request, `{answers, usage}` response)
comes from `docs.typesafe.ai`, both read directly rather than
guessed. But "read the spec correctly" and "actually works" are
different claims — run `--dry-run` first once `fire-transition` is
built and on `PATH`, then a real run once the key is available, and
expect to find at least one wiring mistake before either does.
