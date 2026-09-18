#!/usr/bin/env python3
"""Standalone Jev-driven CLI loop over the existing `fire-transition` binary.

Orchestration on top of an unmodified CLI primitive, the same
relationship cascade-demo/run.sh has to `fire-transition` -- nothing
here is a new DMML engine primitive. Each round is now a whole
GENERATION (2026-09-18, Jason's framing: "Jev can handle massive
parallelism. Send them the entire horizon each request, with mutually
exclusive transitions grouped into choices."), not a single action:

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
  4. Otherwise, partition the legal candidates into mutually-exclusive
     GROUPS (group_into_generations -- same machine is always one
     group; different machines are grouped together only if actually
     re-simulating with the real guard evaluator shows a real
     conflict, never a text-scanned guess), send Jev ONE request with
     one `choice` question per group (call_jev_batch -- confirmed live
     that Jev genuinely resolves multiple independent `questions` keys
     in a single call, each with its own confidence/probabilities),
     apply every group's winner in sequence (re-validating each one
     for real against the state as it stands, since a wrong grouping
     would be a real correctness bug, not caught by trusting the
     proof), append the whole generation to the audit log as one
     round, and loop.

KNOWN, DISCLOSED SCOPE LIMITS (not oversights):
  - Candidate (machine, transition, params) tuples are still mostly
    hand-authored in the config file -- DMML has no generic "for every
    node of type X, what fires" query with values already bound.
    Partial exception, added 2026-09-17: a candidate that spawns a
    machine (a real EffectSpawn) can declare `spawns_followup` in the
    config, and this driver then registers the resulting fact-native
    machine's follow-up candidate DYNAMICALLY, the instant the spawn's
    captured Surface `machine` block is written -- no path or node name
    needs predicting ahead of time. `discover_fact_native` also runs
    the real `list-candidates` binary (DMML.MachineFacts.
    candidateTransitions) every round as a transparency pass, printed
    and logged -- confirms independently what's fact-native-discoverable,
    but stays informational rather than auto-firing, because
    list-candidates reports formal param NAMES, not concrete VALUES,
    and turning one into the other is a real binding decision
    `spawns_followup` makes explicitly rather than guessed generically.
    A hand-authored Surface-text machine (furnace/anvil/catalyst
    themselves) is still permanently invisible to list-candidates
    unless something also runs it through DMML.MachineFacts.
    encodeMachine -- that part of the gap is unchanged.
  - Minted-node tracking below is a text-scan heuristic over commit
    output (anything containing "/" that isn't a quoted string), not
    a real DMML.Ast parse -- good enough to bound a run, not a
    replacement for a real `--json` output mode on fire-transition.
  - The "state" text handed to Jev is a light per-round summary, not
    the full world snapshot. Tune `build_state_summary` once you can
    see what Jev actually needs to choose well in practice.

VERIFIED LIVE (2026-09-17): built a real GHC/cabal toolchain, fired
`fire-transition` for real against cascade-demo, and made real live
calls to Jev (api.typesafe.ai, POST /v1/systemone) that chose among
genuinely contested candidates -- including, later the same session, a
round with a real EffectSpawn (a catalyst machine tempering a furnace,
spawning a new fact-native machine), whose captured Surface `machine`
block this driver now writes out and dynamically registers as a new
fireable candidate via a config entry's own `spawns_followup`. See
`examples/jev-driver-demo/README.md` and claude-memory's
2026-09-17 conversation logs for the full account. Still worth running
--dry-run first on any new candidates config before spending a live
call -- this note records that the mechanism works, not that every
config you write will be correct on the first try.
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
    # Optional: when this candidate's firing spawns a machine (a real
    # DMML.Ast.EffectSpawn), this describes the follow-up candidate to
    # register dynamically once the spawn actually happens -- id,
    # transition, verb, params, description, same shape as a config
    # candidate entry minus "machine" (the machine is whatever file the
    # spawn's own captured Surface block gets written to, not knowable
    # ahead of time since it lives under a per-run mkdtemp world dir).
    spawns_followup: dict | None = None
    # Optional: firing this candidate stops the round loop immediately
    # afterward, win or lose, rather than being just another ordinary
    # DMML transition. Deliberately still an ordinary transition (see
    # season.dmml's `end()`) -- legal-checked, dedup-checked, offered to
    # Jev alongside everything else -- rather than a fake pseudo-
    # candidate bypassing dry_fire/apply_winner, so "give Jev a
    # terminate option" doesn't require special-casing the mechanics,
    # only the loop's own exit condition.
    terminates: bool = False


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
            spawns_followup=c.get("spawns_followup"),
            terminates=c.get("terminates", False),
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


def dry_fire(candidate: Candidate, state: RunState, extra_world_files: list[str] = ()) -> tuple[bool, str]:
    cmd = fire_binary() + [candidate.machine, candidate.transition, candidate.verb]
    for w in list(state.world_files) + list(extra_world_files):
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


def group_into_generations(
    legal: list[tuple[Candidate, str]], state: RunState, world_dir: Path
) -> list[list[tuple[Candidate, str]]]:
    """Partitions this round's legal candidates into conflict-free
    groups -- "a whole generation" per Jason's framing: every group
    becomes one independent `choice` question in a single Jev call, and
    every group's winner gets applied in the same round, since nothing
    in a DIFFERENT group can invalidate it.

    Two candidates on the SAME machine are always unioned -- a machine
    fires at most one of its own transitions at a time, a hard
    constraint of the whole `fire-transition` model, not a heuristic.

    Two candidates on DIFFERENT machines are unioned only if actually
    re-simulating shows a real conflict: candidate A's already-known
    dry-fire output is materialized as a real temp --world file, and B
    is dry-fired AGAIN against (state's current world + that file). If
    B is no longer legal, or produces different output than its
    original dry-fire (meaning its provenance citation would have
    changed), A and B are NOT independent and must share a group. This
    is the real guard evaluator doing the check, not a text-scanned
    approximation of which facts overlap -- O(candidates^2) extra
    dry-fire subprocess calls in the worst case, which is fine at the
    scale every scenario built so far actually reaches; a real,
    disclosed cost to revisit if the cannon ever produces enough
    candidates in one round to make that quadratic cost matter.
    """
    parent = {c.id: c.id for c, _ in legal}

    def find(x: str) -> str:
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(a: str, b: str) -> None:
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[ra] = rb

    by_id = {c.id: (c, out) for c, out in legal}

    # Hard rule: same machine -> same group, always.
    by_machine: dict[str, list[str]] = {}
    for c, _ in legal:
        by_machine.setdefault(c.machine, []).append(c.id)
    for ids in by_machine.values():
        for other in ids[1:]:
            union(ids[0], other)

    # Real-simulation rule: different machines, but does firing one
    # actually change the other's legality/output?
    conflict_dir = world_dir / "_conflict_probe"
    conflict_dir.mkdir(exist_ok=True)
    checked: set[tuple[str, str]] = set()
    ids = [c.id for c, _ in legal]
    for i, a_id in enumerate(ids):
        for b_id in ids[i + 1 :]:
            if find(a_id) == find(b_id):
                continue  # already grouped (e.g. via a same-machine chain)
            key = (a_id, b_id)
            if key in checked:
                continue
            checked.add(key)
            a_cand, a_out = by_id[a_id]
            b_cand, b_out = by_id[b_id]
            probe_file = conflict_dir / f"probe-{sanitize_node(a_id)}.dmml"
            probe_file.write_text(a_out)
            ok, out2 = dry_fire(b_cand, state, extra_world_files=[str(probe_file)])
            if (not ok) or out2 != b_out:
                union(a_id, b_id)

    groups: dict[str, list[tuple[Candidate, str]]] = {}
    for c, out in legal:
        groups.setdefault(find(c.id), []).append((c, out))
    return list(groups.values())


def call_jev_batch(api_key: str, model: str, instructions: str, state_summary: str, groups: list[list[tuple]]) -> dict:
    questions = {}
    for i, group in enumerate(groups):
        questions[f"gen_{i}"] = {
            "type": "choice",
            "instructions": instructions,
            "criteria": {c.id: c.description for c, _ in group},
        }
    body = {"state": state_summary, "model": model, "questions": questions}
    req = urllib.request.Request(
        JEV_ENDPOINT,
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as e:
        print(f"fatal: Jev call failed: {e.code} {e.read().decode()}", file=sys.stderr)
        sys.exit(3)


def build_state_summary(round_no: int, state: RunState, legal: list[tuple]) -> str:
    return (
        f"Round {round_no}. World has {len(state.world_files)} committed fact files, "
        f"{state.total_firings} prior firings, {len(state.known_nodes)} known nodes. "
        f"{len(legal)} actions are legal this round."
    )


def split_output_blocks(output: str) -> list[tuple[str, str]]:
    """Split fire-transition's stdout into its top-level commit/machine
    blocks. Needed once a firing includes a spawn effect: DMML.Fire.
    renderFiredCommits then prints MULTIPLE commits (the primary
    ordinary-facts commit, plus one commit per spawned machine's own
    encoded facts -- the one-commit-per-fact constraint from
    DMML.MachineFacts), followed by a human-readable `machine` block per
    spawn (DMML.Fire.renderFiredMachine) -- two different top-level
    grammars (DMML.Ast.TopCommit vs TopMachine), which a --world file
    and a --machine file can never both be parsed as. A `machine` block
    itself contains internal blank lines (between its own states and
    transitions), so this can't split on blank lines the way the old
    single-file dump implicitly assumed -- it splits on any line
    starting literally `commit ` or `machine ` at column 0, mirroring
    how DMML.Surface's own top-level parser tells the two productions
    apart.
    """
    blocks: list[tuple[str, str]] = []
    kind: str | None = None
    lines: list[str] = []

    def flush():
        if kind is not None:
            body = "\n".join(lines).rstrip("\n") + "\n"
            if body.strip():
                blocks.append((kind, body))

    for line in output.split("\n"):
        if line.startswith("commit "):
            flush()
            kind, lines = "commit", [line]
        elif line.startswith("machine "):
            flush()
            kind, lines = "machine", [line]
        else:
            lines.append(line)
    flush()
    return blocks


def sanitize_node(node: str) -> str:
    return node.replace("/", "_")


def discover_fact_native(state: RunState) -> list[str]:
    """Transparency pass, not a candidate source: runs the real
    `list-candidates` binary (DMML.MachineFacts.candidateTransitions)
    against the accumulated --world files and returns whatever
    (machine, transition, params) triples it finds among FACT-NATIVE
    machines -- anything a spawn produced, per that module's own
    disclosed scope. Printed each round so it's visible whether more of
    the world is fact-native-discoverable than this config's hand-
    authored + spawns_followup candidates currently expose; NOT wired
    into `legal` directly, because list-candidates reports formal param
    NAMES, not concrete VALUES to fire with -- turning a discovered
    triple into something fireable still needs a real binding decision,
    which spawns_followup already makes explicitly per spawn rather
    than guessed here.
    """
    import shlex

    cmd = shlex.split(os.environ.get("LIST_CANDIDATES", "list-candidates")) + state.world_files
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    except FileNotFoundError:
        return []
    if proc.returncode != 0:
        return []
    lines = [ln for ln in proc.stdout.splitlines() if ln.strip()]
    return [] if lines == ["list-candidates: no fact-native machines found in scope"] else lines


def apply_winner(candidate: Candidate, output: str, world_dir: Path, round_no: int, state: RunState) -> int:
    blocks = split_output_blocks(output)
    new_nodes = set(NODE_TOKEN_RE.findall(output)) - state.known_nodes
    state.known_nodes |= new_nodes

    commit_idx = 0
    spawned_machine_file: str | None = None
    for kind, body in blocks:
        if kind == "commit":
            out_file = world_dir / f"{round_no:03d}-{candidate.id}-c{commit_idx}.dmml"
            out_file.write_text(body)
            state.world_files.append(str(out_file))
            commit_idx += 1
        else:  # "machine" -- a real EffectSpawn's captured Surface block
            node = body.splitlines()[0][len("machine "):].strip()
            out_file = world_dir / f"machine-{sanitize_node(node)}.dmml"
            out_file.write_text(body)
            if str(out_file) not in state.machine_files:
                state.machine_files.append(str(out_file))
            spawned_machine_file = str(out_file)
            print(f"  spawned machine: {node} -> {out_file}")

    if spawned_machine_file and candidate.spawns_followup:
        fu = candidate.spawns_followup
        if fu["id"] not in state.candidates:
            state.candidates[fu["id"]] = Candidate(
                id=fu["id"],
                machine=spawned_machine_file,
                transition=fu["transition"],
                verb=fu["verb"],
                params=fu.get("params", {}),
                description=fu["description"],
            )
            print(f"  registered follow-up candidate: {fu['id']}")

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

        fact_native = discover_fact_native(state)
        if fact_native:
            print(f"  list-candidates (fact-native, informational): {fact_native}")

        groups = group_into_generations(legal, state, world_dir)
        state_summary = build_state_summary(round_no, state, legal)
        state_summary += f" Grouped into {len(groups)} mutually-independent decision(s) this generation."

        if args.dry_run:
            winners = [group[0] for group in groups]
            jev_response = {"dry_run": True}
        else:
            jev_response = call_jev_batch(args.api_key, jev_cfg["model"], jev_cfg["instructions"], state_summary, groups)
            answers = jev_response.get("answers") if isinstance(jev_response, dict) else None
            if not isinstance(answers, dict):
                print(
                    f"fatal: Jev's round {round_no} response didn't have the expected shape (answers): "
                    f"raw response: {json.dumps(jev_response)}",
                    file=sys.stderr,
                )
                sys.exit(4)
            winners = []
            for i, group in enumerate(groups):
                key = f"gen_{i}"
                try:
                    chosen_id = answers[key]["choice"]
                except (KeyError, TypeError) as e:
                    print(
                        f"fatal: Jev's round {round_no} response missing/malformed answer for {key!r}: {e!r}\n"
                        f"raw response: {json.dumps(jev_response)}",
                        file=sys.stderr,
                    )
                    sys.exit(4)
                match = next(((c, o) for c, o in group if c.id == chosen_id), None)
                if match is None:
                    print(
                        f"fatal: Jev chose {chosen_id!r} for {key!r} in round {round_no}, which is not among "
                        f"that group's own candidates {[c.id for c, _ in group]}",
                        file=sys.stderr,
                    )
                    sys.exit(4)
                winners.append(match)

        applied: list[tuple[Candidate, str]] = []
        minted_total = 0
        stop_reason = None
        for idx, (winner, out) in enumerate(winners):
            if state.total_firings >= budget.max_total_firings:
                print(f"  stopping mid-generation: hit max_total_firings={budget.max_total_firings}")
                break
            if idx > 0:
                # Grouping proved independence at compute time -- this is the
                # cheap, honest check that it actually held once earlier
                # winners in this same generation have already applied,
                # rather than trusting the proof and moving on.
                ok, fresh_out = dry_fire(winner, state)
                if not ok:
                    print(
                        f"fatal: {winner.id!r} was grouped as independent this round but is no longer legal "
                        f"after applying {[w.id for w, _ in applied]} -- a real conflict the grouping missed; "
                        "refusing to silently apply it",
                        file=sys.stderr,
                    )
                    sys.exit(5)
                out = fresh_out
            minted = apply_winner(winner, out, world_dir, round_no, state)
            minted_total += minted
            applied.append((winner, out))
            print(f"round {round_no}: fired {winner.id} ({minted} new node(s), {state.total_firings} total firings)")
            if winner.terminates:
                stop_reason = f"{winner.id} terminates the run -- stopping by choice, not by budget"
                break

        record = {
            "round": round_no,
            "state_summary": state_summary,
            "legal_candidate_ids": [c.id for c, _ in legal],
            "groups": [[c.id for c, _ in g] for g in groups],
            "fact_native_discovered": fact_native,
            "jev_response": jev_response,
            "chosen": [w.id for w, _ in applied],
            "minted_nodes_this_round": minted_total,
            "total_firings": state.total_firings,
        }
        audit.write(json.dumps(record) + "\n")
        audit.flush()

        if stop_reason:
            print(f"=== round {round_no}: {stop_reason} ===")
            break
        round_no += 1

    print(f"world dir: {world_dir}")
    print(f"audit log: {audit_path}")
    audit.close()


def subprocess_mkdtemp() -> str:
    import tempfile

    return tempfile.mkdtemp(prefix="jev-driver-")


if __name__ == "__main__":
    main()
