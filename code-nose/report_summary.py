#!/usr/bin/env python3
"""Format Stage 2's ranked ndjson as a markdown report for a GitHub
Actions job summary ($GITHUB_STEP_SUMMARY). Advisory only -- this is
never what gates the build; check_baseline.py is.

Usage: python3 report_summary.py scored.ndjson >> "$GITHUB_STEP_SUMMARY"
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

TOP_N = 15


def main() -> None:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else None
    text = path.read_text() if path else sys.stdin.read()
    rows = [json.loads(l) for l in text.splitlines() if l.strip()]

    print("## code-nose findings\n")
    if not rows:
        print("No candidates this run -- either nothing changed that matches "
              "a known pattern, or Stage 1 found nothing. Advisory only; "
              "this never blocks the PR.\n")
        return

    ranked = rows[: TOP_N]
    print(
        f"{len(rows)} candidate(s) scored this run "
        f"(top {len(ranked)} shown). Independent `noul` scores, ranked -- "
        f"never a verdict, never a required check.\n"
    )
    print("| rank | score | cluster | files | claim |")
    print("|---|---|---|---|---|")
    for r in ranked:
        claim_first_line = r["claim"].splitlines()[0]
        if len(claim_first_line) > 140:
            claim_first_line = claim_first_line[:137] + "..."
        files = ", ".join(Path(f).name for f in r["files"])
        prob = f" (p={r['probability']:.2f})" if r.get("probability") is not None else ""
        print(f"| {r['rank']} | {r['score']:.2f}{prob} | {r['cluster']} | `{files}` | {claim_first_line} |")
    print()


if __name__ == "__main__":
    main()
