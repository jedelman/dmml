#!/usr/bin/env python3
"""Phase 2 of the endurance test: the same 4-agent, corner-sampled
authoring loop as run.py, but corner sampling and cross-agent
convergence now go through REAL git -- each agent is its own git repo
and branch, cloned from a shared genesis, and every round ends with a
full-mesh sync via the ACTUAL sync-spike broker (real git fetch, real
git merge-base, real git merge, real sequence-renamed minted contests)
instead of Python directly writing files into one shared directory.

Phase 1 (run.py) tested the Contested primitive's CONTENT logic in
isolation. This tests the actual sync MECHANISM it's meant to run
inside -- dogfooding broker.sh under real multi-agent load for the
first time since the 2-player setup-demo.sh spike.

Sync topology: full mesh every round (every one of the 4*(4-1)=12
ordered pairs runs a real broker.sh sync each round) -- chosen over a
partial/gossip schedule to validate the real mechanism first. Made safe
by dmml's check-divergence dedup fix (98bd5b4): a Contest minted by an
earlier pairwise sync this round is never re-minted by a later one,
verified directly against the exact failure case that motivated it
before this harness was built on top of it.

Usage:
    OPENROUTER_API_KEY=... python3 run_git_sync.py [--rounds N] [--seed S]
"""
import argparse
import json
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import run as base  # reuse dispatch/corner-sampling/classification helpers

HERE = Path(__file__).resolve().parent
DMML_HS = HERE.parent / "dmml-hs"
BROKER_SH = HERE.parent.parent / "written-world" / "sync-spike" / "broker" / "broker.sh"
RESULTS_DIR = HERE / "results" / "git-sync"

AGENTS = base.AGENTS  # kimi, deepseek, glm, deepseek2 (post roster-swap)


def sh(*args, **kwargs):
    return subprocess.run(list(args), capture_output=True, text=True, **kwargs)


def git(repo, *args):
    r = sh("git", "-C", str(repo), *args)
    if r.returncode != 0:
        raise RuntimeError(f"git -C {repo} {' '.join(args)} failed:\n{r.stdout}\n{r.stderr}")
    return r.stdout


def setup_repos(workdir):
    genesis = workdir / "genesis"
    genesis.mkdir()
    (genesis / "commits").mkdir()
    shutil.copy(base.SEED_GENESIS, genesis / "commits" / "0000-seed-genesis.dmml")
    for i, f in enumerate(base.MACHINE_FILES, start=1):
        shutil.copy(f, genesis / "commits" / f"000{i}-seed-{f.stem}.dmml")
    sh("git", "-C", str(genesis), "init", "--quiet", "--initial-branch=genesis")
    sh("git", "-C", str(genesis), "config", "user.email", "endurance@example.com")
    sh("git", "-C", str(genesis), "config", "user.name", "genesis")
    sh("git", "-C", str(genesis), "add", "commits")
    sh("git", "-C", str(genesis), "commit", "--quiet", "-m", "seed genesis")

    repos = {}
    for agent in AGENTS:
        name = agent["name"]
        d = workdir / name
        branch = f"player/{name}"
        sh("git", "clone", "--quiet", str(genesis), str(d))
        sh("git", "-C", str(d), "checkout", "--quiet", "-b", branch)
        sh("git", "-C", str(d), "config", "user.email", "endurance@example.com")
        sh("git", "-C", str(d), "config", "user.name", name)
        repos[name] = {"dir": d, "branch": branch}
    return repos


def agent_commit_files(repo_dir):
    return sorted((repo_dir / "commits").glob("*.dmml"))


def adopt_into_repo(repo_dir, agent_name, round_no, accepted_paths, validate_bin, log):
    """git add + commit each accepted file into this agent's own repo,
    one file per commit -- the append-only, one-file-per-commit
    convention every other part of this project already follows."""
    existing = len(agent_commit_files(repo_dir))
    for i, p in enumerate(accepted_paths, start=1):
        kind = base.classify_file(validate_bin, p)
        seq = existing + i
        final_name = f"{seq:04d}-r{round_no}-{agent_name}-{kind}.dmml"
        dest = repo_dir / "commits" / final_name
        dest.write_text(p.read_text())
        p.unlink(missing_ok=True)
        git(repo_dir, "add", f"commits/{final_name}")
        git(repo_dir, "commit", "--quiet", "-m", f"{agent_name}: {final_name}")
        log(f"    [{agent_name}] committed {final_name}")


MINT_SUBJ_PRED_RE = re.compile(r"^  (\S+) \. (\S+): ")


def full_mesh_sync(repos, validate_bin, divergence_bin, round_no, log):
    env = dict(os.environ)
    env["DMML_VALIDATOR"] = str(validate_bin)
    env["DMML_DIVERGENCE_CHECK"] = str(divergence_bin)
    names = list(repos.keys())
    total_contests = 0
    # Only pairs ACTUALLY MINTED this round -- not "every Contest that
    # currently exists anywhere in the repo," which would re-count an
    # unresolved contest from an earlier round as freshly contested
    # again every single round after it (a real bug found running
    # this: an unresolved round-2 contest still sitting there,
    # unresolved, in round 3 isn't a NEW divergence, and treating it as
    # one triggered a false oscillation-thrash stop with only one real
    # underlying dispute). Parsed straight from broker.sh's own mint
    # output, same regex phase 1's run.py already uses for this.
    minted_this_round = set()
    for a in names:
        for b in names:
            if a == b:
                continue
            r = sh(
                "bash", str(BROKER_SH), b, str(repos[b]["dir"]), repos[b]["branch"],
                cwd=str(repos[a]["dir"]), env=env,
            )
            out = r.stdout
            minted = [l for l in out.splitlines() if l.startswith("DIVERGENCE minted as content:")]
            if r.returncode != 0:
                # stderr matters here -- a real crash once hid entirely
                # in stderr while stdout looked like an ordinary,
                # successful sync truncated mid-stream. Always log both.
                log(f"    [sync] {a} <- {b}: FAILED (exit {r.returncode})\n"
                    f"      stdout:\n{out}\n      stderr:\n{r.stderr}")
                continue
            if minted:
                total_contests += len(minted)
                log(f"    [sync] {a} <- {b}: {len(minted)} contest(s) minted")
                for line in out.splitlines():
                    m = MINT_SUBJ_PRED_RE.match(line)
                    if m:
                        minted_this_round.add((m.group(1), m.group(2)))
            else:
                log(f"    [sync] {a} <- {b}: ok, no divergence")
    return total_contests, minted_this_round


def render_repo_snapshot(render_bin, repo_dir):
    files = agent_commit_files(repo_dir)
    r = sh(str(render_bin), *[str(f) for f in files])
    if r.returncode != 0:
        raise RuntimeError(f"render-snapshot failed on {repo_dir}:\n{r.stdout}")
    return r.stdout


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rounds", type=int, default=20)
    ap.add_argument("--seed", type=int, default=20260902)
    args = ap.parse_args()

    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        print("OPENROUTER_API_KEY not set", file=sys.stderr)
        return 1

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    validate_bin, render_bin, divergence_bin = base.build_binaries()
    surface_text = base.SURFACE_PATH.read_text()

    round_log = []

    def log(msg):
        print(msg)
        round_log.append(msg)

    workdir = Path(tempfile.mkdtemp(prefix="endurance-git-sync-"))
    log(f"=== git-sync endurance test: {len(AGENTS)} agents, up to {args.rounds} rounds, "
        f"full-mesh sync, workdir={workdir} ===")
    repos = setup_repos(workdir)

    contest_history = []  # (round, subj, pred)
    thrash_reason = None
    report_rounds = []

    for round_no in range(1, args.rounds + 1):
        log(f"\n--- round {round_no} ---")
        # All 4 repos are content-equivalent after the previous round's
        # full-mesh sync (or, in round 1, all freshly cloned from the
        # same genesis) -- verify that rather than assume it, then
        # sample ONE canonical set of 4 corners from it, same partition
        # logic phase 1 uses, just backed by a real per-repo
        # materialization instead of a Python-level file list.
        file_sets_start = {name: {f.name for f in agent_commit_files(r["dir"])} for name, r in repos.items()}
        if len(set(map(frozenset, file_sets_start.values()))) != 1:
            log(f"  WARNING: repos entered round {round_no} NOT converged: "
                f"{ {n: len(s) for n, s in file_sets_start.items()} }")
        canonical_repo = repos[AGENTS[0]["name"]]["dir"]
        full_snapshot = render_repo_snapshot(render_bin, canonical_repo)
        corners = base.sample_corners(full_snapshot, len(AGENTS), random.Random(args.seed * 1000 + round_no))

        round_accepted = {}
        round_stats = {}
        for agent, (corner_text, corner_nodes) in zip(AGENTS, corners):
            name = agent["name"]
            repo_dir = repos[name]["dir"]
            world_files = agent_commit_files(repo_dir)
            node_index = base.machine_defs_by_node(world_files)
            machine_text = base.machine_defs_for_corner(node_index, corner_nodes)
            log(f"  [{name}] corner: {len(corner_nodes)} node(s), own repo has {len(world_files)} file(s)")
            paths, stats = base.run_agent_round(
                api_key, agent, corner_text, corner_nodes, machine_text, surface_text,
                validate_bin, render_bin, world_files, log, scratch_dir=RESULTS_DIR,
            )
            round_accepted[name] = paths
            round_stats[name] = stats
            adopt_into_repo(repo_dir, name, round_no, paths, validate_bin, log)

        total_attempts = sum(s["valid"] + s["invalid"] + s["no_fence"] for s in round_stats.values())
        total_valid = sum(s["valid"] for s in round_stats.values())
        total_invalid = sum(s["invalid"] for s in round_stats.values())
        fail_rate = total_invalid / total_attempts if total_attempts else 0.0
        log(f"  round {round_no} authoring totals: valid={total_valid} invalid={total_invalid} "
            f"attempts={total_attempts} fail_rate={fail_rate:.2f}")

        log(f"  full-mesh sync ({len(AGENTS) * (len(AGENTS) - 1)} pairwise broker.sh runs)...")
        new_contests, minted_this_round = full_mesh_sync(repos, validate_bin, divergence_bin, round_no, log)

        # Real convergence check -- with a correct dedup fix and full
        # mesh, every repo should end this round with the identical file
        # set. Verify it, don't assume it.
        file_sets = {name: {f.name for f in agent_commit_files(r["dir"])} for name, r in repos.items()}
        converged = len(set(map(frozenset, file_sets.values()))) == 1
        if converged:
            log(f"  converged: all {len(repos)} repos agree, {len(next(iter(file_sets.values())))} file(s) each")
        else:
            log(f"  WARNING: repos did NOT converge after full mesh sync: "
                f"{ {n: len(s) for n, s in file_sets.items()} }")

        observer_dir = repos[AGENTS[0]["name"]]["dir"]
        for (s, p) in minted_this_round:
            contest_history.append((round_no, s, p))

        repeat_pairs = {}
        for (r, s, p) in contest_history:
            repeat_pairs.setdefault((s, p), set()).add(r)
        oscillating = [(s, p, sorted(rs)) for (s, p), rs in repeat_pairs.items() if len(rs) >= 2]

        report_rounds.append({
            "round": round_no, "stats": round_stats, "new_contests_minted": new_contests,
            "fail_rate": fail_rate, "converged": converged,
        })

        (RESULTS_DIR / f"snapshot-after-round{round_no}.txt").write_text(
            render_repo_snapshot(render_bin, observer_dir)
        )

        if oscillating:
            thrash_reason = f"same (subject, predicate) contested across multiple rounds: {oscillating}"
        elif not converged:
            thrash_reason = f"round {round_no}: repos failed to converge after full-mesh sync (real sync bug, not content thrash)"
        elif total_attempts and fail_rate >= 0.5:
            thrash_reason = f"round {round_no} failure rate {fail_rate:.2f} (>= 0.5)"

        if thrash_reason:
            log(f"\n*** STOPPING after round {round_no}: {thrash_reason} ***")
            break

    observer_dir = repos[AGENTS[0]["name"]]["dir"]
    final_snapshot = render_repo_snapshot(render_bin, observer_dir)
    (RESULTS_DIR / "snapshot-final.txt").write_text(final_snapshot)
    # Persist the observer repo's flat commit files as real evidence
    # (not the .git directory itself -- this harness's own repo doesn't
    # need nested git history, just what was actually authored/merged).
    evidence_dir = RESULTS_DIR / "commits"
    if evidence_dir.exists():
        shutil.rmtree(evidence_dir)
    shutil.copytree(observer_dir / "commits", evidence_dir)

    report = {
        "rounds_run": len(report_rounds),
        "rounds_requested": args.rounds,
        "thrash_reason": thrash_reason,
        "final_file_count": len(agent_commit_files(observer_dir)),
        "rounds": report_rounds,
    }
    (RESULTS_DIR / "report.json").write_text(json.dumps(report, indent=2))
    (RESULTS_DIR / "log.txt").write_text("\n".join(round_log))
    log(f"\n=== done: {len(report_rounds)} round(s), thrash={thrash_reason!r} ===")
    log(f"(workdir {workdir} left on disk for inspection)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
