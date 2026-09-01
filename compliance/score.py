#!/usr/bin/env python3
"""Run results/dispatch.ndjson through the real production authoring
boundary (dmml/examples/compliance_check.rs, which calls
from_json::update_from_json directly) and produce a checkpoint report.

Usage:
    python3 score.py

Requires dispatch.py to have already produced results/dispatch.ndjson.
Builds the Rust checker if needed, then writes:
  results/verdicts.ndjson  -- one verdict per (model, scenario)
  results/report.md        -- pass-rate table + failure detail
"""
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
DMML_CRATE = HERE.parent / "dmml"
DISPATCH_PATH = HERE / "results" / "dispatch.ndjson"
VERDICTS_PATH = HERE / "results" / "verdicts.ndjson"
REPORT_PATH = HERE / "results" / "report.md"


def run_checker(dispatch_text: str) -> str:
    result = subprocess.run(
        ["cargo", "run", "--quiet", "--example", "compliance_check"],
        cwd=DMML_CRATE,
        input=dispatch_text,
        capture_output=True,
        text=True,
        check=True,
    )
    if result.stderr.strip():
        print(result.stderr, file=sys.stderr)
    return result.stdout


def main() -> int:
    if not DISPATCH_PATH.exists():
        print(f"{DISPATCH_PATH} not found -- run dispatch.py first", file=sys.stderr)
        return 1

    dispatch_text = DISPATCH_PATH.read_text()
    verdicts_text = run_checker(dispatch_text)
    VERDICTS_PATH.write_text(verdicts_text)

    verdicts = [json.loads(line) for line in verdicts_text.splitlines() if line.strip()]

    by_model = defaultdict(list)
    by_scenario = defaultdict(list)
    for v in verdicts:
        by_model[v["model"]].append(v)
        by_scenario[v["scenario"]].append(v)

    lines = ["# DMML authoring-compliance checkpoint", ""]
    lines.append(f"{len(verdicts)} (model, scenario) dispatches scored against the real "
                  "`from_json::update_from_json` production boundary.")
    lines.append("")

    lines.append("## Pass rate by model")
    lines.append("")
    lines.append("| model | accepted | rejected | unparseable | pass rate |")
    lines.append("|---|---|---|---|---|")
    for model, vs in sorted(by_model.items()):
        acc = sum(1 for v in vs if v["outcome"] == "accepted")
        rej = sum(1 for v in vs if v["outcome"] == "rejected")
        unp = sum(1 for v in vs if v["outcome"] == "unparseable")
        rate = f"{acc / len(vs):.0%}" if vs else "n/a"
        lines.append(f"| {model} | {acc} | {rej} | {unp} | {rate} |")
    lines.append("")

    lines.append("## Pass rate by scenario")
    lines.append("")
    lines.append("| scenario | accepted | rejected | unparseable |")
    lines.append("|---|---|---|---|")
    for scenario, vs in sorted(by_scenario.items()):
        acc = sum(1 for v in vs if v["outcome"] == "accepted")
        rej = sum(1 for v in vs if v["outcome"] == "rejected")
        unp = sum(1 for v in vs if v["outcome"] == "unparseable")
        lines.append(f"| {scenario} | {acc} | {rej} | {unp} |")
    lines.append("")

    failures = [v for v in verdicts if v["outcome"] != "accepted"]
    if failures:
        lines.append("## Failure detail")
        lines.append("")
        for v in failures:
            lines.append(f"### {v['model']} / {v['scenario']} ({v['outcome']}, fenced={v['fenced']})")
            for err in v["errors"]:
                lines.append(f"- {err}")
            lines.append("")

    REPORT_PATH.write_text("\n".join(lines))
    print(f"wrote {VERDICTS_PATH}", file=sys.stderr)
    print(f"wrote {REPORT_PATH}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
