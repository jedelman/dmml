#!/usr/bin/env python3
"""Score results/dispatch.ndjson against the real Surface parser
(ComplianceCheckInformed.hs), then compare predicate-name choice
between the blind and informed conditions -- the actual question this
checkpoint asks.

Usage:
    python3 score.py
"""
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
DMML_HS = HERE.parent / "dmml-hs"
DISPATCH_PATH = HERE / "results" / "dispatch.ndjson"
VERDICTS_PATH = HERE / "results" / "verdicts.ndjson"
REPORT_PATH = HERE / "results" / "report.md"


def build_checker() -> Path:
    binary = DMML_HS / "compliance-check-informed"
    subprocess.run(
        ["ghc", "-isrc", "-iapp", "-O0", "app/ComplianceCheckInformed.hs", "-o", str(binary)],
        cwd=DMML_HS,
        check=True,
        capture_output=True,
        text=True,
    )
    return binary


def run_checker(binary: Path, dispatch_text: str) -> str:
    result = subprocess.run([str(binary)], input=dispatch_text, capture_output=True, text=True, check=True)
    if result.stderr.strip():
        print(result.stderr, file=sys.stderr)
    return result.stdout


def predicate_for_shrine_incense(facts: list[str]) -> str | None:
    """Pull out the predicate name from whichever fact links
    shrine/threshold to offering/incense -- the one relationship every
    reply in this checkpoint is authoring, regardless of what name it
    picked for the predicate."""
    for f in facts:
        subj_pred, _, val = f.partition("=")
        subj, _, pred = subj_pred.partition(".")
        if subj == "shrine/threshold" and val == "offering/incense":
            return pred
    return None


def main() -> int:
    if not DISPATCH_PATH.exists():
        print(f"{DISPATCH_PATH} not found -- run dispatch.py first", file=sys.stderr)
        return 1

    binary = build_checker()
    dispatch_text = DISPATCH_PATH.read_text()
    verdicts_text = run_checker(binary, dispatch_text)
    VERDICTS_PATH.write_text(verdicts_text)

    verdicts = [json.loads(line) for line in verdicts_text.splitlines() if line.strip()]
    dispatch_by_key = {(r["model"], r["id"], r["condition"]): r for r in (json.loads(l) for l in dispatch_text.splitlines() if l.strip())}

    lines = ["# DMML informed-vs-blind authoring checkpoint", ""]
    lines.append(f"{len(verdicts)} dispatches scored against the real `DMML.Surface.parseCommitSurface` "
                 "parser. Question: does a materialized world snapshot prevent predicate-name drift?")
    lines.append("")

    lines.append("## Predicate chosen for (shrine/threshold, ?, offering/incense), by condition")
    lines.append("")
    lines.append("| model | condition | outcome | predicate chosen | matches existing ('accepts')? |")
    lines.append("|---|---|---|---|---|")

    # match verdicts back to their (model, id, condition) via dispatch order (score.py preserves order)
    dispatch_records = [json.loads(l) for l in dispatch_text.splitlines() if l.strip()]
    for rec, v in zip(dispatch_records, verdicts):
        pred = predicate_for_shrine_incense(v.get("facts", [])) if v["outcome"] == "accepted" else None
        matches = "yes" if pred == "accepts" else ("no" if pred else "n/a")
        lines.append(f"| {rec['model']} | {rec['condition']} | {v['outcome']} | {pred or '-'} | {matches} |")
    lines.append("")

    blind_preds = [
        predicate_for_shrine_incense(v.get("facts", []))
        for rec, v in zip(dispatch_records, verdicts)
        if rec["condition"] == "blind" and v["outcome"] == "accepted"
    ]
    informed_preds = [
        predicate_for_shrine_incense(v.get("facts", []))
        for rec, v in zip(dispatch_records, verdicts)
        if rec["condition"] == "informed" and v["outcome"] == "accepted"
    ]

    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Blind condition: {len(set(p for p in blind_preds if p))} distinct predicate name(s) "
                 f"chosen across {len(blind_preds)} accepted replies: {sorted(set(p for p in blind_preds if p))}")
    lines.append(f"- Informed condition: {len(set(p for p in informed_preds if p))} distinct predicate name(s) "
                 f"chosen across {len(informed_preds)} accepted replies: {sorted(set(p for p in informed_preds if p))}")
    all_informed_match = all(p == "accepts" for p in informed_preds) and len(informed_preds) > 0
    lines.append(f"- All informed replies matched the existing 'accepts' relation: {all_informed_match}")
    lines.append("")

    redeclares = [(rec, v) for rec, v in zip(dispatch_records, verdicts) if v.get("redeclaresExisting")]
    if redeclares:
        lines.append("## Redeclared an already-declared predicate anyway")
        lines.append("")
        for rec, v in redeclares:
            lines.append(f"- {rec['model']} / {rec['condition']}: redeclared {v['redeclaresExisting']}")
        lines.append("")

    REPORT_PATH.write_text("\n".join(lines))
    print(f"wrote {VERDICTS_PATH}", file=sys.stderr)
    print(f"wrote {REPORT_PATH}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
