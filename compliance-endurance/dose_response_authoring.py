#!/usr/bin/env python3
"""Plank 1 dose-response experiment, run against the REAL free-authoring
harness (run.py's run_agent_round), not de-prose. Jason, 2026-09-03,
after de-prose's version came back a null result: "the original
evidence came from the free-authoring harness so we should design the
experiment based on that... when we encountered it we didn't treat it
as evidence but a bug to resolve. Now we need to treat it as evidence."

Reproducibility, not novelty, is the point: this reuses run_agent_round
itself (the exact function behind REPORT.md's original "55 string-
literal facts over 70 characters" finding) unmodified in its accept/
reject mechanics, with one additive, optional constraint -- a real cap
on string-literal length, checked via the same check-string-cap binary
built for the de-prose version, wired in as a THIRD failure mode with
IDENTICAL retry semantics to a parse failure (2 failed attempts and the
agent gives up for the round) so the retry dynamic that produced the
original evidence is unchanged.

70 characters is not arbitrary: it's REPORT.md's own threshold, so
"cap=70" directly operationalizes the same boundary the original
finding was measured against, rather than inventing a new one.

Two conditions, same seed world, same 4-agent roster as the real
endurance run, run back to back: uncapped (replicates the original
mechanism exactly) and cap=70 (the real manipulation). Prediction:
desiring-production account -> real string_cap_hits under the cap
condition (the agents keep trying to write long strings and get
bounced, not cleanly complying) and/or compensatory growth elsewhere
(more facts/nodes) rather than a clean proportional shrink. Deflationary
account -> few/no string_cap_hits, clean shrink, no compensation.

Usage:
    OPENROUTER_API_KEY=... python3 dose_response_authoring.py \
        [--rounds N] [--seed S] [--out-dir results/dose-response-authoring]
"""
import argparse
import json
import os
import random
import sys
from pathlib import Path

import run as base

HERE = Path(__file__).resolve().parent


def build_string_cap():
    string_cap = base.DMML_HS / "check-string-cap"
    r = base.sh("ghc", "-isrc", "-iapp", "-O0", "app/CheckStringCap.hs", "-o", str(string_cap), cwd=str(base.DMML_HS))
    if r.returncode != 0:
        print(r.stdout, r.stderr, file=sys.stderr)
        raise RuntimeError("build failed: app/CheckStringCap.hs")
    return string_cap


def run_condition(api_key, label, rounds, seed, out_dir, validate_bin, render_bin, surface_text,
                   string_cap_bin, max_string_length, log):
    condition_dir = out_dir / label
    condition_dir.mkdir(parents=True, exist_ok=True)
    scratch_dir = condition_dir / "_scratch"
    scratch_dir.mkdir(parents=True, exist_ok=True)
    commits_dir = condition_dir / "commits"
    commits_dir.mkdir(parents=True, exist_ok=True)

    world_files = [base.SEED_GENESIS] + base.MACHINE_FILES
    seq = 1
    totals = {"valid": 0, "invalid": 0, "no_fence": 0, "string_cap_hits": 0, "prompt_tokens": 0, "completion_tokens": 0}
    per_round = []

    for round_no in range(1, rounds + 1):
        log(f"  [{label}] round {round_no}/{rounds}")
        full_snapshot = base.render_snapshot(render_bin, world_files)
        corners = base.sample_corners(full_snapshot, len(base.AGENTS), random.Random(seed * 1000 + round_no))
        node_index = base.machine_defs_by_node(world_files)

        round_accepted = {}
        for agent, (corner_text, corner_nodes) in zip(base.AGENTS, corners):
            machine_text = base.machine_defs_for_corner(node_index, corner_nodes)
            paths, stats = base.run_agent_round(
                api_key, agent, corner_text, corner_nodes, machine_text, surface_text,
                validate_bin, render_bin, world_files, log, scratch_dir=scratch_dir,
                string_cap_bin=string_cap_bin, max_string_length=max_string_length,
            )
            round_accepted[agent["name"]] = paths
            for k in ("valid", "invalid", "no_fence", "string_cap_hits", "prompt_tokens", "completion_tokens"):
                totals[k] += stats[k]
            log(f"    [{agent['name']}] valid={stats['valid']} invalid={stats['invalid']} string_cap_hits={stats['string_cap_hits']}")

        adopted_this_round = 0
        for agent in base.AGENTS:
            for p in round_accepted[agent["name"]]:
                kind = base.classify_file(validate_bin, p)
                final_name = commits_dir / f"{seq:04d}-r{round_no}-{agent['name']}-{kind}.dmml"
                final_name.write_text(p.read_text())
                p.unlink(missing_ok=True)
                world_files.append(final_name)
                adopted_this_round += 1
                seq += 1
        per_round.append({"round": round_no, "adopted": adopted_this_round})

    # Real measurement: every string-literal value across every adopted
    # commit this condition produced, not just whether the cap was hit --
    # gives the actual length distribution, not just a pass/fail count.
    import re

    string_lengths = []
    for f in commits_dir.glob("*.dmml"):
        for m in re.finditer(r'"([^"]*)"', f.read_text()):
            string_lengths.append(len(m.group(1)))

    total_chars = sum(f.stat().st_size for f in commits_dir.glob("*.dmml"))

    return {
        "label": label,
        "max_string_length": max_string_length,
        "totals": totals,
        "per_round": per_round,
        "n_commits_adopted": seq - 1,
        "string_lengths": string_lengths,
        "max_string_seen": max(string_lengths) if string_lengths else 0,
        "over_70_count": sum(1 for l in string_lengths if l > 70),
        "total_bytes_adopted": total_chars,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rounds", type=int, default=5)
    ap.add_argument("--seed", type=int, default=20260903)
    ap.add_argument("--out-dir", type=Path, default=HERE / "results" / "dose-response-authoring")
    ap.add_argument("--cap", type=int, default=70, help="the tightened condition's cap, chars")
    args = ap.parse_args()

    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        print("OPENROUTER_API_KEY not set", file=sys.stderr)
        return 1

    print("[dose-response-authoring] building binaries...", flush=True)
    validate_bin, render_bin, _divergence_bin, _entropy_bin, _checkpoint_bin = base.build_binaries()
    string_cap_bin = build_string_cap()
    surface_text = base.SURFACE_PATH.read_text()

    def log(msg):
        print(msg, flush=True)

    args.out_dir.mkdir(parents=True, exist_ok=True)

    log(f"=== condition 1/2: uncapped (replicates run.py's real mechanism exactly) ===")
    uncapped = run_condition(
        api_key, "uncapped", args.rounds, args.seed, args.out_dir,
        validate_bin, render_bin, surface_text, None, None, log,
    )

    log(f"\n=== condition 2/2: cap={args.cap} (the real manipulation) ===")
    capped = run_condition(
        api_key, f"cap{args.cap}", args.rounds, args.seed, args.out_dir,
        validate_bin, render_bin, surface_text, string_cap_bin, args.cap, log,
    )

    summary = {"uncapped": uncapped, "capped": capped}
    (args.out_dir / "summary.json").write_text(json.dumps(summary, indent=2))

    print()
    print("[dose-response-authoring] done. Summary:")
    for cond in (uncapped, capped):
        print(
            f"  {cond['label']}: {cond['n_commits_adopted']} commits, "
            f"{cond['totals']['string_cap_hits']} string-cap hits, "
            f"max string seen {cond['max_string_seen']} chars, "
            f"{cond['over_70_count']} strings over 70 chars, "
            f"{cond['total_bytes_adopted']} total bytes adopted"
        )
    print(f"  full summary: {args.out_dir / 'summary.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
