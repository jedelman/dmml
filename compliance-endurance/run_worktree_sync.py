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
(as of the 2026-09-02 collision-free-mints redesign) divergence
REPORTING, never Contest-minting -- nothing is minted anymore, a real
overlap just shows up as a genuinely multi-valued fact. No full mesh
needed: since every agent worktree shares the SAME underlying repo,
there is exactly one place ("mine") for divergence to be checked
against, not four.

REWORKED 2026-09-02 for the 200-commit E1 run: scoped by commit volume
(--target-commits), not round count -- the old round cap plus its own
thrash-detector (oscillating-contest / 3+-contests-per-round /
high-fail-rate early stop) is gone, since nothing mints a Contest to
oscillate on anymore. Wall-clock and token spend are tracked and
reported but are safety-cap guardrails only, not the primary stop
condition (Jason's explicit call). A real DMML.Entropy sidecar runs
alongside as a pure observer, watching commits/ live -- see
written-world/dev-journal/2026-09-02-entropy-sidecar-guardian-process-
pattern.md.

Usage:
    OPENROUTER_API_KEY=... python3 run_worktree_sync.py [--target-commits N] [--seed S]
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
import time
from pathlib import Path

import run as base  # reuse dispatch/corner-sampling/classification helpers

HERE = Path(__file__).resolve().parent
DMML_HS = HERE.parent / "dmml-hs"
SYNC_SPIKE_BROKER = HERE.parent.parent / "written-world" / "sync-spike" / "broker"
INSTALL_HOOKS_SH = SYNC_SPIKE_BROKER / "install-hooks.sh"
HOOK_MERGE_SH = SYNC_SPIKE_BROKER / "hook-merge.sh"
REBUILD_CACHE_SH = SYNC_SPIKE_BROKER / "rebuild-cache.sh"
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
    validation and (as of the 2026-09-02 collision-free-mints redesign)
    REPORTING divergence, never minting a Contest anymore.

    REWORKED to match: this used to parse "DIVERGENCE minted as
    content:" -- that string hasn't existed in check-divergence's
    output since dmml a8470c7, so this was silently finding zero
    matches on every real run since then, a real stale-parser bug
    caught before spending real dispatch cost on it. Now parses the
    real output ("DIVERGENCE (live, unresolved): subj . pred"), and
    returns the pairs still live/unresolved after this merge -- nothing
    is ever "minted"; a pair may just still be genuinely multi-valued
    (ungoverned, or governed but not yet resolved)."""
    r = sh("bash", str(HOOK_MERGE_SH), branch, cwd=str(canon), env=env)
    # Real bug found running this for real (still true post-redesign):
    # git routes an invoked hook's own stdout through to the CALLING
    # process's stderr, not its stdout. Search both.
    combined = r.stdout + r.stderr
    if r.returncode != 0:
        log(f"    [sync] canonical <- {branch}: REJECTED (exit {r.returncode})\n"
            f"      stdout:\n{r.stdout}\n      stderr:\n{r.stderr}")
        return False, set()
    live_pairs = set()
    for line in combined.splitlines():
        m = DIVERGENCE_SUBJ_PRED_RE.match(line)
        if m:
            live_pairs.add((m.group(1), m.group(2)))
    if live_pairs:
        log(f"    [sync] canonical <- {branch}: ACCEPTED, {len(live_pairs)} pair(s) still live/unresolved")
    else:
        log(f"    [sync] canonical <- {branch}: ACCEPTED, no divergence")

    # F2/checkpoint-per-commit real-scale check: this merge's post-merge
    # hook should have folded a SMALL number of new files (in the
    # bootstrap case, everything so far once; every merge after that,
    # only what this specific merge introduced) -- never a count that
    # grows with total history. Surfaced per-merge here rather than only
    # checked once at the end, so a regression shows up round-by-round,
    # not just in a final summary.
    cp_fold_count = None
    m = CHECKPOINT_FOLD_RE.search(combined)
    if m:
        cp_fold_count = int(m.group(1))
        log(f"    [sync] checkpoint: folded {cp_fold_count} new file(s) this merge")
    elif "checkpoint-rebuild failed" in combined:
        log(f"    [sync] WARNING: checkpoint-rebuild failed this merge (non-fatal, see hook output above)")

    return True, live_pairs, cp_fold_count


DIVERGENCE_SUBJ_PRED_RE = re.compile(r"^DIVERGENCE \(live, unresolved\): (\S+) \. (\S+)$")
CHECKPOINT_FOLD_RE = re.compile(r"\[post-merge\] checkpointed (\d+) new file\(s\)")


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
    ap.add_argument("--target-commits", type=int, default=200,
                     help="stop once this many valid commits have landed (primary scoping axis)")
    ap.add_argument("--max-rounds", type=int, default=60, help="safety cap, not the primary stop condition")
    ap.add_argument("--max-wall-seconds", type=int, default=4 * 3600, help="safety cap")
    ap.add_argument("--max-tokens", type=int, default=8_000_000, help="safety cap (prompt+completion combined)")
    ap.add_argument("--seed", type=int, default=20260903)
    ap.add_argument("--entropy-window", type=int, default=5)
    ap.add_argument("--entropy-threshold", type=float, default=0.5)
    args = ap.parse_args()

    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        print("OPENROUTER_API_KEY not set", file=sys.stderr)
        return 1

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    validate_bin, render_bin, divergence_bin, entropy_sidecar_bin, checkpoint_rebuild_bin = base.build_binaries()
    surface_text = base.SURFACE_PATH.read_text()

    round_log = []

    def log(msg):
        print(msg, flush=True)
        round_log.append(msg)

    workdir = Path(tempfile.mkdtemp(prefix="endurance-worktree-sync-"))
    log(f"=== worktree-sync endurance test: {len(AGENTS)} agents "
        f"({', '.join(a['name'] for a in AGENTS)}), target={args.target_commits} commits "
        f"(safety caps: {args.max_rounds} rounds / {args.max_wall_seconds}s / {args.max_tokens} tokens), "
        f"hub topology via hooks, workdir={workdir} ===")
    canon, worktrees = setup_world(workdir)

    env = dict(os.environ)
    env["DMML_VALIDATOR"] = str(validate_bin)
    env["DMML_DIVERGENCE_CHECK"] = str(divergence_bin)
    env["DMML_RENDER_SNAPSHOT"] = str(render_bin)
    env["DMML_REBUILD_CACHE"] = str(REBUILD_CACHE_SH)
    env["DMML_CHECKPOINT_REBUILD"] = str(checkpoint_rebuild_bin)

    # Entropy sidecar: a real, separate, resumable process watching the
    # canonical repo's commits/ live for the whole run -- an observer,
    # not a stop-trigger (this harness's own stop condition is
    # commit-volume, unrelated to what the sidecar finds; see
    # written-world/dev-journal/2026-09-02-entropy-sidecar-guardian-
    # process-pattern.md for why that decoupling is deliberate).
    sidecar_checkpoint = RESULTS_DIR / "entropy-checkpoint.json"
    sidecar_proc = None
    if entropy_sidecar_bin.exists():
        sidecar_proc = subprocess.Popen(
            [str(entropy_sidecar_bin), str(canon / "commits"), str(sidecar_checkpoint),
             str(args.entropy_window), str(args.entropy_threshold), "--watch", "10"],
            stdout=open(RESULTS_DIR / "entropy-sidecar.log", "w"),
            stderr=subprocess.STDOUT,
        )
        log(f"  entropy sidecar started (pid {sidecar_proc.pid}), watching {canon / 'commits'} every 10s")
    else:
        log(f"  WARNING: entropy sidecar binary not found at {entropy_sidecar_bin}, running without it")

    live_pair_history = []  # (round, subj, pred) -- pairs reported still-live after a sync, informational only
    checkpoint_fold_history = []  # {round, agent, folded} -- per-merge checkpoint fold counts, real-scale evidence
    report_rounds = []
    stop_reason = None
    total_valid_commits = 0
    total_prompt_tokens = 0
    total_completion_tokens = 0
    total_dispatch_seconds = 0.0
    per_agent_commits = {a["name"]: 0 for a in AGENTS}
    run_start = time.monotonic()

    def write_status():
        elapsed = time.monotonic() - run_start
        status = {
            "rounds_run": len(report_rounds),
            "total_valid_commits": total_valid_commits,
            "target_commits": args.target_commits,
            "wall_elapsed_seconds": round(elapsed, 1),
            "total_prompt_tokens": total_prompt_tokens,
            "total_completion_tokens": total_completion_tokens,
            "total_tokens": total_prompt_tokens + total_completion_tokens,
            "total_dispatch_seconds": round(total_dispatch_seconds, 1),
            "avg_dispatch_seconds": round(total_dispatch_seconds / max(1, sum(
                r["stats"][a]["n_dispatches"] for r in report_rounds for a in r["stats"]
            )), 2) if report_rounds else 0,
            "per_agent_commits": per_agent_commits,
            "live_unresolved_pairs_seen": sorted({f"{s}.{p}" for (_, s, p) in live_pair_history}),
        }
        (RESULTS_DIR / "status.json").write_text(json.dumps(status, indent=2))
        return status

    round_no = 0
    while True:
        round_no += 1
        log(f"\n--- round {round_no} --- (commits so far: {total_valid_commits}/{args.target_commits}, "
            f"tokens so far: {total_prompt_tokens + total_completion_tokens}, "
            f"elapsed: {time.monotonic() - run_start:.0f}s)")
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
            total_prompt_tokens += stats["prompt_tokens"]
            total_completion_tokens += stats["completion_tokens"]
            total_dispatch_seconds += stats["dispatch_seconds"]
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
            per_agent_commits[name] += stats["valid"]
            total_valid_commits += stats["valid"]

        total_attempts = sum(s["valid"] + s["invalid"] + s["no_fence"] for s in round_stats.values())
        total_invalid = sum(s["invalid"] for s in round_stats.values())
        round_valid = sum(s["valid"] for s in round_stats.values())
        fail_rate = total_invalid / total_attempts if total_attempts else 0.0
        log(f"  round {round_no} authoring totals: valid={round_valid} invalid={total_invalid} "
            f"attempts={total_attempts} fail_rate={fail_rate:.2f} "
            f"(cumulative commits: {total_valid_commits}/{args.target_commits})")

        log(f"  hub sync ({len(AGENTS)} hook-merge.sh calls into canonical)...")
        for agent in AGENTS:
            name = agent["name"]
            if not round_accepted[name]:
                continue  # nothing new on this agent's branch to merge
            ok, live_pairs, cp_fold_count = sync_agent_into_canonical(canon, worktrees[name]["branch"], env, log)
            for (s, p) in live_pairs:
                live_pair_history.append((round_no, s, p))
            if cp_fold_count is not None:
                checkpoint_fold_history.append({"round": round_no, "agent": name, "folded": cp_fold_count})

        report_rounds.append({
            "round": round_no, "stats": round_stats, "fail_rate": fail_rate,
            "cumulative_commits": total_valid_commits,
        })
        (RESULTS_DIR / f"snapshot-after-round{round_no}.txt").write_text(
            render_repo_snapshot(render_bin, canon)
        )
        write_status()

        elapsed = time.monotonic() - run_start
        # Real, disclosed guardrails -- NOT the primary stop condition
        # (that's target_commits, per Jason's explicit call: commit
        # volume, not wall-clock or token budget). These only exist to
        # kill a stuck or runaway run.
        if total_valid_commits >= args.target_commits:
            stop_reason = f"reached target of {args.target_commits} commits"
        elif round_no >= args.max_rounds:
            stop_reason = f"safety cap: {args.max_rounds} rounds reached"
        elif elapsed >= args.max_wall_seconds:
            stop_reason = f"safety cap: {args.max_wall_seconds}s wall-clock reached"
        elif (total_prompt_tokens + total_completion_tokens) >= args.max_tokens:
            stop_reason = f"safety cap: {args.max_tokens} tokens reached"
        elif round_valid == 0 and len(report_rounds) >= 2 and report_rounds[-2]["stats"] and \
                sum(s["valid"] for s in report_rounds[-2]["stats"].values()) == 0:
            stop_reason = f"stuck: two consecutive rounds ({round_no - 1}, {round_no}) produced zero valid commits"

        if stop_reason:
            log(f"\n*** STOPPING after round {round_no}: {stop_reason} ***")
            break

    if sidecar_proc is not None:
        log("  stopping entropy sidecar...")
        sidecar_proc.terminate()
        try:
            sidecar_proc.wait(timeout=15)
        except subprocess.TimeoutExpired:
            sidecar_proc.kill()

    final_snapshot = render_repo_snapshot(render_bin, canon)
    (RESULTS_DIR / "snapshot-final.txt").write_text(final_snapshot)
    evidence_dir = RESULTS_DIR / "commits"
    if evidence_dir.exists():
        shutil.rmtree(evidence_dir)
    shutil.copytree(canon / "commits", evidence_dir)

    entropy_alerts = sorted((canon / "commits").glob("*entropy-collapse*.dmml"))

    # Checkpoint-per-commit real-scale verification: independently fold
    # every real commits/*.dmml file from scratch under the same final
    # tree-sha the incremental chain actually reached, and diff against
    # what that chain of many single-merge incremental folds produced.
    # This is the exact same check the small worktree demo already
    # verified once by hand -- run again here at real endurance scale
    # (dozens of merges, not 4) to catch anything the small demo
    # wouldn't have surfaced (e.g. a bug that only shows up once a
    # checkpoint chain is many links deep).
    checkpoint_dir = canon / "checkpoints"
    final_tree_sha = git(canon, "rev-parse", "HEAD:commits").strip()
    checkpoint_files = sorted(checkpoint_dir.glob("*.json")) if checkpoint_dir.exists() else []
    tip_checkpoint = checkpoint_dir / f"{final_tree_sha}.json"
    checkpoint_verification = {
        "checkpoint_files_written": len(checkpoint_files),
        "tip_checkpoint_exists": tip_checkpoint.exists(),
        "fold_counts_per_merge": [h["folded"] for h in checkpoint_fold_history],
        "max_fold_count": max((h["folded"] for h in checkpoint_fold_history), default=None),
    }
    if tip_checkpoint.exists():
        reference_out = RESULTS_DIR / "checkpoint-reference-full-replay.json"
        r = sh(str(checkpoint_rebuild_bin), final_tree_sha, str(reference_out), "none",
               *[str(f) for f in agent_commit_files(canon)])
        if r.returncode != 0:
            checkpoint_verification["matches_full_replay"] = False
            checkpoint_verification["error"] = r.stdout + r.stderr
        else:
            checkpoint_verification["matches_full_replay"] = (
                tip_checkpoint.read_text() == reference_out.read_text()
            )
    log(f"  checkpoint verification: {checkpoint_verification}")

    report = {
        "rounds_run": len(report_rounds),
        "target_commits": args.target_commits,
        "total_valid_commits": total_valid_commits,
        "stop_reason": stop_reason,
        "final_file_count": len(agent_commit_files(canon)),
        "wall_elapsed_seconds": round(time.monotonic() - run_start, 1),
        "total_prompt_tokens": total_prompt_tokens,
        "total_completion_tokens": total_completion_tokens,
        "per_agent_commits": per_agent_commits,
        "live_unresolved_pairs": sorted({f"{s}.{p}" for (_, s, p) in live_pair_history}),
        "entropy_collapse_alerts": [f.name for f in entropy_alerts],
        "checkpoint_verification": checkpoint_verification,
        "rounds": report_rounds,
    }
    (RESULTS_DIR / "report.json").write_text(json.dumps(report, indent=2))
    (RESULTS_DIR / "log.txt").write_text("\n".join(round_log))
    write_status()
    log(f"\n=== done: {len(report_rounds)} round(s), {total_valid_commits} commits, "
        f"stop_reason={stop_reason!r}, entropy alerts={len(entropy_alerts)} ===")
    log(f"(workdir {workdir} left on disk for inspection)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
