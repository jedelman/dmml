# dmml-hs

A Haskell translation of the DMML JSON authoring surface — not a runtime
engine, a precise executable spec artifact. Every type here is translated
field-for-field from the real Rust `dmml` crate's `src/from_json.rs` and
`src/ast.rs`, checked against that source directly rather than against
any of this repo's prose docs (`GRAMMAR.md`, `written-world/SPEC.md`),
since those had independently drifted out of sync with the real code —
see the commit history around 2026-08-31 for what that looked like.

- `src/DMML/Json.hs` — the wire-format `*Input` types (`CommitInput`,
  `MachineInput`, `ReferenceInput`, `UpdateInput`, ...) with `aeson`
  `FromJSON` instances matching the real JSON shape field-for-field.
- `src/DMML/Ast.hs` — the validated target types (`CommitStmt`,
  `MachineStmt`, ...), translated from `ast.rs`.
- `src/DMML/FromJson.hs` — the validation and AST-construction logic
  translated from `from_json.rs`: identifier/node-ref lexical checks,
  the empty-commit and duplicate-fact rejections, `commitFromJson`/
  `machineFromJson`/`referenceFromJson`/`updateFromJson`.
- `app/Main.hs` — the Haskell mirror of
  `dmml/examples/agent_authoring_demo.rs`'s three cases, run against
  this translation and checked to produce the same accept/reject
  outcomes (and, for the duplicate-fact case, the same error text) as
  the real compiled Rust binary.

## What this is not

Not a parser generator, not wired into any runtime, and it does not
implement `validate.rs`'s self-declaration/range checks or
`interpret.rs`'s materialization — only what `from_json.rs` itself does:
JSON → AST, with the shape and lexical checks that entry point performs
before anything reaches the interpreter. If `from_json.rs` changes,
this drifts out of sync exactly the way `GRAMMAR.md` did — there's no
mechanism here to prevent that.

## Building

Depends only on `aeson`, `text`, and `containers`, all available via
`apt` (`libghc-aeson-dev`) on the toolchain this was built against
(GHC 9.4.7) — no `cabal update`/Hackage access needed.

```sh
ghc -isrc -iapp -O0 -Wall app/Main.hs -o demo
./demo
```

## Versioning — v0.1.0, the surface `written-world` builds on

`written-world/cli/cabal.project` pins this package as a
`source-repository-package` by commit, per that repo's own discipline
("bump this tag deliberately... never floating on a branch"). Jason's
call (2026-09-05): stop letting that pin track a moving commit and
version deliberately instead, so `written-world` (and any future
in-world generative agent — see `written-world/dev-journal/2026-09-05-
pantheon-demiurge-lineage.md`) has a real, stable foundation to build
on rather than one that can shift under it session to session.

**v0.1.0 = commit `8c93f82`.** Covers everything through the guard
evaluator, governed arbitration (multi-valued `Alternatives` +
`findGoverningMachine`), the template/guard-eligibility rendering
pipeline (`eligibleTemplates`, `renderTemplateWith`,
`resolveViaGoverningMachine`), the describability linter, and
`DMML.Guard.availableTransitions` — real, built, and verified against
real content this session (the E1 endurance corpus, fresh zero-literal
worlds, `world-browser-demo`).

**Honest gap: this is a commit-pin milestone, not a real git tag.**
`git push origin v0.1.0` (and a plain branch-ref push used to probe the
same path) returned a real, reproducible `403` for tag refs
specifically while an equivalent branch-ref push succeeded — this
session's push credential can create branches but not tags against
this repo. Recorded here as the milestone marker instead of silently
giving up on versioning altogether. If tag-push access is available to
you: `git tag -a v0.1.0 8c93f82 -m '...' && git push origin v0.1.0`
turns this into a real tag; nothing else about the discipline changes.
Future dmml-hs changes bump this milestone deliberately (a new commit,
named the same way, once `written-world`'s pin is meant to move); the
pin never moves implicitly.
