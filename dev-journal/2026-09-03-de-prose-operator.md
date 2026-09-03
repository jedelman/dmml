# De-prose: refining plain prose into DMML, and three real bugs it found

Jason's framing, taken literally rather than as metaphor: "a de-prose
operator should take plain prose and a dmml world (which may be empty)
and extract dmml from it into the world... it may be worth it to
consider prose simply as malformed dmml." Built as a refinement pipeline
(`compliance-endurance/deprose.py`), reusing existing project machinery
at each stage rather than one black-box LLM call:

1. **Ore extraction** — one LLM call given the world's current declared
   vocabulary (rendered via `render-snapshot`) plus the raw prose,
   producing candidate commit text. The world-context framing is what
   folds the original dedup question ("if two agents read the same blog
   post... how do we mint that they are the same node?") into extraction
   itself, via an explicit reuse rule, rather than a separate pass.
2. **Smelting** — a bounded parse-repair loop against the real
   `validate-commit` parser, reusing `run.py`'s established
   `RETRY_PROMPT` idiom.
3. **Assay** — self-declaration check (`check-declared`, new) plus
   `DMML.Retroconsistency`'s whole-tree consistency gate (`retro-gate`)
   — a de-prosed fact is exactly as external to the deterministic core as
   a retro-implied one, and gets the same check before being trusted.
4. **Deposit** — only a candidate clearing both 2 and 3 is written; a
   rejected candidate is reported, never forced in.

This was tried for real, twice, and found three real bugs along the way.
None of them were found by reasoning about the design — all three
surfaced by actually running the pipeline against real prose.

## Bug 1: no self-declaration checker existed anywhere in dmml-hs

The first real run used `forge \`locatedIn\` ashgrove` while only ever
declaring `locatedOn` — accepted outright by `validate-commit` (shape
only) and untouched by `gateConsistentTree` (a different property:
negated-guard consistency, not vocabulary closure). `sync-spike/
README.md` had already disclosed this as a theoretical gap
("`validate.rs`/`interpret.rs` aren't ported to `dmml-hs` yet"); de-prose
turned it from disclosed-but-never-triggered into a real, silent content
bug.

Fixed with the smallest real check, not a port of the production crate's
own two-pass `validate_self_declared`: `DMML.SelfDeclaration.
undeclaredPredicates` (every fact's predicate must be a key in
`snapshotDeclared`), wired into a new `check-declared` CLI
(`app/CheckDeclared.hs`). One real false positive found and fixed along
the way: `"a"` (RdfType/Turtle-sugar) initially flagged as undeclared,
since `DMML.Materialize`'s `predText RdfType = "a"` makes it
indistinguishable from a literal `PredIdent "a"` once materialized.
Verified against the real 200-commit E1 corpus (30 real `. a = ` facts,
zero ever preceded by `declare relation a`) before adding the exemption.

**Running the new checker against the same E1 corpus — already
committed, already reported as a fully successful endurance run — found
9 genuinely undeclared predicates that slipped through undetected the
whole time**: `gatheredAt`, `unbehest`, `mark`, `discoveredBy`,
`fallOn`, `gatheringState`, `offeringsGathered`, `guidedBy`, `focus`.
Spot-checked `guidedBy` directly: `ritual/waterRite . guidedBy =
npc/fordWarden` in `0196-r18-deepseek2-commit.dmml`, never declared
anywhere in the corpus. This doesn't retroactively break anything the
E1 report claimed (it never claimed vocabulary closure), but it's a
real gap in what "fully successful" covered, and belongs in its own
follow-up, not buried here.

## Bug 2: same-run candidate explosion, with inconsistent node identity

The first real end-to-end de-prose run, given one short single-scene
prose passage, asked the model for "one or more fenced code blocks" and
got back 6 near-paraphrases of the *same* content as separate
"candidates." 5 were accepted (1 rejected on bug 1's undeclared
predicates). Diffing the accepted files: candidates 5 and 6 were
byte-identical; the rest differed only cosmetically. Worse than plain
duplication: the paraphrases used **two incompatible node identities for
the same referents** — `Mara :: a Person` / `forge_1` in three files,
`Mara :: a Blacksmith` / `forge/1` in the other two — both now live in
the same world simultaneously. This is exactly the cross-agent dedup
failure mode Jason's original question named, except happening *within*
a single run rather than across two.

Root cause: nothing constrained "one or more commits" to mean
*independent* content — the model treated it as license to offer
alternate drafts. Fixed in the prompt (`DEPROSE_SYSTEM_PROMPT`): default
to exactly one commit per de-prose call, capturing everything the prose
asserts (a commit can hold many facts), and reserve multiple blocks for
prose that plainly describes multiple separate, unrelated scenes — never
alternate phrasings of the same content. Re-run against the same source
text: 1 clean candidate, no duplication.

## Bug 3: cross-run file-overwrite / silent data loss

The pipeline is explicitly designed for incremental use — feed a prior
run's real output back in as `--world-dir` for the next passage, which
is exactly how the dedup question gets tested for real. Output filenames
were numbered purely by within-run candidate index
(`{i:03d}-deprosed.dmml`), with no awareness of what was already in
`out_dir`. Running a second prose passage with `--world-dir` and
`--out-dir` both pointing at the first run's output directory (the
natural incremental pattern) silently overwrote `001-deprosed.dmml` with
the second run's `001-deprosed.dmml`, destroying the first run's real
content outright — not flagged, not warned, just gone. Caught only
because the accepted file's content was inspected by hand right after;
in an unattended run this would be silent.

Fixed by scanning `out_dir` for existing `NNN-deprosed.dmml` files and
continuing the index from `max(existing) + 1`, so incremental runs into
the same directory append rather than collide. A related, smaller issue
fixed in the same pass: `declare_repair_loop` (added alongside bug 1's
fix, see below) could abort the whole assay stage outright if a
declare-focused repair edit itself introduced a parse error (a real case
hit here: the model moved `declare` lines outside the `commit` block) —
now falls back to one pass of the existing parse-repair loop to patch
that specific error and keeps the remaining declare-repair budget
instead of giving up on first collision.

## A fourth, smaller gap noticed and closed in passing

The self-declaration check (bug 1's fix) had no repair path — a
candidate with a merely-forgotten `declare` line was rejected outright,
same as a genuinely irreconcilable fact, even though "forgot to declare
something you already used" is a mechanical omission, not an epistemic
conflict (unlike a `retro-gate` failure, which legitimately deserves
outright rejection rather than repair). Added `declare_repair_loop`,
mirroring the existing parse-repair loop's structure and retry budget,
feeding the real `check-declared` output back to the model.

## The actual dedup/reuse test, run for real

With bugs 1–3 fixed, ran the originally-intended test: a second, related
prose passage ("The blacksmith of Ashgrove has a new problem...", "Her
father used to warn her...") de-prosed against the first passage's real
committed output as `--world-dir`. Result: `mara . shortOn = "ore"` —
the model correctly resolved both "the blacksmith of Ashgrove" and "her
father" back to the existing `mara`/`corwin` nodes rather than minting
new ones. This is real, positive evidence that the reuse mechanism (an
explicit rule in the extraction prompt, backed by rendering the current
world as context) works for the referring-expression case the original
question posed.

**Disclosed limitation, not smoothed over**: the same second pass
extracted almost nothing else from the passage — the merchant's offer,
the triple price, the "squeezed by a monopoly supplier" framing were all
dropped rather than turned into new facts referencing the existing
nodes. Whether this is desirably conservative (Rule 3's "extract only
what the prose actually asserts" arguably makes "squeezedBy" an
interpretive gloss, not a literal assertion) or under-extraction is not
settled by this one run. Confirmed reuse works; did not confirm the
pipeline reliably extracts *rich* new content that references existing
nodes — that needs more real runs, ideally with a stronger model or a
sharper extraction prompt, before treating de-prose as production-ready
for anything beyond simple cases.

## What's still open

- De-prose's extraction completeness (the limitation just above) is
  untested past this one passage pair.
- The 9 undeclared predicates found in the already-committed E1 corpus
  need their own follow-up — not fixed here, just found and disclosed.
- `check-declared` is built as a standalone local helper in
  `deprose.py` rather than folded into `run.py`'s shared
  `build_binaries()`, specifically to avoid re-touching that function's
  3 existing call sites again; worth reconsidering once `check-declared`
  has more than one caller.
- `results/deprose-test/` is real evidence (kept, not scratch) — two
  real prose sources and their real, currently-accepted output.
