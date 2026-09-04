# dmml-hs is canonical; the Rust dmml/dmml-runtime crates are retired

Jason, in response to being shown the real ambiguity (`dmml-agent-nucleus/
GRAMMAR.md` names `dmml/src/from_json.rs` — the Rust crate — as source
of truth, while `android-poc/README.md` already called dmml-hs "the
interpreter this bridge exists to carry"): "canonical source will be
dmml-hs. not sure why that would be ambiguous! we can actually fully
retire the rust implementation (RIP)."

## What this actually settles

`dmml-hs` is now the one real interpreter — both its JSON front-end
(`DMML.Json`/`DMML.FromJson`, matching the Rust crate's old wire shapes)
and its own text front-end (`DMML.Surface`, real grammar in
`dmml-hs/SURFACE.md`, which the Rust crate never had at all) are
canonical. Everything built today — generalized `Effect`, chained
retract, `DMML.Fire`'s real execution/firing semantics, the whole-tree
consistency gate, value-qualified `consumes` — is now THE grammar, not
a parallel experiment nobody had to reconcile with the "real" one.

`GRAMMAR.md` (the one page anyone forking `dmml-agent-nucleus` is
actually pointed at) is rewritten accordingly — see that commit.

## Real, checked blast radius before touching anything destructive

`written-world` still has FOUR real Rust packages with a live git
dependency on the Rust crates, not just a stray reference:

```
server/Cargo.toml:   engine = { package = "dmml-runtime", git = ".../dmml", rev = "8116b2b..." }
server/Cargo.toml:   dmml = { git = ".../dmml", rev = "8116b2b..." }
client/Cargo.toml:   (same two)
cli/Cargo.toml:      engine = { package = "dmml-runtime", ... }
appview/Cargo.toml:  engine = { package = "dmml-runtime", ... }
```

Real usage, not just declared: 7 files under `server/src/` and 2 under
`client/src/` actually reference `engine::`/`dmml::`. This is a genuine
migration in `written-world`, not a one-line Cargo.toml edit — those
four packages need to either call into `dmml-hs` some other way (FFI,
the JNI-bridge shape `android-poc/` already proved works for a simple
case) or get rewritten against dmml-hs directly, before the Rust crates
can actually be deleted out from under them.

**Not done in this session, deliberately**: the Rust `dmml`/
`dmml-runtime` crates themselves are NOT deleted yet. Per this project's
own "hard-to-reverse operations get checked first" discipline, removing
code four other real packages still build against needs its own
sequencing decision (migrate `written-world`'s four packages first, then
delete the Rust crates; or mark the crates deprecated/frozen now and
delete once nothing points at them) rather than an in-session unilateral
deletion.

## What's done this session, marking the decision real rather than just stated

- `GRAMMAR.md` rewritten: source-of-truth pointer moved from
  `dmml/src/from_json.rs` to `dmml-hs`'s own `DMML.Json`/`DMML.FromJson`
  (JSON front-end) and `dmml-hs/SURFACE.md` (text front-end); the
  machine/effect section rewritten to describe the real, current
  generalized grammar (general assert/retract, chained retract,
  node-minting via a transition parameter) instead of the old
  `{kind: "assert"|"retract", ident}`-only shape; a new section on
  firing (`DMML.Fire`/`fire-transition`) added, since execution
  semantics didn't exist in either implementation when `GRAMMAR.md` was
  first written.

## What's still open, real, not yet done

- The Rust crates themselves: not deleted, not yet marked deprecated in
  their own READMEs.
- `written-world`'s four-package migration off `engine`/`dmml`.
- `dmml-hs` still has no `.cabal`/`cabal.project` — "canonical" and
  "hand-typed `ghc` invocations, no buildable package" don't sit well
  together for something other packages are now supposed to depend on
  for real.
- No CI for dmml-hs at all.
- **Found while rewriting `GRAMMAR.md`, not assumed**: the retired Rust
  crate's `graph.rs` checked a `consumes` citation's `cid` against what
  it had actually recorded as observed; `DMML.Materialize.applyConsume`
  in `dmml-hs` has no equivalent at all — a citation naming a `cid`
  nobody ever saw is accepted the same as a real one. Real regression
  now that `dmml-hs` is canonical rather than a spike; tracked as
  jedelman/dmml#6, not designed or fixed here.
