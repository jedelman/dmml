# DMML authoring-compliance checkpoint

**Purpose**: before any decision to retire `dmml/src/from_json.rs` as the
production authoring boundary, get a real, checked-against-the-actual-
parser baseline for whether light models can author valid DMML JSON
given only `dmml-agent-nucleus/GRAMMAR.md` as their reference. Not a
proxy metric — the oracle here (`dmml/examples/compliance_check.rs`) is
the exact `from_json::update_from_json` entry point production code
calls today, so a checkpoint run here says something real about whether
the doc GRAMMAR.md fix (2026-08-31) actually made authoring viable, and
gives a pre-retirement baseline to compare any replacement authoring
path against.

## Status: first real checkpoint run, 2026-08-31

15/15 (model, scenario) pairs accepted against the real production
`update_from_json` boundary — see `results/report.md` for the table,
`results/dispatch.ndjson`/`results/verdicts.ndjson` for the raw evidence
behind it. Spot-checked substantively, not just for parse success: the
fact-level-consume scenario's output correctly used `FactRef`'s wildcard
`object`-omitted form and a real `via` citation, across all three
models, not a degenerate but technically-valid shape.

Getting a real run took two fixes, both now in `dispatch.py`:
`GRAMMAR_PATH` pointed one directory too deep (would have crashed
before sending anything), and `glm-5.3-flash`/`gemini-3.7-flash` reject
`reasoning.effort: "none"` outright — verified live against the API,
correcting `written-world/CLAUDE.md`'s note that `glm-5.3-flash`'s
reasoning support "was not yet verified." Both models reason
by default with no way to disable it, and the first attempt at 4000
`max_tokens` let one call's reasoning eat the whole budget before any
content came out (`content: null`, no error) — bumped to 8000 fixed it.

An earlier attempt this session, under auto mode, got blocked by the
permission classifier before any request left the machine; running for
real needed the user to drop auto mode and approve the outbound calls
directly.

## Pipeline

```sh
# 1. Dispatch scenarios.json to each model, capture raw replies.
python3 dispatch.py            # needs OPENROUTER_API_KEY
# -> results/dispatch.ndjson

# 2. Score every reply against the REAL production authoring boundary
#    (from_json::update_from_json, via dmml/examples/compliance_check.rs)
#    and produce a checkpoint report.
python3 score.py
# -> results/verdicts.ndjson, results/report.md
```

`score.py` builds and runs the Rust checker itself (`cargo run --example
compliance_check` inside `../dmml`) — no separate build step needed.

## What's being tested

`scenarios.json` holds five authoring tasks chosen to cover the real
grammar surface `from_json.rs` implements, not just the easy path:

1. a plain commit (`declares` + `facts`, tagged object literals)
2. a batch of two simultaneous commits (tests the `update`/batching
   wrapper, not just a single commit)
3. a fact-level `consumes` retraction plus a `refs.via` citation
4. a machine (states, a guarded transition, an effect)
5. an adversarial prompt phrased in the *retired* schema's vocabulary
   (`produces`/N-Quads/`created_at`) to check whether GRAMMAR.md's
   authority actually overrides a misleading task description, not just
   a neutral one

Each model gets the exact same system prompt: the full, current
`GRAMMAR.md` text plus instructions to answer in one fenced JSON block
shaped as `{"update": [...]}`. `compliance_check.rs` extracts the fence
(reusing `from_json::extract_fenced_block`, the same utility a real
chat-authoring caller would use) and runs the result through
`update_from_json` — nothing hand-relaxed, nothing pre-validated.

Models chosen for this first checkpoint (`dispatch.py`'s `MODELS`):
`google/gemini-3.7-flash` and `z-ai/glm-5.3-flash` (the two named
in-product DMML-authoring targets per `written-world/CLAUDE.md`'s
"In-product authoring agents" section), plus `moonshotai/kimi-k2.5` as a
known-hallucination-prone stress test (its own dispatch-pipeline
warning: "will get things wrong that don't show up as compile errors"
when not given exact signatures). **That warning is specifically about
Kimi authoring code without exact signatures in front of it** (a
different task than DMML content-authoring with GRAMMAR.md as full
context) — worth stating plainly since the 2026-08-31 run scored it
5/5, same as the other two: it isn't evidence the dispatch-pipeline
warning was wrong, only that "hand it a full, current, exact reference
document" is exactly the condition that warning says Kimi needs and
this checkpoint's prompt provides.

## Reading the report

`report.md` gives pass rate by model and by scenario, plus every
non-accepted reply's real `from_json.rs` error text (not a paraphrase).
A `rejected` outcome (valid JSON, invalid DMML content) and an
`unparseable` outcome (no valid JSON found at all, fenced or not) are
kept distinct — they imply different fixes: a `rejected` case is
GRAMMAR.md or the model's understanding of a specific rule; an
`unparseable` case is more often a model that didn't follow the
fence-and-shape instruction at all.

**This checkpoint is scoped to the `update_from_json` entry point only**
— `commit_from_json`/`machine_from_json` are subsumed by it (a batch of
one), but standalone `reference_from_json` output (which never appears
inside an `UpdateInput`) is not exercised here.

## What this checkpoint does NOT decide

A green checkpoint here says light models can produce valid
`from_json.rs`-shaped JSON against current docs — it says nothing about
whether the described "retire `from_json.rs` as the production
authoring boundary" swap itself is safe: that also needs a real design
for what replaces it (`server/`'s wasm32 target, `mcp-server`'s call
sites, and `validate.rs`/`interpret.rs` downstream all currently assume
exactly `from_json.rs`'s AST shape), a migration plan, and its own test
coverage. This is the first, narrowest input to that larger decision,
not a substitute for it.
