#!/usr/bin/env python3
"""Standalone Jev-driven CLI loop over the existing `fire-transition` binary.

Orchestration on top of an unmodified CLI primitive, the same
relationship cascade-demo/run.sh has to `fire-transition` -- nothing
here is a new DMML engine primitive. Each round:

  1. Dry-fire every candidate in the config against the current world
     snapshot. A candidate is "legal" iff `fire-transition` exits 0
     and its output differs from that candidate's own last firing
     (the exact dedup cascade-demo/run.sh uses, and for the same
     reason: a from->to guard that stays satisfied after firing would
     otherwise "succeed" forever with nothing new).
  2. If nothing is legal and new, stop -- a real fixpoint, the healthy
     termination case.
  3. If a budget cap is hit first, stop and say exactly which cap and
     why -- never silently keep going, never silently raise the cap.
  4. Otherwise, hand the legal candidates to Jev as a Choice question,
     apply whichever one it picks, append the round to the audit log,
     and loop.

KNOWN, DISCLOSED SCOPE LIMITS (not oversights):
  - Candidate (machine, transition, params) tuples are hand-authored
    in the config file, not auto-enumerated from declared nodes. DMML
    has no generic "for every node of type X, what fires" query
    exposed via any CLI yet -- WorldBrowserDemo.hs's own doc comment
    flags the same gap (DMML.Guard.availableTransitions takes one
    already-bound EvalContext, it doesn't generate candidate
    bindings). Widening a config's candidate list is what stands in
    for "new content" until that enumeration exists.
  - Minted-node tracking below is a text-scan heuristic over commit
    output (anything containing "/" that isn't a quoted string), not
    a real DMML.Ast parse -- good enough to bound a run, not a
    replacement for a real `--json` output mode on fire-transition.
  - The "state" text handed to Jev is a light per-round summary, not
    the full world snapshot. Tune `build_state_summary` once you can
    see what Jev actually needs to choose well in practice.

UNTESTED IN THIS SESSION: this sandbox has no GHC/cabal toolchain and
no Jev API key, so `fire-transition` was never actually invoked here
and no live Jev call was made. The CLI argument shape is taken
directly from FireTransition.hs's own parseArgs/usage string and the
wire format from docs.typesafe.ai (POST /v1/systemone, Bearer auth,
{state, model, questions} request / {answers, usage} response) -- both
read, not guessed at -- but run this for real, with --dry-run first,
before trusting it with anything you care about.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path

JEV_ENDPOINT = "https://api.typesafe.ai/v1/systemone"
NODE_TOKEN_RE = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")


@dataclass
class Candidate:
    id: str
    machine: str
    transition: str
    verb: str
    params: dict
    description: str
    firings: int = 0
    last_hash: str | None = None


@dataclass
class Budget:
    max_rounds: int
    max_total_firings: int
    max_firings_per_candidate: int
    max_minted_nodes: int


@dataclass
class RunState:
    world_files: list[str]
    machine_files: list[str]
    candidates: dict[str, Candidate]
    known_nodes: set[str] = field(default_factory=set)
    total_firings: int = 0
    minted_nodes: int = 0


def load_config(path: Path) -> tuple[RunState, Budget, dict]:
    cfg = json.loads(path.read_text())
    base = path.parent
    world_files = [str((base / w).resolve()) for w in cfg["world_seed"]]
    machine_files = [str((base / m).resolve()) for m in cfg["machines"]]
    candidates = {}
    for c in cfg["candidates"]:
        candidates[c["id"]] = Candidate(
            id=c["id"],
            machine=str((base / c["machine"]).resolve()),
            transition=c["transition"],
            verb=c["verb"],
            params=c.get("params", {}),
            description=c["description"],
        )
    b = cfg["budget"]
    budget = Budget(
        max_rounds=b["max_rounds"],
        max_total_firings=b["max_total_firings"],
        max_firings_per_candidate=b["max_firings_per_candidate"],
        max_minted_nodes=b["max_minted_nodes"],
    )
    state = RunState(world_files=world_files, machine_files=machine_files, candidates=candidates)
    for wf in world_files:
        state.known_nodes |= set(NODE_TOKEN_RE.findall(Path(wf).read_text()))
    return state, budget, cfg["jev"]


def fire_binary() -> list[str]:
    import shlex

    return shlex.split(os.environ.get("FIRE_TRANSITION", "fire-transition"))


def dry_fire(candidate: Candidate, state: RunState) -> tuple[bool, str]:
    cmd = fire_binary() + [candidate.machine, candidate.transition, candidate.verb]
    for w in state.world_files:
        cmd += ["--world", w]
    for m in state.machine_files:
        if m != candidate.machine:
            cmd += ["--machine", m]
    for k, v in candidate.params.items():
        cmd += ["--param", f"{k}={v}"]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    except FileNotFoundError:
        print(
            f"fatal: '{' '.join(fire_binary())}' not found -- build dmml-hs and put fire-transition on PATH, "
            "or set FIRE_TRANSITION (e.g. 'stack exec fire-transition --')",
            file=sys.stderr,
        )
        sys.exit(2)
    if proc.returncode != 0:
        return False, proc.stderr.strip() or proc.stdout.strip()
    return True, proc.stdout


def legal_candidates(state: RunState) -> list[tuple[Candidate, str]]:
    legal = []
    for c in state.candidates.values():
        ok, out = dry_fire(c, state)
        if not ok:
            continue
        h = hashlib.sha256(out.encode()).hexdigest()
        if h == c.last_hash:
            continue  # legal, but identical to its own last firing -- not new
        legal.append((c, out))
    return legal


def build_state_summary(round_no: int, state: RunState, legal: list[tuple]) -> str:
    return (
        f"Round {round_no}. World has {len(state.world_files)} committed fact files, "
        f"{state.total_firings} prior firings, {len(state.known_nodes)} known nodes. "
        f"{len(legal)} actions are legal this round."
    )


def call_jev(api_key: str, model: str, instructions: str, state_summary: str, legal: list[tuple]) -> dict:
    criteria = {c.id: c.description for c, _ in legal}
    body = {
        "state": state_summary,
        "model": model,
        "questions": {
            "next_action": {
                "type": "choice",
                "instructions": instructions,
                "criteria": criteria,
            }
        },
    }
    req = urllib.request.Request(
        JEV_ENDPOINT,
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as e:
        print(f"fatal: Jev call failed: {e.code} {e.read().decode()}", file=sys.stderr)
        sys.exit(3)


def apply_winner(candidate: Candidate, output: str, world_dir: Path, round_no: int, state: RunState) -> int:
    out_file = world_dir / f"{round_no:03d}-{candidate.id}.dmml"
    out_file.write_text(output)
    state.world_files.append(str(out_file))
    new_nodes = set(NODE_TOKEN_RE.findall(output)) - state.known_nodes
    state.known_nodes |= new_nodes
    state.minted_nodes += len(new_nodes)
    state.total_firings += 1
    candidate.firings += 1
    candidate.last_hash = hashlib.sha256(output.encode()).hexdigest()
    return len(new_nodes)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("config", type=Path)
    ap.add_argument("--world-dir", type=Path, default=None, help="scratch dir for accumulating firings (default: mkdtemp)")
    ap.add_argument("--audit-log", type=Path, default=None)
    ap.add_argument("--dry-run", action="store_true", help="skip the Jev call, pick the first legal candidate deterministically")
    ap.add_argument("--api-key", default=os.environ.get("TYPESAFE_API_KEY"))
    args = ap.parse_args()

    state, budget, jev_cfg = load_config(args.config)

    world_dir = args.world_dir or Path(subprocess_mkdtemp())
    world_dir.mkdir(parents=True, exist_ok=True)
    audit_path = args.audit_log or (world_dir / "audit.jsonl")
    audit = audit_path.open("a")

    if not args.dry_run and not args.api_key:
        print("fatal: no Jev API key. Set TYPESAFE_API_KEY or pass --api-key, or run --dry-run.", file=sys.stderr)
        sys.exit(2)

    round_no = 1
    while True:
        if round_no > budget.max_rounds:
            print(f"=== stopped: hit max_rounds={budget.max_rounds} without a fixpoint ===")
            break
        if state.total_firings >= budget.max_total_firings:
            print(f"=== stopped: hit max_total_firings={budget.max_total_firings} ===")
            break
        if state.minted_nodes >= budget.max_minted_nodes:
            print(f"=== stopped: hit max_minted_nodes={budget.max_minted_nodes} ===")
            break

        legal = [
            (c, out)
            for c, out in legal_candidates(state)
            if c.firings < budget.max_firings_per_candidate
        ]
        if not legal:
            print(f"=== round {round_no}: fixpoint -- nothing legal and new, stopping cleanly ===")
            break

        state_summary = build_state_summary(round_no, state, legal)

        if args.dry_run:
            winner, out = legal[0]
            jev_response = {"dry_run": True}
        else:
            jev_response = call_jev(args.api_key, jev_cfg["model"], jev_cfg["instructions"], state_summary, legal)
            answer = jev_response["answers"]["next_action"]
            chosen_id = answer["choice"]
            winner, out = next((c, o) for c, o in legal if c.id == chosen_id)

        minted = apply_winner(winner, out, world_dir, round_no, state)

        record = {
            "round": round_no,
            "state_summary": state_summary,
            "legal_candidate_ids": [c.id for c, _ in legal],
            "jev_response": jev_response,
            "chosen": winner.id,
            "minted_nodes_this_round": minted,
            "total_firings": state.total_firings,
        }
        audit.write(json.dumps(record) + "\n")
        audit.flush()
        print(f"round {round_no}: fired {winner.id} ({minted} new node(s), {state.total_firings} total firings)")
        round_no += 1

    print(f"world dir: {world_dir}")
    print(f"audit log: {audit_path}")
    audit.close()


def subprocess_mkdtemp() -> str:
    import tempfile

    return tempfile.mkdtemp(prefix="jev-driver-")


if __name__ == "__main__":
    main()
