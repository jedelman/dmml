#!/usr/bin/env python3
"""Endurance/stability test: 4 fixed models, up to 20 stacked rounds,
each round handing every agent a randomly-sampled, slightly-overlapping
CORNER of the real materialized world (not the whole thing, and not a
static context -- regenerated from the real accepted-so-far state every
round) and letting each author as many commits/machines as it wants
this round, with no persona or personality given.

Real peer-to-peer risk is preserved on purpose: agents in the same
round do NOT see each other's output (mirrors the sync-spike's actual
divergence risk). After each round, every pair of agents' new commits
is checked for real overlapping (subject, predicate) touches via
dmml-hs's own check-divergence binary -- a real hit mints a Contest
(fact-commit + machine) into the world, exactly like the broker does.

Stops early -- before the 20-round cap -- the moment a real thrash
signal fires: the same (subject, predicate) contested in two different
rounds (agents oscillating on the same fact), a round where 3+ new
contests are minted at once, or a round where half or more of all
authoring attempts fail to produce usable content.

Usage:
    OPENROUTER_API_KEY=... python3 run.py [--rounds N] [--seed S]
"""
import argparse
import json
import os
import random
import re
import subprocess
import sys
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
DMML_HS = HERE.parent / "dmml-hs"
SURFACE_PATH = DMML_HS / "SURFACE.md"
SEED_DIR = DMML_HS / "examples" / "endurance"
SEED_GENESIS = SEED_DIR / "seed-genesis.dmml"
MACHINE_FILES = sorted((SEED_DIR / "machines").glob("*.dmml"))
RESULTS_DIR = HERE / "results"
COMMITS_DIR = RESULTS_DIR / "commits"

AGENTS = [
    {"name": "kimi", "model": "moonshotai/kimi-k2.5", "reasoning_none": True},
    {"name": "deepseek", "model": "deepseek/deepseek-v4-flash-0731", "reasoning_none": True},
    {"name": "glm", "model": "z-ai/glm-5.3-flash", "reasoning_none": False},
    {"name": "gemini", "model": "google/gemini-3.7-flash", "reasoning_none": False},
]
MAX_TOKENS = 12000
MAX_ATTEMPTS_PER_AGENT_PER_ROUND = 4  # hard cap, bounds runaway cost/time
CORNER_MAX_FACT_LINES = 26

SYSTEM_PROMPT_TEMPLATE = """You are an agent authoring content for a shared, append-only DMML \
(Desiring-Machine Markup Language) world, using its text authoring syntax. Several other agents \
are extending this same world in this same round, each seeing a different, overlapping slice of it \
-- you will not see their output until a later round, if at all. This is real: your commits become \
part of the permanent world the moment they're accepted.

--- SURFACE.md (commit and machine grammar) ---
{surface}
--- end SURFACE.md ---

Below is the CURRENT STATE of the portion of the world visible to you this round -- a real \
materialized slice, not the whole world. Extend it: mint new nodes, assert new facts connecting to \
what's shown, advance a machine-governed node's state via an ordinary fact assertion consistent with \
its machine definition (if shown), or declare a brand-new machine of your own. Stay consistent with \
what you're shown -- reuse real node names and declared predicates exactly as given; don't invent a \
new predicate for something already declared.

--- YOUR CORNER OF THE WORLD ---
{corner}
--- end YOUR CORNER OF THE WORLD ---

Respond with exactly ONE fenced code block containing a single DMML commit OR a single DMML machine \
(never both, never more than one). You may add brief prose outside the fence."""

CONTINUE_PROMPT = """That was accepted. You may author ONE more commit or machine this round, \
extending your corner further (it's been refreshed below with your own accepted work folded in), \
or stop here.

--- YOUR CORNER OF THE WORLD (refreshed) ---
{corner}
--- end YOUR CORNER OF THE WORLD ---

Reply with exactly ONE more fenced DMML commit/machine to continue this round, or reply with \
exactly the single word DONE (nothing else) to stop."""

RETRY_PROMPT = """That did not parse as valid DMML. Real parser error:

{error}

Try again: respond with exactly ONE fenced code block containing a single, corrected DMML commit \
or machine."""


def sh(*args, **kwargs):
    return subprocess.run(list(args), capture_output=True, text=True, **kwargs)


def build_binaries():
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    COMMITS_DIR.mkdir(parents=True, exist_ok=True)
    validate = DMML_HS / "validate-commit"
    render = DMML_HS / "render-snapshot"
    divergence = DMML_HS / "check-divergence"
    for src, out in [
        ("app/ValidateCommit.hs", validate),
        ("app/RenderSnapshot.hs", render),
        ("app/CheckDivergence.hs", divergence),
    ]:
        r = sh("ghc", "-isrc", "-iapp", "-O0", src, "-o", str(out), cwd=str(DMML_HS))
        if r.returncode != 0:
            print(r.stdout, r.stderr, file=sys.stderr)
            raise RuntimeError(f"build failed: {src}")
    return validate, render, divergence


def validate_file(validate_bin, path):
    r = sh(str(validate_bin), str(path))
    return r.returncode == 0, r.stdout


def render_snapshot(render_bin, files):
    r = sh(str(render_bin), *[str(f) for f in files])
    if r.returncode != 0:
        raise RuntimeError(f"render-snapshot failed on {files}:\n{r.stdout}")
    return r.stdout


def classify_file(validate_bin, path):
    """'commit' or 'machine' -- reuses the same dual-parse ValidateCommit
    already does, distinguished here by which surface parser actually
    accepts it (a machine file starts with the literal 'machine' verb;
    cheap and correct given the grammar's fixed first token)."""
    text = Path(path).read_text()
    return "machine" if text.lstrip().startswith("machine ") else "commit"


def extract_fence(text):
    m = re.search(r"```[^\n]*\n(.*?)```", text, re.DOTALL)
    if not m:
        return None
    body = m.group(1).strip()
    return body or None


def call_openrouter(api_key, model, reasoning_none, messages):
    payload = {"model": model, "max_tokens": MAX_TOKENS, "messages": messages}
    if reasoning_none:
        payload["reasoning"] = {"effort": "none"}
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=200) as resp:
        body = json.loads(resp.read().decode("utf-8"))
    if "choices" not in body:
        raise RuntimeError(f"unexpected response: {body}")
    msg = body["choices"][0]["message"]
    return msg.get("content") or ""


# ---- snapshot parsing / corner sampling ----

FACT_RE = re.compile(r"^  (\S+) \. (\S+) = (.+)$")
DISPUTE_HEADER_RE = re.compile(r"^  (\S+) \. (\S+) is disputed:$")


def parse_snapshot(text):
    """Returns (declared_block_text, facts: list[(subj, pred, val, is_node)], contested_block_text)."""
    lines = text.splitlines()
    facts = []
    declared_lines = []
    section = None
    for line in lines:
        if line == "Declared predicates:":
            section = "declared"
            continue
        if line == "Current facts:":
            section = "facts"
            continue
        if line.startswith("CONTESTED"):
            section = "contested"
            continue
        if section == "declared" and line.strip():
            declared_lines.append(line)
        elif section == "facts":
            m = FACT_RE.match(line)
            if m:
                subj, pred, val = m.groups()
                is_node = not (val.startswith('"') or val in ("true", "false") or val.replace(".", "", 1).lstrip("-").isdigit())
                facts.append((subj, pred, val, is_node))
    declared_block = "Declared predicates:\n" + "\n".join(declared_lines)
    return declared_block, facts


def sample_corners(full_text, n_agents, rng):
    """Returns list of (corner_text, corner_nodes: set[str])."""
    declared_block, facts = parse_snapshot(full_text)
    nodes = sorted({f[0] for f in facts} | {f[2] for f in facts if f[3]})
    if not nodes:
        return [(declared_block, set()) for _ in range(n_agents)]
    shuffled = nodes[:]
    rng.shuffle(shuffled)
    chunks = [shuffled[i::n_agents] for i in range(n_agents)]
    corners = []
    for i in range(n_agents):
        home = set(chunks[i])
        neighbor = chunks[(i + 1) % n_agents]
        overlap = set(rng.sample(neighbor, k=max(1, len(neighbor) // 5))) if neighbor else set()
        corner_nodes = home | overlap
        corner_facts = [f for f in facts if f[0] in corner_nodes or (f[3] and f[2] in corner_nodes)]
        rng.shuffle(corner_facts)
        corner_facts = corner_facts[:CORNER_MAX_FACT_LINES]
        fact_lines = "\n".join(f"  {s} . {p} = {v}" for s, p, v, _ in sorted(corner_facts))
        corners.append((declared_block + "\n\nCurrent facts (your corner only):\n" + fact_lines, corner_nodes))
    return corners


MACHINE_NODE_RE = re.compile(r"^machine (\S+)")


def machine_defs_by_node(world_files):
    """Scans every machine-classified file currently in the world (the
    original 10 plus anything agents or check-divergence have minted
    since) and indexes each by the node it's attached to, so a corner
    can be shown only the machine definitions for nodes actually in it
    -- not all of them, every round, regardless of corner. Showing every
    agent every machine's full definition unconditionally was a real
    design bug found by running this test: it silently re-widened every
    corner back to the whole world's "interesting" nodes and was the
    actual cause of round 1's heavy, immediate multi-way convergence on
    the same handful of machine-governed nodes, not real corner overlap."""
    index = {}
    for f in world_files:
        text = Path(f).read_text()
        if not text.lstrip().startswith("machine "):
            continue
        m = MACHINE_NODE_RE.match(text.lstrip())
        if m:
            index[m.group(1)] = text
    return index


def machine_defs_for_corner(node_index, corner_nodes):
    texts = [node_index[n] for n in sorted(corner_nodes) if n in node_index]
    if not texts:
        return "(no machine-governed node falls in your corner this round)"
    return "\n\n".join(texts)


# ---- agent round loop ----

def run_agent_round(api_key, agent, corner_text, corner_nodes, machine_defs_text, surface_text,
                     validate_bin, render_bin, base_files, log):
    accepted_paths = []
    messages = [
        {"role": "system", "content": SYSTEM_PROMPT_TEMPLATE.format(
            surface=surface_text + "\n\n--- Machine definitions for the machine-governed nodes in YOUR corner ---\n" + machine_defs_text,
            corner=corner_text)},
        {"role": "user", "content": "Author your first commit or machine for this round."},
    ]
    n_valid = 0
    n_invalid = 0
    n_no_fence = 0
    for attempt in range(MAX_ATTEMPTS_PER_AGENT_PER_ROUND):
        try:
            reply = call_openrouter(api_key, agent["model"], agent["reasoning_none"], messages)
        except Exception as e:  # noqa: BLE001
            log(f"    [{agent['name']}] dispatch error: {e}")
            break
        candidate = extract_fence(reply)
        if candidate is None:
            if reply.strip().upper() == "DONE":
                log(f"    [{agent['name']}] DONE after {n_valid} commit(s)")
                break
            n_no_fence += 1
            log(f"    [{agent['name']}] no fenced content, treating as stop")
            break
        messages.append({"role": "assistant", "content": reply})
        idx = len(accepted_paths) + n_invalid + 1
        tmp_path = RESULTS_DIR / f"_scratch-{agent['name']}-{idx}.dmml"
        tmp_path.write_text(candidate)
        ok, err = validate_file(validate_bin, tmp_path)
        if not ok:
            n_invalid += 1
            log(f"    [{agent['name']}] attempt {attempt+1}: INVALID -- {err.strip()[:200]}")
            if n_invalid >= 2:
                tmp_path.unlink(missing_ok=True)
                break
            messages.append({"role": "user", "content": RETRY_PROMPT.format(error=err)})
            tmp_path.unlink(missing_ok=True)
            continue
        n_valid += 1
        kind = classify_file(validate_bin, tmp_path)
        log(f"    [{agent['name']}] attempt {attempt+1}: accepted ({kind})")
        accepted_paths.append(tmp_path)
        if attempt + 1 >= MAX_ATTEMPTS_PER_AGENT_PER_ROUND:
            break
        # Refresh this agent's own view (base world + their own round-so-far output)
        refreshed = render_snapshot(render_bin, base_files + accepted_paths)
        rng_local = random.Random(hash((agent["name"], len(accepted_paths))))
        declared_block, facts = parse_snapshot(refreshed)
        nodes = sorted({f[0] for f in facts})
        fact_lines = "\n".join(f"  {s} . {p} = {v}" for s, p, v, _ in facts[:CORNER_MAX_FACT_LINES])
        own_corner = declared_block + "\n\nCurrent facts (your corner, refreshed):\n" + fact_lines
        messages.append({"role": "user", "content": CONTINUE_PROMPT.format(corner=own_corner)})
    return accepted_paths, {"valid": n_valid, "invalid": n_invalid, "no_fence": n_no_fence}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rounds", type=int, default=20)
    ap.add_argument("--seed", type=int, default=20260901)
    args = ap.parse_args()

    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        print("OPENROUTER_API_KEY not set", file=sys.stderr)
        return 1

    validate_bin, render_bin, divergence_bin = build_binaries()
    surface_text = SURFACE_PATH.read_text()
    rng = random.Random(args.seed)

    world_files = [SEED_GENESIS] + MACHINE_FILES
    seq = 1  # global monotonic sequence for permanently-adopted round output
    round_log = []
    contest_history = []  # list of (round, subj, pred) for thrash detection
    thrash_reason = None

    def log(msg):
        print(msg)
        round_log.append(msg)

    log(f"=== endurance test: {len(AGENTS)} agents, up to {args.rounds} rounds, seed={args.seed} ===")
    snapshot0 = render_snapshot(render_bin, world_files)
    (RESULTS_DIR / "snapshot-round0-seed.txt").write_text(snapshot0)
    log(f"seed world: {len(world_files)} files, {snapshot0.count(chr(10))} lines")

    report_rounds = []

    for round_no in range(1, args.rounds + 1):
        log(f"\n--- round {round_no} ---")
        full_snapshot = render_snapshot(render_bin, world_files)
        corners = sample_corners(full_snapshot, len(AGENTS), random.Random(args.seed * 1000 + round_no))
        node_index = machine_defs_by_node(world_files)

        round_accepted = {}  # agent_name -> list of Paths
        round_stats = {}
        for agent, (corner_text, corner_nodes) in zip(AGENTS, corners):
            machine_text = machine_defs_for_corner(node_index, corner_nodes)
            log(f"  [{agent['name']}] corner: {len(corner_nodes)} node(s), {len(machine_text.splitlines())} machine-def line(s)")
            paths, stats = run_agent_round(
                api_key, agent, corner_text, corner_nodes, machine_text, surface_text,
                validate_bin, render_bin, world_files, log,
            )
            round_accepted[agent["name"]] = paths
            round_stats[agent["name"]] = stats

        total_attempts = sum(s["valid"] + s["invalid"] + s["no_fence"] for s in round_stats.values())
        total_valid = sum(s["valid"] for s in round_stats.values())
        total_invalid = sum(s["invalid"] for s in round_stats.values())
        fail_rate = (total_invalid) / total_attempts if total_attempts else 0.0
        log(f"  round {round_no} totals: valid={total_valid} invalid={total_invalid} attempts={total_attempts} fail_rate={fail_rate:.2f}")

        # Adopt every valid file into the permanent world, with a real
        # sequence prefix (same discipline as broker.sh -- materialization
        # order matters).
        adopted_this_round = []
        for agent in AGENTS:
            for p in round_accepted[agent["name"]]:
                kind = classify_file(validate_bin, p)
                final_name = COMMITS_DIR / f"{seq:04d}-r{round_no}-{agent['name']}-{kind}.dmml"
                final_name.write_text(p.read_text())
                p.unlink(missing_ok=True)
                world_files.append(final_name)
                adopted_this_round.append((agent["name"], kind, final_name))
                seq += 1

        # Pairwise divergence check across every pair of agents' OWN new
        # commit files this round (machines excluded -- check-divergence's
        # materializer only understands commits, same as broker.sh).
        new_contests = 0
        agent_commit_files = {
            a["name"]: [f for (name, kind, f) in adopted_this_round if name == a["name"] and kind == "commit"]
            for a in AGENTS
        }
        for i in range(len(AGENTS)):
            for j in range(i + 1, len(AGENTS)):
                a, b = AGENTS[i]["name"], AGENTS[j]["name"]
                if not agent_commit_files[a] or not agent_commit_files[b]:
                    continue
                list_a = RESULTS_DIR / f"_list-{a}.txt"
                list_b = RESULTS_DIR / f"_list-{b}.txt"
                list_a.write_text("\n".join(str(f) for f in agent_commit_files[a]))
                list_b.write_text("\n".join(str(f) for f in agent_commit_files[b]))
                pair_out = RESULTS_DIR / "_pair_contests"
                pair_out.mkdir(exist_ok=True)
                r = sh(str(divergence_bin), str(list_a), str(list_b), str(pair_out), a, b)
                out = r.stdout
                if "no divergence" in out:
                    continue
                for line in out.splitlines():
                    if line.startswith("DIVERGENCE minted as content:"):
                        # files are named contest-N-...dmml / .machine.dmml in pair_out
                        pass
                    m = re.match(r"^  (\S+) \. (\S+): ", line)
                    if m:
                        subj, pred = m.groups()
                        contest_history.append((round_no, subj, pred))
                        new_contests += 1
                        log(f"  CONTEST minted: {a} vs {b} on {subj} . {pred}")
                # adopt minted contest files into the world, sequence-renamed
                for f in sorted(pair_out.glob("contest-*.dmml")):
                    kind = "machine" if f.name.endswith(".machine.dmml") else "commit"
                    tag = "machine" if kind == "machine" else "commit"
                    final_name = COMMITS_DIR / f"{seq:04d}-r{round_no}-contest-{a}-{b}-{tag}.dmml"
                    final_name.write_text(f.read_text())
                    f.unlink()
                    world_files.append(final_name)
                    seq += 1

        # Thrash detection.
        repeat_pairs = {}
        for (r, s, p) in contest_history:
            repeat_pairs.setdefault((s, p), set()).add(r)
        oscillating = [(s, p, sorted(rs)) for (s, p), rs in repeat_pairs.items() if len(rs) >= 2]

        round_summary = {
            "round": round_no,
            "stats": round_stats,
            "new_contests": new_contests,
            "fail_rate": fail_rate,
            "adopted_files": [str(f) for (_, _, f) in adopted_this_round],
        }
        report_rounds.append(round_summary)

        if oscillating:
            thrash_reason = f"same (subject, predicate) contested across multiple rounds: {oscillating}"
        elif new_contests >= 3:
            thrash_reason = f"round {round_no} minted {new_contests} contests at once"
        elif total_attempts and fail_rate >= 0.5:
            thrash_reason = f"round {round_no} failure rate {fail_rate:.2f} (>= 0.5)"

        (RESULTS_DIR / f"snapshot-after-round{round_no}.txt").write_text(render_snapshot(render_bin, world_files))

        if thrash_reason:
            log(f"\n*** THRASH DETECTED, stopping early after round {round_no}: {thrash_reason} ***")
            break

    final_snapshot = render_snapshot(render_bin, world_files)
    (RESULTS_DIR / "snapshot-final.txt").write_text(final_snapshot)

    report = {
        "rounds_run": len(report_rounds),
        "rounds_requested": args.rounds,
        "thrash_reason": thrash_reason,
        "final_world_file_count": len(world_files),
        "rounds": report_rounds,
    }
    (RESULTS_DIR / "report.json").write_text(json.dumps(report, indent=2))
    (RESULTS_DIR / "log.txt").write_text("\n".join(round_log))
    log(f"\n=== done: {len(report_rounds)} round(s) run, thrash={thrash_reason!r} ===")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
