#!/usr/bin/env python3
"""Score results/dispatch.ndjson's JSON-format records against the real
Rust `update_from_json` boundary and its surface-format records against
the real `DMML.Surface.parseCommitSurface` parser, then produce ONE
combined side-by-side report -- the actual head-to-head this checkpoint
exists for.

Usage:
    python3 score.py

Requires dispatch.py to have already produced results/dispatch.ndjson.
Writes:
  results/verdicts.ndjson  -- one verdict per (model, scenario, format)
  results/report.md        -- side-by-side pass-rate comparison
"""
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
DMML_CRATE = HERE.parent / "dmml"
DMML_HS = HERE.parent / "dmml-hs"
DISPATCH_PATH = HERE / "results" / "dispatch.ndjson"
VERDICTS_PATH = HERE / "results" / "verdicts.ndjson"
REPORT_PATH = HERE / "results" / "report.md"


def run_json_checker(records: list[dict]) -> list[dict]:
    input_text = "\n".join(json.dumps(r) for r in records)
    result = subprocess.run(
        ["cargo", "run", "--quiet", "--example", "compliance_check"],
        cwd=DMML_CRATE,
        input=input_text,
        capture_output=True,
        text=True,
        check=True,
    )
    if result.stderr.strip():
        print(result.stderr, file=sys.stderr)
    verdicts = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    for v in verdicts:
        v["format"] = "json"
    return verdicts


def run_surface_checker(records: list[dict]) -> list[dict]:
    binary = DMML_HS / "compliance-check-surface"
    subprocess.run(
        ["ghc", "-isrc", "-iapp", "-O0", "app/ComplianceCheckSurface.hs", "-o", str(binary)],
        cwd=DMML_HS,
        check=True,
        capture_output=True,
        text=True,
    )
    input_text = "\n".join(json.dumps(r) for r in records)
    result = subprocess.run(
        [str(binary)],
        input=input_text,
        capture_output=True,
        text=True,
        check=True,
    )
    if result.stderr.strip():
        print(result.stderr, file=sys.stderr)
    verdicts = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    for v in verdicts:
        v["format"] = "surface"
    return verdicts


def main() -> int:
    if not DISPATCH_PATH.exists():
        print(f"{DISPATCH_PATH} not found -- run dispatch.py first", file=sys.stderr)
        return 1

    records = [json.loads(l) for l in DISPATCH_PATH.read_text().splitlines() if l.strip()]
    json_records = [r for r in records if r["format"] == "json"]
    surface_records = [r for r in records if r["format"] == "surface"]

    print(f"scoring {len(json_records)} JSON-format records ...", file=sys.stderr)
    json_verdicts = run_json_checker(json_records)
    print(f"scoring {len(surface_records)} surface-format records ...", file=sys.stderr)
    surface_verdicts = run_surface_checker(surface_records)

    all_verdicts = json_verdicts + surface_verdicts
    VERDICTS_PATH.write_text("\n".join(json.dumps(v) for v in all_verdicts))

    # side-by-side: keyed by (model, id) -> {"json": outcome, "surface": outcome}
    combined = defaultdict(dict)
    scenario_titles = {}
    for v in all_verdicts:
        key = (v["model"], v["id"])
        combined[key][v["format"]] = v["outcome"]
        scenario_titles[v["id"]] = v["scenario"]

    models = sorted({m for m, _ in combined})
    ids = sorted({i for _, i in combined})

    lines = ["# DMML head-to-head: JSON vs. Surface syntax", ""]
    lines.append(f"{len(json_records)} JSON dispatches + {len(surface_records)} Surface dispatches, "
                  "same models, same tightened scenarios, scored against each format's own real parser.")
    lines.append("")

    lines.append("## Side-by-side outcome per (model, scenario)")
    lines.append("")
    lines.append("| model | scenario | JSON | Surface |")
    lines.append("|---|---|---|---|")
    for model in models:
        for sid in ids:
            outcomes = combined[(model, sid)]
            j = outcomes.get("json", "MISSING")
            s = outcomes.get("surface", "MISSING")
            mark = "" if j == s == "accepted" else "  <-- DIVERGES" if j != s else ""
            lines.append(f"| {model} | {scenario_titles.get(sid, sid)} | {j} | {s}{mark} |")
    lines.append("")

    lines.append("## Aggregate pass rate")
    lines.append("")
    lines.append("| format | accepted | rejected | unparseable | pass rate |")
    lines.append("|---|---|---|---|---|")
    for fmt, vs in (("json", json_verdicts), ("surface", surface_verdicts)):
        acc = sum(1 for v in vs if v["outcome"] == "accepted")
        rej = sum(1 for v in vs if v["outcome"] == "rejected")
        unp = sum(1 for v in vs if v["outcome"] == "unparseable")
        rate = f"{acc / len(vs):.0%}" if vs else "n/a"
        lines.append(f"| {fmt} | {acc} | {rej} | {unp} | {rate} |")
    lines.append("")

    divergences = [
        (model, sid, combined[(model, sid)])
        for model in models
        for sid in ids
        if combined[(model, sid)].get("json") != combined[(model, sid)].get("surface")
    ]
    if divergences:
        lines.append("## Divergences (JSON and Surface disagreed on the same model+scenario)")
        lines.append("")
        for model, sid, outcomes in divergences:
            lines.append(f"- **{model} / {scenario_titles.get(sid, sid)}**: json={outcomes.get('json')}, surface={outcomes.get('surface')}")
        lines.append("")
    else:
        lines.append("## No divergences")
        lines.append("")
        lines.append("Every (model, scenario) pair got the same accept/reject outcome in both formats.")
        lines.append("")

    REPORT_PATH.write_text("\n".join(lines))
    print(f"wrote {VERDICTS_PATH}", file=sys.stderr)
    print(f"wrote {REPORT_PATH}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
