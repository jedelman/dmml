# dmml-hs Phase 1 hardening: a real .cabal/cabal.project and CI

First two items on `jedelman/written-world#138`'s Phase 1 checklist
("`dmml-hs` hardening -- blocks everything else"), following straight
from today's earlier "canonical source will be dmml-hs... full speed
ahead Claude!" instructions.

## The problem

`dmml-hs` built only via hand-typed `ghc -isrc -iapp -O0 app/X.hs -o
binary` invocations, one per entry point, with no package manifest at
all. Nothing downstream could depend on it as a real package, there was
no single build command, and no CI could exist without one.

## What's real now

`dmml-hs.cabal` (commit `f85bcc4`): a library stanza exposing all 14
`DMML.*` modules under `src/`, plus one executable stanza per `app/*.hs`
entry point -- 18 total, kebab-cased from the existing hand-built binary
names where one already existed (`CheckDeclared.hs` -> `check-declared`,
etc.), `demo` for `Main.hs` matching README's own prior convention.
Every executable depends on the library's full dependency set
(base/aeson/bytestring/containers/directory/filepath/megaparsec/text) --
simpler and safer than hand-tracking which file needs which import,
verified by actually hitting and fixing every missing-module error `ghc`
raised until `cabal build all` exited 0 for the library and all 18
executables.

**Real verification, not just a clean compile**: re-ran
`fire-transition` and `retro-gate-demo` through the cabal-built binaries
(`cabal list-bin <name>`) against the exact scenarios exercised earlier
today by hand. `fire-transition keeper.dmml witnessEruption npc/keeper
--world world.dmml --param eruption=volcano/ashkar` still fires the
chained retract cleanly; adding `--machine dependent-watcher.dmml` still
correctly refuses, naming `watchtower/relay`'s `at`-guard by name.
`retro-gate-demo`'s four scenarios all still print the same PASS lines.
Identical output through the cabal build as through the ad hoc `ghc`
build earlier -- the package manifest didn't silently change behavior.

## A real mistake, caught in the same session before it shipped further

The first cabal.project baked `active-repositories: none` straight into
the tracked file -- a workaround for this dev sandbox's own broken path
to Hackage's secure index (`root.json` signature verification fails
here, no real network fetch available). It worked, because every
dependency `dmml-hs` needs happens to already be installed as a GHC
global/boot package in *this* sandbox specifically. That's not true of a
real GitHub Actions runner or another contributor's fresh machine --
`aeson`/`megaparsec`/etc. are not GHC boot libraries anywhere else, so
the committed override would have silently cut off real Hackage access
and broken `cabal build` for everyone but this one sandbox. Caught before
merging further work on top of it, not by CI (there wasn't any yet) but
by actually asking "would this work anywhere but here" before writing
the CI workflow that would have inherited the same broken assumption.
Fixed (`077ca2e`): the tracked `cabal.project` says nothing about
repositories now; the workaround moved to a new, gitignored
`cabal.project.local`. Re-verified with `rm -rf dist-newstyle && cabal
build all` that the sandbox still builds clean with the override moved.

## CI

`.github/workflows/dmml-hs-ci.yml` (commit `077ca2e`): `cabal build all`,
then the five demo binaries that are real self-contained assertions, not
just illustrative output (`governance-demo`, `guard-demo`,
`retro-chain-demo`, `retro-gate-demo`, `retroconsistency-demo` --
verified each one `exitFailure`s and prints `FAIL: ...` on a real
assertion failure, confirmed by reading `System.Exit` usage in all five
`app/*.hs` files, not assumed from their PASS-printing alone), plus
`examples/cascade-demo/run.sh`, the one example under `examples/` that
has an actual committed script rather than args typed by hand into a
terminal this session.

**Real, disclosed gap, not silently left out**: `fire-demo`, `sense-demo`,
`retract-demo`, `retract-value-demo`, `complex-demo`, and the
chained-retract-demo/value-disambiguation-demo examples were all
exercised for real today, but via ad hoc `fire-transition`/`validate-
commit` CLI invocations typed by hand in this session -- nothing commits
the exact `--world`/`--machine`/`--param` args anywhere, so there's
nothing for CI to run yet without first writing real `run.sh` scripts for
each (the way `cascade-demo/run.sh` already exists). Noted on
`jedelman/written-world#138` rather than claiming full demo-suite
coverage.

**Also disclosed**: this workflow has NOT been confirmed green on a real
GitHub Actions run -- this session has no path to trigger or observe
Actions runs. Everything checked here is local: a clean `cabal build all`
from a fresh `dist-newstyle`, all five demo binaries individually
confirmed exit 0 with real PASS output, `cascade-demo/run.sh` reaching
its documented fixpoint, and the workflow YAML parsing as valid YAML.
Worth confirming the first real push-triggered run rather than assuming
the `haskell-actions/setup` + `cabal update` + build sequence behaves
identically on a real runner as it does against this sandbox's
pre-provisioned package set.

## What's still open on Phase 1

Real atproto/XRPC connectivity (currently zero -- `dmml-hs` is
local-file-only); `jedelman/dmml#4`'s remaining FNV-1a-is-not-a-real-CID
note; `jedelman/dmml#6` (consumes citation-integrity checking). See
`jedelman/written-world#138` for live status.
