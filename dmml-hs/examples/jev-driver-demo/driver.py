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

SELF-EXTENDING GROWTH (2026-09-18): with an `extend` block in the
config, the loop no longer draws from a fixed pool. At the end of each
round it fires the CANNON (app/Cannon.hs) at the live frontier -- the
nodes the world currently records as `cleared` -- minting brand-new
architecture, seeding its initial state, and registering its
zero-parameter transitions as ordinary candidates for the next round.
Growth compounds because the cannon's `breed` mode (DMML.Recombine)
takes two machines that ALREADY EXIST, most-recently-minted first, so
generation N+1 is bred from generation N and the vocabulary deepens
instead of re-stamping the same five hand-authored shapes forever.

Without `extend`, the loop terminates at a real fixpoint once the
config's candidates are exhausted -- which is the honest description of
every run before this: bounded by how much architecture a human wrote
into the JSON. With it, the fixpoint moves: the run ends on a budget, on
a terminate candidate, or when the frontier itself is empty.

There is deliberately NO FITNESS FUNCTION. The cannon fires a frontier;
which offspring becomes real is still entirely the chooser's decision,
same as every other candidate. A breeder without selection is half an
evolutionary loop, and the missing half is a decision not yet made, not
an oversight.

KNOWN, DISCLOSED SCOPE LIMITS (not oversights):
  - The frontier is found by TEXT-SCANNING the accumulated world files
    for the dungeon's own reachability convention (`X `cleared`
    mark/yes`), the same convention app/Cannon.hs emits and
    DMML.Recombine.reanchor keys on. Same class of heuristic as the
    minted-node tracking below, and disclosed for the same reason: it
    is not a real DMML.Ast parse, and it would not see a `cleared` fact
    that some later commit retracted (nothing in this dungeon does).
  - Only ZERO-PARAMETER transitions of a minted machine are registered
    as candidates. A parameterized one needs a real binding decision --
    exactly the reason `discover_fact_native` stays informational --
    and guessing values here would be worse than skipping them. Skipped
    ones are printed and logged, never silently dropped.
  - Minted candidates' descriptions are generated MECHANICALLY from the
    machine's actual guard and effect lines. They are deliberately flat
    and factual rather than evocative: a description is what the
    chooser reasons over, and inventing flavour for architecture no
    human has seen would be putting words in the world's mouth.
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
class ExtendPolicy:
    """How much brand-new architecture the cannon may add per round.

    Everything here is deterministic -- no randomness anywhere -- so a
    --dry-run rehearsal of a config produces exactly the machines a
    live run will, which is the only way to check a growth policy
    without spending real chooser calls on it.
    """

    enabled: bool = False
    per_round: int = 1
    max_minted_machines: int = 0
    # Variants cycled through when STAMPING a fresh room, and crossover
    # modes cycled through when BREEDING. Cycled by index rather than
    # chosen, for the determinism above.
    variants: list[str] = field(default_factory=lambda: ["hall", "forge", "vault", "spur"])
    modes: list[str] = field(default_factory=lambda: ["chimera", "union", "splice1"])
    # Below this many machines in scope there is nothing worth crossing,
    # so the cannon stamps a template instead. Two is the real minimum;
    # raising it delays the first breeding.
    breed_after: int = 2


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
    minted_machines: int = 0
    # Machine files the cannon itself minted during this run, oldest
    # first. Kept separate from `machine_files` (which also holds the
    # config's seed machines) so breeding can reach for the most recent
    # OFFSPRING and actually deepen a lineage.
    minted_machine_files: list[str] = field(default_factory=list)


def load_config(path: Path) -> tuple[RunState, Budget, ExtendPolicy, dict]:
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
    ex = cfg.get("extend") or {}
    extend = ExtendPolicy(
        enabled=bool(ex.get("enabled", False)),
        per_round=int(ex.get("per_round", 1)),
        max_minted_machines=int(ex.get("max_minted_machines", 0)),
        variants=list(ex.get("variants", ["hall", "forge", "vault", "spur"])),
        modes=list(ex.get("modes", ["chimera", "union", "splice1"])),
        breed_after=int(ex.get("breed_after", 2)),
    )
    state = RunState(world_files=world_files, machine_files=machine_files, candidates=candidates)
    for wf in world_files:
        state.known_nodes |= set(NODE_TOKEN_RE.findall(Path(wf).read_text()))
    return state, budget, extend, cfg["jev"]


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


CLEARED_RE = re.compile(r"^\s*([A-Za-z0-9_.\-/]+)\s+`cleared`\s+mark/yes\s*$")


def cannon_binary() -> list[str]:
    import shlex

    return shlex.split(os.environ.get("CANNON", "cannon"))


def frontier_nodes(state: RunState) -> list[str]:
    """The live frontier: every node the accumulated world currently
    records as cleared, in first-seen order.

    This is where new architecture can legally attach, because every
    room app/Cannon.hs fires guards on its parent being cleared. Found
    by text-scan over the world files rather than by a real parse --
    see the module docstring's disclosure. Order is stable so that
    cycling through it stays deterministic.
    """
    seen: list[str] = []
    for wf in state.world_files:
        try:
            text = Path(wf).read_text()
        except OSError:
            continue
        for line in text.splitlines():
            m = CLEARED_RE.match(line)
            if m and m.group(1) not in seen:
                seen.append(m.group(1))
    return seen


def parse_machine_text(text: str) -> dict:
    """Read a rendered Surface `machine` block back into the few pieces
    this driver needs: its node, its declared states in order, and each
    transition's ident, formal params, guard lines and effect lines.

    Deliberately a small line reader over DMML.Fire.renderFiredMachine's
    own exact output shape, not a DMML parser -- the driver already
    treats fire-transition's stdout this way (split_output_blocks), and
    a real parse would mean reimplementing DMML.Surface in Python. What
    it must not do is guess: any line it does not recognise inside a
    transition is kept verbatim under `other`, so nothing is silently
    dropped.
    """
    node = ""
    states: list[str] = []
    transitions: list[dict] = []
    section = None
    cur: dict | None = None

    for raw in text.splitlines():
        line = raw.rstrip()
        if not line.strip():
            continue
        if line.startswith("machine "):
            node = line[len("machine "):].strip()
            section = None
            continue
        if line.strip() == "states":
            section = "states"
            continue
        if line.strip().startswith("transition "):
            head = line.strip()[len("transition "):]
            ident, _, rest = head.partition("(")
            params = [x.strip() for x in rest.rstrip(")").split(",") if x.strip()]
            cur = {"ident": ident.strip(), "params": params, "from": None, "to": None,
                   "guards": [], "effects": [], "other": []}
            transitions.append(cur)
            section = "transition"
            continue
        body = line.strip()
        if section == "states":
            states.append(body)
        elif section == "transition" and cur is not None:
            if "->" in body and not body.startswith(("guard ", "assert ", "retract ", "spawn ", "graft ")):
                a, _, b = body.partition("->")
                cur["from"], cur["to"] = a.strip(), b.strip()
            elif body.startswith("guard "):
                cur["guards"].append(body[len("guard "):])
            elif body.startswith(("assert ", "retract ", "spawn ", "graft ")):
                cur["effects"].append(body)
            else:
                cur["other"].append(body)
    return {"node": node, "states": states, "transitions": transitions}


def describe_minted(machine: dict, t: dict, provenance: str) -> str:
    """A candidate description built ONLY from what the machine actually
    says. No invented flavour -- see the module docstring."""
    reqs = [g for g in t["guards"]] or ["nothing"]
    # The lifecycle bookkeeping is noise to a chooser; what it needs is
    # what firing this does to the WORLD.
    world = [
        e for e in t["effects"]
        if not (e.startswith("assert self `state`") or e.startswith("retract self `state`"))
    ] or ["nothing beyond advancing its own state"]
    return (
        f"{t['ident']} on {machine['node']} — {provenance}. "
        f"Requires: {'; '.join(reqs)}. "
        f"Yields: {'; '.join(world)}. "
        f"Lifecycle: {t['from']} -> {t['to']}."
    )


def run_cannon(args: list[str]) -> str | None:
    try:
        proc = subprocess.run(cannon_binary() + args, capture_output=True, text=True, timeout=30)
    except FileNotFoundError:
        print(
            f"fatal: '{' '.join(cannon_binary())}' not found -- build dmml-hs's `cannon` and put it on PATH, "
            "or set CANNON. Required because this config enables `extend`.",
            file=sys.stderr,
        )
        sys.exit(2)
    if proc.returncode != 0:
        print(f"  cannon refused ({' '.join(args)}): {proc.stderr.strip()}")
        return None
    return proc.stdout


def plan_extension(state: RunState, extend: ExtendPolicy, frontier: list[str], seq: int) -> tuple[str, list[str], str]:
    """Decide the next shot, deterministically. Returns (kind, cannon
    args, human provenance).

    STAMP while there is not yet enough material to cross, then BREED --
    and breed with the most recently MINTED machine as parent A, so each
    generation is crossed with the one before it rather than endlessly
    re-crossing the seed pair. Parent B cycles through everything else
    in scope, which keeps the lineage deep without making it narrow.
    """
    anchor = frontier[seq % len(frontier)]
    new_node = f"room/g{seq}"
    pool = state.minted_machine_files + [
        m for m in state.machine_files if m not in state.minted_machine_files
    ]

    def stamp():
        variant = extend.variants[seq % len(extend.variants)]
        return "stamp", [variant, new_node, anchor], f"stamped as a {variant} onto {anchor}"

    if len(pool) < max(2, extend.breed_after):
        return stamp()

    # Parent A is the newest OFFSPRING when one exists, so generation
    # N+1 crosses generation N and the lineage actually deepens.
    # Falling back to the newest seed only happens on a run's very
    # first breeding.
    a = state.minted_machine_files[-1] if state.minted_machine_files else state.machine_files[-1]
    # Parent B cycles over the SEED machines, which is a fixed-length
    # list. Cycling over the whole pool instead looks equivalent and is
    # not: the pool grows by one every time `seq` does, so `seq % len`
    # lands on the same file over and over and breadth silently
    # collapses to a single parent. Caught in a dry run where eight
    # straight generations all crossed with room-wForge.
    seeds = [m for m in state.machine_files if m not in state.minted_machine_files]
    others = [m for m in seeds if m != a] or [m for m in pool if m != a]
    if not others:
        return stamp()
    b = others[seq % len(others)]
    mode = extend.modes[seq % len(extend.modes)]
    return (
        "breed",
        ["breed", mode, new_node, a, b, anchor],
        f"bred by crossing {Path(a).stem} with {Path(b).stem} ({mode}), anchored on {anchor}",
    )


def extend_world(state: RunState, extend: ExtendPolicy, world_dir: Path, round_no: int) -> list[dict]:
    """Fire the cannon at the live frontier and fold whatever it mints
    into the run: machine file, seeded initial state, new candidates.

    This is the step that makes the loop self-extending. Everything it
    adds is ORDINARY -- an ordinary Surface machine file, an ordinary
    world commit seeding its state, ordinary candidates that go through
    the same dry_fire/dedup/grouping path as the hand-authored ones.
    Nothing downstream knows or cares that a machine was minted mid-run
    rather than written into the config, which is the whole point.
    """
    minted: list[dict] = []
    if not extend.enabled:
        return minted

    for _ in range(extend.per_round):
        if state.minted_machines >= extend.max_minted_machines:
            print(f"  extend: at max_minted_machines={extend.max_minted_machines}, stopping growth")
            break
        frontier = frontier_nodes(state)
        if not frontier:
            print("  extend: frontier is empty -- nothing cleared yet to attach to")
            break

        seq = state.minted_machines
        kind, args, provenance = plan_extension(state, extend, frontier, seq)
        out = run_cannon(args)
        if out is None:
            break

        machine = parse_machine_text(out)
        if not machine["node"] or not machine["states"]:
            print(f"  extend: cannon output for {args} had no node/states -- refusing to register it")
            break

        machine_file = world_dir / f"minted-{sanitize_node(machine['node'])}.dmml"
        machine_file.write_text(out)
        state.machine_files.append(str(machine_file))
        state.minted_machine_files.append(str(machine_file))
        state.minted_machines += 1

        # A machine's current state is mutable world data, not structural
        # definition -- app/Cannon.hs deliberately does not emit it, so
        # the caller seeds it. Its FIRST declared state is its initial
        # one, which is the same lifecycle-order convention
        # DMML.Recombine's state alignment already relies on.
        seed = world_dir / f"{round_no:03d}-extend-{sanitize_node(machine['node'])}.dmml"
        seed.write_text(f"commit extends\n  {machine['node']} `state` {machine['states'][0]}\n")
        state.world_files.append(str(seed))
        # Count cannon-minted nodes against the SAME max_minted_nodes cap
        # firings are counted against. Growth is the dominant source of
        # new world once `extend` is on, so a node budget that quietly
        # stopped covering it would be a cap that reads as a bound and
        # is not one.
        fresh = set(NODE_TOKEN_RE.findall(out)) - state.known_nodes
        state.known_nodes |= fresh
        state.minted_nodes += len(fresh)

        registered, skipped = [], []
        for t in machine["transitions"]:
            if t["params"]:
                skipped.append(f"{t['ident']}({', '.join(t['params'])})")
                continue
            cid = f"{sanitize_node(machine['node'])}-{t['ident']}"
            if cid in state.candidates:
                continue
            state.candidates[cid] = Candidate(
                id=cid,
                machine=str(machine_file),
                transition=t["ident"],
                verb="breaches",
                params={},
                description=describe_minted(machine, t, provenance),
            )
            registered.append(cid)

        print(f"  extend: {kind} -> {machine['node']} ({provenance})")
        print(f"          registered {len(registered)} candidate(s): {registered}")
        if skipped:
            print(f"          skipped {len(skipped)} parameterized transition(s), needs real bindings: {skipped}")
        minted.append({
            "kind": kind,
            "node": machine["node"],
            "provenance": provenance,
            "cannon_args": args,
            "machine_file": str(machine_file),
            "registered_candidates": registered,
            "skipped_parameterized": skipped,
        })
    return minted


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

    state, budget, extend, jev_cfg = load_config(args.config)

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
            if extend.enabled:
                print(
                    f"=== round {round_no}: fixpoint -- nothing legal and new, even with growth enabled "
                    f"({state.minted_machines} machine(s) minted). Stopping cleanly ==="
                )
            else:
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

        # Grow the world AFTER this generation has applied, so the cannon
        # fires at the frontier as it actually stands now -- including
        # anything this round's winners just cleared. The machines it
        # mints become ordinary candidates in the NEXT round's `legal`
        # pass, which is exactly what keeps that pass from draining to a
        # fixpoint.
        minted_machines = [] if stop_reason else extend_world(state, extend, world_dir, round_no)

        record = {
            "round": round_no,
            "state_summary": state_summary,
            "legal_candidate_ids": [c.id for c, _ in legal],
            "groups": [[c.id for c, _ in g] for g in groups],
            "fact_native_discovered": fact_native,
            "jev_response": jev_response,
            "chosen": [w.id for w, _ in applied],
            "minted_nodes_this_round": minted_total,
            "minted_machines_this_round": minted_machines,
            "total_firings": state.total_firings,
        }
        audit.write(json.dumps(record) + "\n")
        audit.flush()

        if stop_reason:
            print(f"=== round {round_no}: {stop_reason} ===")
            break
        round_no += 1

    if extend.enabled:
        print(f"minted {state.minted_machines} machine(s) mid-run off the live frontier")
    print(f"world dir: {world_dir}")
    print(f"audit log: {audit_path}")
    audit.close()


def subprocess_mkdtemp() -> str:
    import tempfile

    return tempfile.mkdtemp(prefix="jev-driver-")


if __name__ == "__main__":
    main()
