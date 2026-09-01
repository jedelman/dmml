#!/usr/bin/env python3
"""Phase 3 of the endurance test: the same 4-agent, corner-sampled
authoring loop as run.py/run_git_sync.py, but now modeling the ACTUAL
scenario the 4 dispatch models represent -- one player's own local
authoring agents, not 4 independent players -- via the real worktree +
git-hooks mechanism designed and verified in sync-spike/broker/
(pre-merge-commit, post-merge, hook-merge.sh, install-hooks.sh).

Replaces run_git_sync.py's 4-full-clones + 12-pairwise-broker.sh-calls
design with the topology that actually fits worktrees: ONE canonical
repo (the player's own checkout), 4 agent worktrees sharing its object
store and hooks, and a hub-style sync each round -- canonical merges
FROM each agent branch via hook-merge.sh, hooks handle validation and
Contest-minting automatically. No full mesh needed: since every agent
worktree shares the SAME underlying repo, there is exactly one place
("mine") for divergence to be checked against, not four.

Usage:
    OPENROUTER_API_KEY=... python3 run_worktree_sync.py [--rounds N] [--seed S]
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
SYNC_SPIKE_BROKER = HERE.parent.parent / "written-world" / "sync-spike" / "broker"
INSTALL_HOOKS_SH = SYNC_SPIKE_BROKER / "install-hooks.sh"
HOOK_MERGE_SH = SYNC_SPIKE_BROKER / "hook-merge.sh"
RESULTS_DIR = HERE / "results" / "worktree-sync"

AGENTS = base.AGENTS  # kimi, deepseek, mercury, deepseek2


def sh(*args, **kwargs):
    return subprocess.run(list(args), capture_output=True, text=True, **kwargs)


def git(repo, *args):
    r = sh("git", "-C", str(repo), *args)
    if r.returncode != 0:
        raise RuntimeError(f"git -C {repo} {' '.join(args)} failed:\n{r.stdout}\n{r.stderr}")
    return r.stdout


def setup_world(workdir):
    canon = workdir / "canonical"
    canon.mkdir()
    (canon / "commits").mkdir()
    shutil.copy(base.SEED_GENESIS, canon / "commits" / "0000-seed-genesis.dmml")
    for i, f in enumerate(base.MACHINE_FILES, start=1):
        shutil.copy(f, canon / "commits" / f"000{i}-seed-{f.stem}.dmml")
    sh("git", "-C", str(canon), "init", "--quiet", "--initial-branch=player/canonical")
    sh("git", "-C", str(canon), "config", "user.email", "endurance@example.com")
    sh("git", "-C", str(canon), "config", "user.name", "canonical")
    sh("git", "-C", str(canon), "add", "commits")
    sh("git", "-C", str(canon), "commit", "--quiet", "-m", "seed genesis")

    r = sh("bash", str(INSTALL_HOOKS_SH), str(canon))
    if r.returncode != 0:
        raise RuntimeError(f"install-hooks.sh failed:\n{r.stdout}\n{r.stderr}")

    worktrees = {}
    for agent in AGENTS:
        name = agent["name"]
        branch = f"agent/{name}"
        d = workdir / name
        git(canon, "worktree", "add", str(d), "-b", branch)
        sh("git", "-C", str(d), "config", "user.email", "endurance@example.com")
        sh("git", "-C", str(d), "config", "user.name", name)
        worktrees[name] = {"dir": d, "branch": branch}
    return canon, worktrees


def agent_commit_files(repo_dir):
    return sorted((repo_dir / "commits").glob("*.dmml"))


def sync_agent_into_canonical(canon, branch, env, log):
    """hook-merge.sh IS the broker now -- 3 lines (git merge --no-ff
    || git merge --abort), with pre-merge-commit/post-merge doing the
    validation and Contest-minting that used to be broker.sh's own
    ~150 lines of hand-rolled orchestration."""
    r = sh("bash", str(HOOK_MERGE_SH), branch, cwd=str(canon), env=env)
    # Real bug found running this for real: git routes an invoked
    # hook's own stdout through to the CALLING process's stderr, not
    # its stdout -- confirmed directly (post-merge's "DIVERGENCE
    # minted..." line landed in r.stderr, never r.stdout, even on a
    # clean, successful merge). Checking stdout alone silently missed
    # every real mint; a contest was genuinely minted and committed
    # while the log kept reporting "no divergence." Search both.
    combined = r.stdout + r.stderr
    minted = [l for l in combined.splitlines() if l.startswith("DIVERGENCE minted as content:")]
    if r.returncode != 0:
        log(f"    [sync] canonical <- {branch}: REJECTED (exit {r.returncode})\n"
            f"      stdout:\n{r.stdout}\n      stderr:\n{r.stderr}")
        return False, 0, set()
    minted_pairs = set()
    for line in combined.splitlines():
        m = MINT_SUBJ_PRED_RE.match(line)
        if m:
            minted_pairs.add((m.group(1), m.group(2)))
    if minted:
        log(f"    [sync] canonical <- {branch}: ACCEPTED, {len(minted)} contest(s) minted")
    else:
        log(f"    [sync] canonical <- {branch}: ACCEPTED, no divergence")
    return True, len(minted), minted_pairs


MINT_SUBJ_PRED_RE = re.compile(r"^  (\S+) \. (\S+): ")


def render_repo_snapshot(render_bin, repo_dir):
    files = agent_commit_files(repo_dir)
    r = sh(str(render_bin), *[str(f) for f in files])
    if r.returncode != 0:
        raise RuntimeError(f"render-snapshot failed on {repo_dir}:\n{r.stdout}")
    return r.stdout


def refresh_worktree_to_canonical(worktree_dir, canon_branch):
    """Fast-forwards an agent's own worktree branch to match canonical
    at the start of each round -- safe because by the end of the prior
    round every agent's own new commits were already merged INTO
    canonical (this round's sync phase), so canonical is always a real
    superset. Keeps each agent's own mid-round self-view (used by
    run_agent_round's own CONTINUE_PROMPT refresh) consistent with the
    real, current shared state instead of silently staying stale."""
    git(worktree_dir, "merge", "--ff-only", canon_branch)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rounds", type=int, default=20)
    ap.add_argument("--seed", type=int, default=20260903)
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

    workdir = Path(tempfile.mkdtemp(prefix="endurance-worktree-sync-"))
    log(f"=== worktree-sync endurance test: {len(AGENTS)} agents "
        f"({', '.join(a['name'] for a in AGENTS)}), up to {args.rounds} rounds, "
        f"hub topology via hooks, workdir={workdir} ===")
    canon, worktrees = setup_world(workdir)

    env = dict(os.environ)
    env["DMML_VALIDATOR"] = str(validate_bin)
    env["DMML_DIVERGENCE_CHECK"] = str(divergence_bin)

    contest_history = []  # (round, subj, pred), only real mints
    thrash_reason = None
    report_rounds = []

    for round_no in range(1, args.rounds + 1):
        log(f"\n--- round {round_no} ---")
        canon_branch = git(canon, "rev-parse", "--abbrev-ref", "HEAD").strip()
        for agent in AGENTS:
            refresh_worktree_to_canonical(worktrees[agent["name"]]["dir"], canon_branch)

        full_snapshot = render_repo_snapshot(render_bin, canon)
        corners = base.sample_corners(full_snapshot, len(AGENTS), random.Random(args.seed * 1000 + round_no))
        node_index = base.machine_defs_by_node(agent_commit_files(canon))

        round_stats = {}
        round_accepted = {}
        for agent, (corner_text, corner_nodes) in zip(AGENTS, corners):
            name = agent["name"]
            wt_dir = worktrees[name]["dir"]
            world_files = agent_commit_files(wt_dir)
            machine_text = base.machine_defs_for_corner(node_index, corner_nodes)
            log(f"  [{name}] corner: {len(corner_nodes)} node(s), own worktree has {len(world_files)} file(s)")
            paths, stats = base.run_agent_round(
                api_key, agent, corner_text, corner_nodes, machine_text, surface_text,
                validate_bin, render_bin, world_files, log, scratch_dir=RESULTS_DIR,
            )
            round_stats[name] = stats
            round_accepted[name] = paths
            existing = len(world_files)
            for i, p in enumerate(paths, start=1):
                kind = base.classify_file(validate_bin, p)
                seq = existing + i
                final_name = f"{seq:04d}-r{round_no}-{name}-{kind}.dmml"
                dest = wt_dir / "commits" / final_name
                dest.write_text(p.read_text())
                p.unlink(missing_ok=True)
                git(wt_dir, "add", f"commits/{final_name}")
                git(wt_dir, "commit", "--quiet", "-m", f"{name}: {final_name}")
                log(f"    [{name}] committed {final_name}")

        total_attempts = sum(s["valid"] + s["invalid"] + s["no_fence"] for s in round_stats.values())
        total_valid = sum(s["valid"] for s in round_stats.values())
        total_invalid = sum(s["invalid"] for s in round_stats.values())
        fail_rate = total_invalid / total_attempts if total_attempts else 0.0
        log(f"  round {round_no} authoring totals: valid={total_valid} invalid={total_invalid} "
            f"attempts={total_attempts} fail_rate={fail_rate:.2f}")

        log(f"  hub sync ({len(AGENTS)} hook-merge.sh calls into canonical)...")
        new_contests = 0
        for agent in AGENTS:
            name = agent["name"]
            if not round_accepted[name]:
                continue  # nothing new on this agent's branch to merge
            ok, n_minted, minted_pairs = sync_agent_into_canonical(canon, worktrees[name]["branch"], env, log)
            new_contests += n_minted
            for (s, p) in minted_pairs:
                contest_history.append((round_no, s, p))

        repeat_pairs = {}
        for (r, s, p) in contest_history:
            repeat_pairs.setdefault((s, p), set()).add(r)
        oscillating = [(s, p, sorted(rs)) for (s, p), rs in repeat_pairs.items() if len(rs) >= 2]

        report_rounds.append({
            "round": round_no, "stats": round_stats, "new_contests_minted": new_contests,
            "fail_rate": fail_rate,
        })
        (RESULTS_DIR / f"snapshot-after-round{round_no}.txt").write_text(
            render_repo_snapshot(render_bin, canon)
        )

        if oscillating:
            thrash_reason = f"same (subject, predicate) contested across multiple rounds: {oscillating}"
        elif new_contests >= 3:
            thrash_reason = f"round {round_no} minted {new_contests} contests at once"
        elif total_attempts and fail_rate >= 0.5:
            thrash_reason = f"round {round_no} failure rate {fail_rate:.2f} (>= 0.5)"

        if thrash_reason:
            log(f"\n*** STOPPING after round {round_no}: {thrash_reason} ***")
            break

    final_snapshot = render_repo_snapshot(render_bin, canon)
    (RESULTS_DIR / "snapshot-final.txt").write_text(final_snapshot)
    evidence_dir = RESULTS_DIR / "commits"
    if evidence_dir.exists():
        shutil.rmtree(evidence_dir)
    shutil.copytree(canon / "commits", evidence_dir)

    report = {
        "rounds_run": len(report_rounds),
        "rounds_requested": args.rounds,
        "thrash_reason": thrash_reason,
        "final_file_count": len(agent_commit_files(canon)),
        "rounds": report_rounds,
    }
    (RESULTS_DIR / "report.json").write_text(json.dumps(report, indent=2))
    (RESULTS_DIR / "log.txt").write_text("\n".join(round_log))
    log(f"\n=== done: {len(report_rounds)} round(s), thrash={thrash_reason!r} ===")
    log(f"(workdir {workdir} left on disk for inspection)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
