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
config, the loop no longer draws from a fixed pool. It fires the CANNON
(app/Cannon.hs) to mint brand-new architecture mid-run, seeding its
initial state and registering its zero-parameter transitions as ordinary
candidates. Growth compounds because the cannon's `breed` mode
(DMML.Recombine) crosses two machines that ALREADY EXIST, newest
offspring first, so generation N+1 is bred from generation N and the
vocabulary deepens instead of re-stamping five hand-authored shapes
forever.

GROWTH IS PULLED, NOT PUSHED. The first version of this fired once per
round off a clock, which was backwards, and the reason is worth keeping:
written-world -- this project's sibling -- generates a room when a player
ARRIVES AT AN UNMAPPED FRONTIER POINT, and once made it is never remade.
That is not lazy evaluation as an optimization. It is what a world made
of attention looks like when you implement it, because attention is the
only genuinely scarce resource here (see THE ECONOMY below) and building
where nobody went spends it on nothing.

So: the demand signal is an UNMAPPED EDGE -- a node the world records as
cleared that nothing is built on yet (`unmapped_frontier`). Such an edge
is not exhaustion, it is a request. The run ends only when there is
nothing legal AND no opened way left unentered.

AND JEV CHOOSES WHERE. When several edges stand open, "where do you press
on?" rides along in the SAME batched call as that round's action choices
-- one more question, no extra call. It is phrased IN-FICTION on purpose
(see FRONTIER_INSTRUCTIONS): asking "which node should the generator
expand" invites an answer from a level designer with taste, while asking
a delver where they are going gets an answer from desire. A world built
where a character wants to go is producing in response to wanting; a
world built where a reader's eye lingers is a slot machine. The frame is
what holds those apart. The anchor used to be chosen here by a modulo
over the frontier -- the single most consequential knob in this file,
since it decides the world's shape.

TWO DESIRES. Pull-only growth has a cost: nothing ever arrives unbidden,
so nothing in the dungeon appears to want anything, and the world is
summoned rather than inhabited. `unbidden_every: N` buys that back --
every Nth round the cannon also builds somewhere the delve did NOT
choose. It is an explicit line item because it is not free: it spends
the scarce resource on what nobody asked for, which is exactly what
makes a world feel like it has its own weather.

THE ECONOMY, and why there is still NO FITNESS FUNCTION. There is no
scoring function anywhere and there does not need to be one. The real
economy is this loop's token budget and its reader's attention: a room
nobody finds interesting costs the same to generate and describe as one
that changes the delve, so boring architecture is literally expensive,
and selection is just what gets pressed into again. A fitness function
would be transcendent -- someone writing down what counts as good.
Scarcity is immanent. The budget knobs below are that economy, and they
are still denominated in counts rather than calls, which is a known
mis-denomination, not a claim that counts are the right unit.

KNOWN, DISCLOSED SCOPE LIMITS (not oversights):
  - Whether a node is "built on" is decided by scanning machines in
    scope for a guard anchored on it (`guarded_nodes`). A machine that
    relates to a place without guarding on it is invisible to that, so
    such a place reads as unmapped and can be built on twice.
  - The frontier is found by TEXT-SCANNING the accumulated world files
    for the dungeon's own reachability convention (`X `cleared`
    mark/yes`), the same convention app/Cannon.hs emits and
    DMML.Recombine.reanchor keys on. Same class of heuristic as the
    minted-node tracking below, and disclosed for the same reason: it
    is not a real DMML.Ast parse, and it would not see a `cleared` fact
    that some later commit retracted (nothing in this dungeon does).
  - A minted machine's parameterized transitions are registered only
    when every param is a MINTING one -- guarded by nothing, asserted
    into existence (see `classify_params`). Those get a fresh name read
    off the world each round. A param that appears in a GUARD is a real
    binding decision -- exactly the reason `discover_fact_native` stays
    informational -- and is still skipped, as is any param this reader
    cannot account for. Skipped ones are printed and logged, never
    silently dropped.
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
import math
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
from collections import Counter
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
    # Params set by answering an ambiguous-binding question, cleared the
    # moment this candidate fires. A binding is a choice about ONE
    # firing -- which rock, this time -- not a standing configuration.
    # Left in place it would silently re-take a rock already spent, and
    # the guard would then refuse for a reason that looks nothing like
    # the real one.
    bound_params: set[str] = field(default_factory=set)
    # Formal params this transition MINTS rather than binds, mapped to
    # the substance each one arrives as ("in quarry/north"). A minting
    # param names something that does not exist yet -- it appears only
    # as an effect subject, never in a guard -- so there is nothing for
    # the engine to enumerate and no question to ask: the world is open,
    # and naming the new thing is the driver's job. Refreshed every round
    # (see refresh_minting_params) because a source that brought the
    # SAME rock every time would not be a source.
    minting_params: dict[str, str] = field(default_factory=dict)
    # Last round's interest score, 0-1, or None if never asked. Recorded
    # rather than acted on for actions -- the group's `choice` decides
    # what happens; this says what the chooser actually WANTED, which is
    # a different and more legible thing in an audit log.
    interest: float | None = None


@dataclass
class ExtendPolicy:
    """How much brand-new architecture the cannon may add per round.

    Everything here is deterministic -- no randomness anywhere -- so a
    --dry-run rehearsal of a config produces exactly the machines a
    live run will, which is the only way to check a growth policy
    without spending real chooser calls on it.
    """

    enabled: bool = False
    max_minted_machines: int = 0
    # How often the world builds somewhere NOBODY asked for: every Nth
    # round, 0 to never. This is the second desire, and it is a line
    # item on purpose. Growth is otherwise entirely pull-based -- the
    # world only appears where the delve pressed for it -- and a world
    # that is purely pull is summoned rather than inhabited: nothing
    # ever arrives unbidden, no weather, nothing else in the dungeon
    # wanting anything. Buying that back costs the one genuinely scarce
    # resource (tokens, attention), so it is spent deliberately and
    # reported, never sprinkled in for free.
    unbidden_every: int = 0
    # Every Nth round, spend on a VOCABULARY-OPENING move whatever the
    # chooser thinks of it. 0 to never.
    #
    # A third line item, for the same reason as the second and a sharper
    # version of it. `unbidden_every` exists because a purely pull-based
    # world is summoned rather than inhabited. This exists because
    # INTEREST IS A LOCAL SIGNAL AND SHAPE-OPENING HAS NON-LOCAL VALUE,
    # and no amount of better phrasing fixes that.
    #
    # A new shape makes no round better. Asked "do you want this?", a
    # reader is answering about THIS round, and the honest answer is no:
    # an `imply` consumes nothing, moves nothing, and produces no event.
    # Its whole value is that it enlarges the option space for every
    # round after -- and a chooser cannot price the future size of its
    # own option space, because pricing it would require already having
    # the options.
    #
    # Measured: the first live run scored implies 0.19-0.36 and built
    # none. Rewriting the description to name the consequence lifted that
    # to 0.47 and got two built, which is better salesmanship and not a
    # better economics. So it becomes an allocation: not a question, a
    # standing decision to spend, reported every time it is spent.
    shape_every: int = 0
    # How many proposals of each connective kind a round may OFFER.
    #
    # Was hard-coded at 2 (1 for wellspring), and that is the real
    # ceiling on densification rather than anything about the chooser: a
    # world of 30 cleared nodes has 435 possible bridges and could be
    # shown two of them. N-squared opportunity, constant-sized menu.
    # Worth raising deliberately and not silently, because every extra
    # proposal is an extra interest question and the budget is tokens.
    proposals_per_kind: int = 2
    # Growth spends its per-round allowance on frontier EDGES before
    # RELATIONS, which is right for reaching new ground and exactly
    # backwards for density -- relations lose ties to edges
    # systematically, and the log said so ("4 relation(s) wanted but
    # this round's growth allowance is already spent"). Flip it when
    # thickening the web matters more than extending it.
    relations_first: bool = False
    # Relational predicates the world may come to hold between two
    # things that already exist -- the pairing axis `bridge` was the
    # only member of. Measured on the maximal run: 81 of 112 machines
    # built were bridges, and predicate evenness came out at 0.21 on a
    # scale where 1.00 is every relation equally used. Not a taste
    # problem: bridge was the only operator proposable N-squared.
    regards: list[str] = field(default_factory=lambda: ["admires", "inspiredBy", "adaptsTo", "desires"])
    # What a corridor is cut OUT of, as "<pred> <obj>". Empty means the
    # old free bridge, which the pure-reachability demos still need
    # because they have no matter to spend.
    bridge_cost: str = ""
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
class InterestPolicy:
    """How much the world grows per round, and on whose say-so.

    Jev's own docs are explicit that "all questions are evaluated in
    parallel, so adding more questions to a call typically doesn't add
    any latency" -- and this loop had been spending that parallelism on
    ACTIONS while keeping GROWTH strictly serial: one frontier edge, one
    relation, one machine per round, however many stood open.

    That serialization was not a scarcity decision, it was a consequence
    of the PRIMITIVE. A `choice` question is a softmax over alternatives:
    its probabilities sum to 1, so it measures relative preference and
    structurally cannot say "all of these are worth building" or "none of
    these are". Asking "which one edge?" forces exactly one answer even
    when six are interesting and even when none are.

    An INTEREST score is a different question, and `noul` is its
    primitive -- an independent 0-to-1 per option, no competition between
    them. Six edges can all come back 0.8. Six edges can all come back
    0.1. So the loop can now build everywhere worth building in ONE pass,
    and -- the half that matters as much -- build NOTHING when nothing is
    interesting, which a choice could never say.

    This is still selection by attention rather than a fitness function.
    Nothing here scores a machine's structure; it asks whether a reader
    wants to go and look. That keeps the scarcity immanent (the token
    budget, the reader) rather than transcendent, which is the line this
    project has held since the fitness question was first deferred.
    """

    enabled: bool = False
    # Selection is RANKED, not thresholded, and the ranking is against
    # the run's own history rather than any fixed number.
    #
    # Measured over 122 live scores in two real runs: mean 0.412, sd
    # 0.086, range 0.21-0.59. Jev never once exceeded 0.6. So every
    # absolute cut this file tried was measuring the calibration of the
    # scale rather than the content of the answer -- a 0.6 bar builds
    # nothing ever, a 0.35 bar admits nearly everything, and neither
    # number says anything about the world. The first version of this
    # used a hard floor; the second a floor plus a relative band, which
    # was better and still a cliff.
    #
    # What the signal is actually good for is ORDER. So: standardize each
    # score against the run's running mean and sd, push it through a
    # sigmoid, and take that as the probability this option gets built.
    # Scale-free by construction -- it cannot be broken by Jev living in
    # 0.2-0.6 rather than 0-1, because it never reads the raw number.
    #
    # Crucially this keeps BOTH capabilities a `choice` lacks: a round
    # where everything sits below the run's own baseline builds nothing,
    # and a round where everything sits above it builds everything.
    #
    # temperature: sds per unit of logit. 1.0 means an option one sd
    # above the run's mean is built ~73% of the time.
    temperature: float = 1.0
    # Prior for the first rounds, before the run has enough history to
    # standardize against -- the measured live figures above, stated as
    # what they are and washed out by real data within a few rounds.
    prior_mean: float = 0.412
    prior_sd: float = 0.086
    prior_weight: int = 12
    # An absolute BACKSTOP, not a knob: it exists to catch a chooser
    # actively saying no to everything, not to decide what is
    # interesting. Deliberately set below the entire observed range
    # (min 0.21), so on any run resembling the measured ones it never
    # fires -- which is the correct behaviour for a guard.
    refuse_below: float = 0.15
    # Hard cap on machines minted in one round however many edges clear
    # the bar. The fan-out is the point, but an unbounded fan-out spends
    # the whole budget in round one and calls it emergence.
    max_growth_per_round: int = 4
    # Hard cap on interest questions per round. Latency is free; tokens
    # are not, and a 100-candidate round would otherwise put 100 extra
    # questions on the wire. Options past the cap simply are not asked
    # about, and that is reported rather than silently truncated.
    max_questions: int = 40
    # Ask an interest score for every ACTION candidate too, not just for
    # growth. Off by default: a group's `choice` already resolves which
    # action happens, so per-candidate interest is observability rather
    # than control -- real, and worth paying for deliberately.
    score_actions: bool = True


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
    # Every interest score this run has seen, so selection can rank
    # against what this world's chooser actually does rather than
    # against a number picked in advance.
    interest_seen: list[float] = field(default_factory=list)
    # Prose catalogs, and this round's rendered sentences per node.
    # Refreshed once per round rather than per lookup: rendering shells
    # out to `render-prose`, which re-parses every world file, and the
    # measured lesson from scan-candidates is that a per-subject
    # subprocess over an unchanged world is what makes a loop slow.
    prose_catalogs: list[str] = field(default_factory=list)
    prose: dict[str, list[str]] = field(default_factory=dict)

    def interest_baseline(self, policy) -> tuple[int, float, float]:
        """(n, mean, sd) over this run's scores, blended with the stated
        prior so early rounds are not standardized against two samples."""
        n = len(self.interest_seen)
        w = policy.prior_weight
        if n == 0:
            return 0, policy.prior_mean, policy.prior_sd
        mean = (sum(self.interest_seen) + w * policy.prior_mean) / (n + w)
        var = sum((x - mean) ** 2 for x in self.interest_seen) + w * policy.prior_sd**2
        return n, mean, math.sqrt(var / (n + w))


def load_config(path: Path) -> tuple[RunState, Budget, ExtendPolicy, InterestPolicy, dict]:
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
        max_minted_machines=int(ex.get("max_minted_machines", 0)),
        unbidden_every=int(ex.get("unbidden_every", 0)),
        shape_every=int(ex.get("shape_every", 0)),
        proposals_per_kind=int(ex.get("proposals_per_kind", 2)),
        relations_first=bool(ex.get("relations_first", False)),
        regards=list(ex.get("regards", ["admires", "inspiredBy", "adaptsTo", "desires"])),
        bridge_cost=str(ex.get("bridge_cost", "")),
        variants=list(ex.get("variants", ["hall", "forge", "vault", "spur"])),
        modes=list(ex.get("modes", ["chimera", "union", "splice1"])),
        breed_after=int(ex.get("breed_after", 2)),
    )
    pr = cfg.get("prose") or {}
    prose_catalogs = [str((base / c).resolve()) for c in pr.get("catalogs", [])]
    it = cfg.get("interest") or {}
    interest = InterestPolicy(
        enabled=bool(it.get("enabled", False)),
        temperature=float(it.get("temperature", 1.0)),
        prior_mean=float(it.get("prior_mean", 0.412)),
        prior_sd=float(it.get("prior_sd", 0.086)),
        prior_weight=int(it.get("prior_weight", 12)),
        refuse_below=float(it.get("refuse_below", 0.15)),
        max_growth_per_round=int(it.get("max_growth_per_round", 4)),
        max_questions=int(it.get("max_questions", 40)),
        score_actions=bool(it.get("score_actions", True)),
    )
    state = RunState(
        world_files=world_files, machine_files=machine_files, candidates=candidates,
        prose_catalogs=prose_catalogs,
    )
    for wf in world_files:
        state.known_nodes |= set(NODE_TOKEN_RE.findall(Path(wf).read_text()))
    return state, budget, extend, interest, cfg["jev"]


# The dry-run chooser -------------------------------------------------------
#
# `--dry-run` used to take `group[0]` -- the first legal candidate, the
# first unmapped edge, the first binding option. That is not "no
# chooser", it is a chooser with a very strong opinion, and the opinion
# is an artifact of list order. Measured 2026-09-18: in a 20-round
# quarry-delve run, several bred rooms competing for the same scarce rock
# never fired once, not because they could not but because a sibling was
# listed ahead of them every single round. Reading that as "those
# machines are dead" would have been reading the sort order as biology.
#
# So: PERLIN NOISE, and deliberately not a hash.
#
# A hash would de-bias it -- uncorrelated, uniform, fine. But a hash is
# memoryless, and a real chooser is not. Jev has intent that persists
# across rounds: a crew that has been digging keeps digging for a while,
# then its attention moves. Perlin is smooth, so sampling it along a time
# axis gives exactly that -- a preference field that DRIFTS. A dry run
# then rehearses the shape of a live run (coherent, with momentum)
# instead of the shape of a shuffle.
#
# Still fully deterministic, which `ExtendPolicy` depends on: fixed
# permutation table, fixed constants, no `random` module anywhere (so
# nothing else seeding the global RNG can perturb a rehearsal). Same
# config, same run, every time.
#
# Each candidate gets a fixed LANE in the field, derived from its id --
# never from its index in any list, which is the whole point.


def _perm_table(seed: int) -> list[int]:
    """Classic Perlin permutation, shuffled by an explicit LCG.

    Hand-rolled rather than `random.shuffle` so this table cannot be
    changed by anything else in the process seeding the global RNG -- a
    rehearsal that silently differs run to run would be worse than the
    positional bias it replaces.
    """
    table = list(range(256))
    state = seed & 0xFFFFFFFF
    for i in range(255, 0, -1):
        state = (state * 1664525 + 1013904223) & 0xFFFFFFFF
        j = state % (i + 1)
        table[i], table[j] = table[j], table[i]
    return table + table


_PERM = _perm_table(0x736D6F6B65)
# Rounds per unit of noise. Smaller drifts more slowly (a chooser that
# sticks with a thing); larger approaches white noise. 0.35 gives roughly
# a three-round attention span, which is about what the live runs show.
DRIFT_TIME_SCALE = 0.35
# How far an option's lane slides sideways per round.
#
# Without it, each option sits on ONE curve through the field forever, and
# over a finite run some curves simply run higher than others -- a
# persistent per-option preference that is not positional bias but is
# still bias. Sliding the lanes decorrelates the long-run means while
# leaving short-term coherence intact.
#
# Chosen by sweeping 0.0-0.3 over 120 key-sets (sizes 3/6/12, 200 rounds
# each) rather than one draw -- on a single draw 0.1 looked twice as good
# as 0.05, and averaged it is not. Mean spread (max-min share of wins,
# normalized) 0.88 -> 0.60, mean run-length 1.88 -> 1.84. The curve is
# flat from 0.02 to 0.15, so this constant is not load-bearing.
#
# The residual 0.60 does not go away at any setting: argmax over a smooth
# field is lumpier than uniform, and that is the price of coherence, not
# a bug left in.
DRIFT_LANE_SCALE = 0.05


def _fade(t: float) -> float:
    return t * t * t * (t * (t * 6 - 15) + 10)


def _grad(h: int, x: float, y: float) -> float:
    h &= 3
    return (x if h in (0, 1) else -x) + (y if h in (0, 2) else -y)


def perlin2(x: float, y: float) -> float:
    """2D Perlin noise, roughly [-1, 1]. Textbook implementation."""
    x0, y0 = math.floor(x), math.floor(y)
    xi, yi = x0 & 255, y0 & 255
    xf, yf = x - x0, y - y0
    u, v = _fade(xf), _fade(yf)
    aa = _PERM[_PERM[xi] + yi]
    ab = _PERM[_PERM[xi] + yi + 1]
    ba = _PERM[_PERM[xi + 1] + yi]
    bb = _PERM[_PERM[xi + 1] + yi + 1]
    lo = _grad(aa, xf, yf) + u * (_grad(ba, xf - 1, yf) - _grad(aa, xf, yf))
    hi = _grad(ab, xf, yf - 1) + u * (_grad(bb, xf - 1, yf - 1) - _grad(ab, xf, yf - 1))
    return lo + v * (hi - lo)


def _lane(key: str) -> float:
    """A stable coordinate for one option, from its NAME.

    From the name and nothing else -- not its index, not the order it was
    discovered, not how many siblings it has. That is what makes the
    chooser indifferent to list order, and it also means an option keeps
    its lane as the world grows around it.
    """
    h = hashlib.sha256(key.encode()).digest()
    return int.from_bytes(h[:8], "big") / 2.0**64 * 256.0


def drift(key: str, round_no: int, salt: str = "") -> float:
    """How much this chooser wants `key` this round."""
    return perlin2(
        round_no * DRIFT_TIME_SCALE,
        _lane(f"{salt}|{key}") + round_no * DRIFT_LANE_SCALE,
    )


def drift_pick(keys, round_no: int, salt: str = ""):
    """The most-wanted option this round. Ties break on the key itself,
    so the result is total and reproducible even in the degenerate case
    where two options land on identical noise."""
    return max(keys, key=lambda k: (drift(str(k), round_no, salt), str(k)))


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


AMBIG_VAR_RE = re.compile(r"^ambiguous-binding:\s*(\S+)\s*$", re.M)
AMBIG_CAND_RE = re.compile(r"^candidate:\s*(\S+)\s*$", re.M)


def parse_ambiguity(stderr: str) -> tuple[str, list[str]] | None:
    """Read `fire-transition`'s ambiguous-binding refusal back out.

    A `?binder` that matches several witnesses is refused rather than
    resolved, deliberately: picking one would be an arbitrary choice with
    fully observable consequences, so the engine hands it over instead
    (DMML.Guard.GuardAmbiguousBinding). It prints the candidates on
    stable `ambiguous-binding:` / `candidate:` lines precisely so a
    driver can turn the refusal into a real question without scraping
    prose.

    That is the whole point of refusing: the engine enumerates the
    options, the chooser picks, and the pick comes back as an ordinary
    `--param` that pre-binds the binder. A refusal is a question with its
    answers already listed, not a dead end.
    """
    var = AMBIG_VAR_RE.search(stderr)
    if not var:
        return None
    options = AMBIG_CAND_RE.findall(stderr)
    return (var.group(1), options) if options else None


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


def scan_candidates(state: RunState) -> tuple[list[tuple[Candidate, str]], list[tuple[Candidate, str, list[str]]]]:
    """One dry-fire pass, two answers: what is legal now, and what is
    refused ONLY because a binder is ambiguous.

    Deliberately one pass. Dry-firing is a subprocess per candidate that
    re-parses every accumulated world file, which is far and away the
    most expensive thing this loop does; scanning twice to answer two
    questions about the same call would double the dominant cost for
    nothing.

    A pending candidate is not illegal -- it is legal several ways at
    once, and the engine refuses to choose among them. Each is a question
    with its options already enumerated by the guard evaluator.
    """
    legal: list[tuple[Candidate, str]] = []
    pending: list[tuple[Candidate, str, list[str]]] = []
    for cand, status, out, var, options in scan_results(state):
        if status == "legal":
            h = hashlib.sha256(out.encode()).hexdigest()
            if h != cand.last_hash:  # legal, but identical to its own last firing -- not new
                legal.append((cand, out))
        elif status == "ambiguous":
            pending.append((cand, var, options))
    return legal, pending


def scan_binary() -> list[str] | None:
    import shlex

    raw = os.environ.get("SCAN_CANDIDATES")
    return shlex.split(raw) if raw else None


def scan_results(state: RunState):
    """Every candidate's verdict, preferring the BATCH path.

    `scan-candidates` materializes the world once and reuses that
    snapshot for every candidate; the per-candidate `fire-transition`
    fallback re-parses every world file per call, so a round costs
    (candidates x world-files) parses of files that did not change. That
    repetition, not the engine, is what made large runs slow -- measured
    at 0.39s per dry-fire against 3200 facts, essentially all of it
    re-reading.

    The fallback is kept deliberately: it needs no extra binary, it is
    what every earlier run used, and having both means the batch path can
    be checked against it rather than trusted.
    """
    binary = scan_binary()
    if not binary:
        for c in state.candidates.values():
            ok, out = dry_fire(c, state)
            if ok:
                yield (c, "legal", out, None, None)
            else:
                amb = parse_ambiguity(out)
                yield (c, "ambiguous", "", amb[0], amb[1]) if amb else (c, "blocked", "", None, None)
        return

    cands = list(state.candidates.values())
    payload = [
        {"id": c.id, "machine": c.machine, "transition": c.transition, "verb": c.verb, "params": c.params}
        for c in cands
    ]
    import tempfile

    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as fh:
        json.dump(payload, fh)
        path = fh.name
    cmd = binary + [path]
    for w in state.world_files:
        cmd += ["--world", w]
    for m in state.machine_files:
        cmd += ["--machine", m]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
    os.unlink(path)
    if proc.returncode != 0:
        print(f"fatal: scan-candidates failed: {proc.stdout.strip()} {proc.stderr.strip()}", file=sys.stderr)
        sys.exit(2)
    by_id = {c.id: c for c in cands}
    for row in json.loads(proc.stdout):
        c = by_id[row["id"]]
        st = row["status"]
        yield (c, st, row.get("output", ""), row.get("var"), row.get("candidates"))


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


# Asked IN-FICTION, on purpose. The same information could be got by
# asking "which node should the generator expand next," but that is a
# different question to a different addressee: it invites Jev to answer
# as a level designer, with taste, when what steers this well is the
# delver answering with desire. A world built where a character wants to
# go is producing in response to wanting. A world built where a reader's
# eye lingers is a slot machine. Keeping the question inside the frame
# is what holds those apart.
# Also in-fiction, for the same reason the frontier question is. The
# difference between "which relation should the generator add" and "what
# should be built between these" is the difference between a level
# designer and someone who lives there.
CONNECT_INSTRUCTIONS = (
    "These are things that already exist in the world and do not yet have anything to do with "
    "each other. Each option would make one relation real -- a way between two places, a way "
    "one material turns into another, a way something arrives, a way two places come to see "
    "each other. Nothing here extends the world outward; all of it thickens what is already "
    "here. Choose by what you want to be true of this place, not by what would tidy the map."
)

FRONTIER_INSTRUCTIONS = (
    "You stand at the edge of what has been built. Each option is a way you have opened "
    "but not yet pressed into -- there is nothing beyond any of them yet, and whichever "
    "you choose is where the dungeon will take shape. The ones you do not choose stay as "
    "they are: unbuilt, still open, still there to come back to. Choose as the delver, by "
    "what you want, not by what would make a tidy map."
)


def call_jev_batch(
    api_key: str,
    model: str,
    instructions: str,
    state_summary: str,
    groups: list[list[tuple]],
    frontier: list[tuple[str, str]] = (),
    bindings: list[tuple[str, str, str, list[str]]] = (),
    connect: list[tuple[str, str, list[str]]] = (),
    interest: list[tuple[str, str]] = (),
) -> dict:
    questions = {}
    for i, group in enumerate(groups):
        questions[f"gen_{i}"] = {
            "type": "choice",
            "instructions": instructions,
            "criteria": {c.id: c.description for c, _ in group},
        }
    for key, var, desc, options in bindings:
        questions[key] = {
            "type": "choice",
            "instructions": (
                desc
                + " Choose as the delver, by what you want. The engine found these and refused to"
                + " pick for you."
            ),
            "criteria": {o: o for o in options},
        }
    if len(connect) > 1:
        questions["connect"] = {
            "type": "choice",
            "instructions": CONNECT_INSTRUCTIONS,
            "criteria": {cid: desc for cid, desc, _args in connect},
        }
    if frontier:
        questions["frontier"] = {
            "type": "choice",
            "instructions": FRONTIER_INSTRUCTIONS,
            "criteria": dict(frontier),
        }
    # INTEREST. One `noul` per option -- independent, so several can all
    # come back high and several can all come back low, which is the
    # thing a `choice` cannot express (its probabilities sum to 1, so it
    # can only ever say which of these, never how many of these or
    # whether any). Free in latency by Jev's own account: "all questions
    # are evaluated in parallel."
    for key, statement in interest:
        questions[key] = {
            "type": "noul",
            # The run's own framing rides on EVERY interest question, not
            # just the action choices. The first live run of this asked
            # bare "is this a way you want to go?" with no delver, no
            # dungeon and no stakes attached, and got 0.43-0.48 across
            # the board -- the correct answer to a question nobody could
            # answer. `state` is shared across questions; `instructions`
            # is not, and interest was the one place it went missing.
            "instructions": instructions + "\n\n" + statement,
            "criteria": {
                "true": "Yes -- there is something here you actually want.",
                "false": "No -- this is not where your attention goes.",
            },
        }
    body = {"state": state_summary, "model": model, "questions": questions}
    # Performance instrumentation. Jev is the one genuinely scarce thing
    # in this loop and until now nothing measured it: not how long a call
    # takes, not what it costs, not whether it actually discriminates.
    # The vendor's own claim -- "all questions are evaluated in parallel,
    # so adding more questions typically doesn't add any latency" -- is
    # the load-bearing assumption behind the whole fan-out design and has
    # never been tested against a real spread of question counts.
    t0 = time.monotonic()
    req = urllib.request.Request(
        JEV_ENDPOINT,
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            out = json.loads(resp.read())
    except urllib.error.HTTPError as e:
        print(f"fatal: Jev call failed: {e.code} {e.read().decode()}", file=sys.stderr)
        sys.exit(3)
    elapsed = time.monotonic() - t0
    kinds = Counter(q["type"] for q in questions.values())
    answers = out.get("answers") or {}
    # Decisiveness, per primitive and on one scale: how far from a
    # shrug. A `choice` reports its own confidence; a `noul` does not,
    # because its single probability IS the answer and its certainty at
    # once -- so 0.5 is maximal indecision and either end is maximal
    # decision.
    conf = [a["confidence"] for a in answers.values()
            if isinstance(a, dict) and isinstance(a.get("confidence"), (int, float))]
    nouls = [a["noul"] for a in answers.values()
             if isinstance(a, dict) and isinstance(a.get("noul"), (int, float))]
    out["_perf"] = {
        "seconds": round(elapsed, 3),
        "questions": len(questions),
        "by_kind": dict(kinds),
        "input_tokens": (out.get("usage") or {}).get("input_tokens"),
        "output_tokens": (out.get("usage") or {}).get("output_tokens"),
        "state_chars": len(state_summary),
        "choice_confidence_mean": round(sum(conf) / len(conf), 3) if conf else None,
        "noul_decisiveness_mean": round(sum(abs(n - 0.5) * 2 for n in nouls) / len(nouls), 3) if nouls else None,
        "noul_spread": round(max(nouls) - min(nouls), 3) if len(nouls) > 1 else None,
        "served_model": out.get("model"),
    }
    perf = out["_perf"]
    print(
        f"  jev: {perf['questions']} question(s) in {perf['seconds']}s, "
        f"{perf['input_tokens']}->{perf['output_tokens']} tokens, "
        f"noul decisiveness {perf['noul_decisiveness_mean']}, spread {perf['noul_spread']}"
    )
    return out


INTEREST_FRONTIER = (
    "Beyond {node} nothing has been built yet. {why} "
    "Is this a way you actually want to go -- somewhere you would spend a turn to see?"
)
INTEREST_CONNECT = (
    "{desc} Is this a relation you actually want in the world -- one that would make it "
    "feel more alive rather than merely larger?"
)
INTEREST_ACTION = (
    "{desc} Setting aside whether it is the best move available, is this something you "
    "actually want to do?"
)


def interest_questions(
    policy: InterestPolicy,
    legal: list[tuple],
    frontier: list[str],
    connect: list[tuple[str, str, list[str]]],
    state: RunState,
) -> tuple[list[tuple[str, str]], int]:
    """Every option this round, each as its own independent yes/no.

    Growth first, actions second, because growth is what interest
    actually DECIDES -- for actions it is observability, and if the cap
    bites it should bite the thing that is merely being watched.
    Returns the questions and how many options went unasked, so a
    truncated round says so instead of quietly shrinking.
    """
    asked: list[tuple[str, str]] = []
    for n in frontier:
        asked.append(
            (f"interest_frontier|{n}", INTEREST_FRONTIER.format(node=n, why=describe_frontier_node(n, state)))
        )
    for cid, desc, _args in connect:
        asked.append((f"interest_connect|{cid}", INTEREST_CONNECT.format(desc=desc)))
    if policy.score_actions:
        for c, _out in legal:
            asked.append((f"interest_action|{c.id}", INTEREST_ACTION.format(desc=c.description)))
    over = max(0, len(asked) - policy.max_questions)
    return asked[: policy.max_questions], over


def interest_probability(score: float, state: RunState, policy: InterestPolicy) -> float:
    """This option's chance of being built, from its rank against the
    run's own history. Scale-free: the raw number is never read, only
    its standing.
    """
    n, mean, sd = state.interest_baseline(policy)
    if score < policy.refuse_below:
        return 0.0
    z = (score - mean) / max(sd, 1e-6)
    return 1.0 / (1.0 + math.exp(-z / max(policy.temperature, 1e-6)))


def pick_wanted(rated, state: RunState, policy: InterestPolicy, round_no: int, salt: str):
    """Split scored options into (wanted, passed over), by a sigmoid on
    each one's standing in this run.

    Not a cliff. An option a little above the run's typical interest is
    usually taken and sometimes not; one a little below is usually left
    and sometimes taken. The draw comes from the drift field, so this is
    probabilistic AND fully reproducible -- the same scores in the same
    run always select the same options.

    Both edge cases survive, which is the whole reason interest exists:
    a round entirely below the run's baseline builds nothing, and a round
    entirely above it builds everything (up to the budget).
    """
    if not policy.enabled or not rated:
        return [], list(rated)
    wanted, passed = [], []
    for score, key in rated:
        p = interest_probability(score, state, policy)
        u = drift_interest(str(key), round_no, f"take|{salt}")
        (wanted if u < p else passed).append((score, key))
    return wanted, passed


def read_interest(answers: dict, key: str) -> float | None:
    """Pull one `noul` back out. A `noul` answer carries no separate
    confidence field -- the single probability IS the answer and its
    certainty at once (0.5 means genuinely undecided), which is why
    nothing here looks for one."""
    a = answers.get(key)
    if not isinstance(a, dict):
        return None
    v = a.get("noul")
    return float(v) if isinstance(v, (int, float)) else None


# Measured over 8000 samples of the drift field: median 0.000, sd 0.306,
# symmetric, and close enough to Gaussian that its own normal CDF maps it
# to a near-uniform [0, 1] (p5 -0.504 -> 0.05, p95 +0.501 -> 0.95).
DRIFT_INTEREST_SD = 0.306


def drift_interest(key: str, round_no: int, salt: str) -> float:
    """Dry-run stand-in for an interest score, in [0, 1].

    The naive squash -- (d + 1) / 2 -- is wrong, and wrong in a way that
    silently disables the feature being rehearsed: the drift field is
    zero-mean with sd 0.306, so that maps almost everything into a narrow
    band around 0.5, nothing ever clears a 0.6 bar, and a dry run reports
    a world that never grows. Caught on the first dry run of the fan-out.

    So: push it through its own measured CDF, which spreads it to roughly
    uniform. This is a stand-in for the SHAPE of an interest distribution
    -- a spread of wants, some strong, some cold -- and explicitly not a
    prediction of what Jev will say. Same field as the chooser, so a
    rehearsal stays coherent; its own salt, because wanting a thing and
    picking it are different measurements.
    """
    z = drift(key, round_no, salt) / DRIFT_INTEREST_SD
    return 0.5 * (1.0 + math.erf(z / math.sqrt(2.0)))


def build_state_summary(round_no: int, state: RunState, legal: list[tuple]) -> str:
    """What is TRUE right now, not how the run is going.

    This used to be pure bookkeeping -- "1 committed fact files, 0 prior
    firings, 7 known nodes" -- and that was survivable as long as every
    question was a `choice`, because a choice carries its alternatives in
    its own criteria and can be answered by comparing them to each other.
    An independent yes/no cannot. Asked "do you want to go north?" with
    nothing but a file count for context, Jev returned 0.43-0.50 on every
    option over two live runs: the correct answer to a question with no
    situation in it.

    Confirmed by probing the primitive directly with a real situation --
    "you are extremely thirsty, there is a well to the east, to the north
    is solid rock" -- which returned 0.95 / 0.02 / 0.15. The primitive
    discriminates sharply. It was never given anything to discriminate on.

    So the summary now describes the WORLD: what has been done, what
    stands open and what the world says about it. Still read off
    committed facts, still no invention. Deliberately bounded -- the last
    few firings and the standing edges, not a dump of every fact, since
    the budget is tokens.
    """
    done = [f"{c.id} x{c.firings}" for c in state.candidates.values() if c.firings]
    lines = [f"Round {round_no}."]
    if done:
        lines.append("So far you have: " + ", ".join(sorted(done)[:8]) + ".")
    else:
        lines.append("You have done nothing yet.")

    open_ways = unmapped_frontier(state)
    if open_ways:
        described = []
        for n in open_ways[:8]:
            sentences = state.prose.get(n)
            if sentences:
                # Sentences already end in a period; the list separator
                # must not add a second one.
                described.append(f"{n} -- {' '.join(sentences)}".rstrip("."))
                continue
            said = [f"{p} {v}" for p, v in facts_about(n, state) if p != "cleared"]
            described.append(f"{n} ({'; '.join(said)})" if said else n)
        lines.append(
            f"{len(open_ways)} way(s) stand open with nothing built beyond them. "
            + " ".join(d + "." for d in described)
        )
    else:
        lines.append("No opened way stands unbuilt.")

    built = [Path(f).stem.replace("minted-", "") for f in state.minted_machine_files]
    if built:
        lines.append(f"The world has grown {len(built)} new piece(s) so far: " + ", ".join(built[-8:]) + ".")
    lines.append(f"{len(legal)} action(s) are legal right now.")
    return " ".join(lines)


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


def guarded_nodes(state: RunState) -> set[str]:
    """Every literal node any machine in scope currently guards on.

    A node that something guards on is BUILT-UPON: there is architecture
    downstream of it. One that nothing guards on is an edge of the map.
    """
    anchors: set[str] = set()
    for mf in state.machine_files:
        try:
            machine = parse_machine_text(Path(mf).read_text())
        except OSError:
            continue
        for t in machine["transitions"]:
            for g in t["guards"]:
                parts = g.split()
                if parts and parts[0] == "not":
                    parts = parts[1:]
                if parts:
                    anchors.add(parts[0])
    return anchors


SUBJECT_RE = re.compile(r"^\s{2}([A-Za-z0-9_.\-/]+)\s+`[A-Za-z0-9_]+`", re.M)


def world_entities(state: RunState) -> list[str]:
    """Every node the world says something ABOUT -- i.e. every fact
    subject, in first-seen order.

    "Something with properties" is the least conventional definition of
    an entity available here: it needs no domain vocabulary, and it
    correctly excludes the value-position nodes (`mark/yes`, a state
    ident) that are vocabulary rather than things.
    """
    seen: list[str] = []
    for wf in state.world_files:
        try:
            text = Path(wf).read_text()
        except OSError:
            continue
        for m in SUBJECT_RE.finditer(text):
            if m.group(1) not in seen:
                seen.append(m.group(1))
    return seen


UNIT_EFFECT_RE = re.compile(r"^(assert|retract)\s+(\?\w+|\$\w+)\s+`(\w+)`\s+(\S+)")


def substance_flow(state: RunState) -> tuple[set[str], set[str], set[str]]:
    """Read the substance graph off the machines: (roots, terminals, all).

    A SUBSTANCE is a (predicate, object) pair asserted or retracted about
    a UNIT -- a `?binder` or `$param` subject, never `self` and never a
    literal place. That distinction is the whole trick: `assert self
    `cleared` mark/yes` is a machine saying something about itself, while
    `retract ?unit `is` clay/raw` is matter moving. Only the second is a
    flow, and only flows can cycle.

    A ROOT is consumed by something and produced by nothing -- the world
    starts with a finite stock of it and then it is gone. A TERMINAL is
    produced and consumed by nothing -- matter piles up there. A flow
    graph with roots and terminals is a DAG, and a DAG runs down.

    Which makes closing it mechanical: a `feed` from a terminal back to a
    root turns the DAG into a cycle. Walls crumble to clay. That is not a
    heuristic about what would be nice, it is the one edge that changes
    the graph's verdict.
    """
    consumed: set[str] = set()
    produced: set[str] = set()
    for mf in state.machine_files:
        try:
            text = Path(mf).read_text()
        except OSError:
            continue
        for line in text.splitlines():
            m = UNIT_EFFECT_RE.match(line.strip())
            if not m:
                continue
            kind, _unit, pred, obj = m.groups()
            (consumed if kind == "retract" else produced).add(f"{pred} {obj}")
    return (consumed - produced, produced - consumed, consumed | produced)


def classify_params(t: dict) -> tuple[list[str], dict[str, str], list[str]]:
    """Split a transition's formal params into (guarded, minting, opaque).

    A param is not one kind of thing. `DMML.Ast.Effect`'s haddock is
    explicit that the world is OPEN: an effect may name a node that does
    not exist yet, and asserting about it is what brings it into being.
    So a param that appears in a GUARD is a question about what is
    already there -- which rock -- and a param that appears only as an
    effect SUBJECT is the opposite, a name for something arriving. The
    two look identical in the grammar and are nothing alike to a chooser.

    This distinction is what the loop was missing. It skipped every
    parameterized transition on the honest ground that binding a param is
    a real decision it would not guess -- true of the first kind, and
    exactly wrong about the second. `cannon replenish` emits
    `fall(unit)`, whose `$unit` is guarded by nothing and asserted into
    existence; skipping it meant the rain existed and never fell, and the
    flow graph kept its source on paper while still running down.

    - GUARDED: still skipped here. It is a binding decision over existing
      nodes, and there is already a path for that
      ('DMML.Guard.GuardAmbiguousBinding' -> the round's binding
      question). Routing it through minting would invent a node where the
      world already has candidates, which is the one thing open-world
      minting must not do.
    - MINTING: registered, with the substance ("in quarry/north") it
      arrives as, so a fresh name can be read off the world rather than
      invented.
    - OPAQUE: a param used somewhere this reader cannot account for (an
      effect VALUE, a spawn body). Skipped, and reported as skipped.
      Guessing here would be the same mistake in a new place.
    """
    guarded: list[str] = []
    minting: dict[str, str] = {}
    opaque: list[str] = []
    for param in t["params"]:
        tok = re.compile(r"\$" + re.escape(param) + r"\b")
        if any(tok.search(g) for g in t["guards"]):
            guarded.append(param)
            continue
        substance = None
        for e in t["effects"]:
            m = UNIT_EFFECT_RE.match(e)
            if m and m.group(1) == "assert" and m.group(2) == f"${param}":
                substance = f"{m.group(3)} {m.group(4)}"
                break
        if substance is None:
            opaque.append(param)
        else:
            minting[param] = substance
    return guarded, minting, opaque


def unit_kind(state: RunState, pred: str, obj: str) -> str | None:
    """What KIND of thing already stands in `<x> `pred` obj`, if anything.

    Read off the world, never invented. If `rock/1 `in` quarry/north` and
    `rock/2 `in` quarry/north` are facts, then what falls into
    quarry/north is a rock, and the run says so because the world already
    did. Returns None when nothing stands in that relation yet -- the
    caller then falls back to the machine's OWN word for it (the formal
    param name, `unit`), which is likewise not a guess.
    """
    rx = re.compile(
        r"^\s+([A-Za-z0-9_.\-/]+)\s+`" + re.escape(pred) + r"`\s+" + re.escape(obj) + r"\s*$"
    )
    counts: dict[str, int] = {}
    for wf in state.world_files:
        try:
            text = Path(wf).read_text()
        except OSError:
            continue
        for line in text.splitlines():
            m = rx.match(line)
            if m and "/" in m.group(1):
                kind = m.group(1).split("/", 1)[0]
                counts[kind] = counts.get(kind, 0) + 1
    if not counts:
        return None
    return sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))[0][0]


def refresh_minting_params(state: RunState) -> list[str]:
    """Give every minting param a fresh name, once per round.

    Freshness is decided by what the world already knows
    (`state.known_nodes`), not by a counter this file keeps, because the
    world is the only thing that can say whether a name is taken. A
    second rock must be a second rock: re-asserting `rock/fall0 `in`
    quarry/north` would resurrect the one already spent rather than bring
    another, which is not replenishment, it is an undo.

    Names carry their provenance -- `rock/fall0` is a rock that arrived
    by falling -- and are allocated against a set that also holds the
    names handed out earlier in this same pass, because several groups
    can fire in one round and two sources must not mint the same node.
    """
    taken = set(state.known_nodes)
    touched: list[str] = []
    for c in sorted(state.candidates.values(), key=lambda c: c.id):
        for param, substance in c.minting_params.items():
            pred, _, obj = substance.partition(" ")
            kind = unit_kind(state, pred, obj) or param
            n = 0
            while f"{kind}/{c.transition}{n}" in taken:
                n += 1
            name = f"{kind}/{c.transition}{n}"
            taken.add(name)
            c.params[param] = name
            touched.append(f"{c.id}:${param}={name}")
    return touched


def flow_digraph(state: RunState) -> tuple[set[str], set[tuple[str, str]]]:
    """The substance-flow DIGRAPH: (substances, directed edges).

    An edge c -> p means some transition consumes c and produces p in the
    same firing, so matter moves that way and only that way. Same reading
    as `DMML.CheckFertility.flowGraph`, kept in step with it deliberately
    (see that module's note on the coupling).
    """
    nodes: set[str] = set()
    edges: set[tuple[str, str]] = set()
    for mf in state.machine_files:
        try:
            machine = parse_machine_text(Path(mf).read_text())
        except OSError:
            continue
        for t in machine["transitions"]:
            cons, prod = set(), set()
            for e in t["effects"]:
                m = UNIT_EFFECT_RE.match(e)
                if not m:
                    continue
                kind, _u, pred, obj = m.groups()
                (cons if kind == "retract" else prod).add(f"{pred} {obj}")
            nodes |= cons | prod
            edges |= {(c, pr) for c in cons for pr in prod}
    return nodes, edges


def _reach(edges: set[tuple[str, str]], start: str) -> set[str]:
    seen, stack = set(), [start]
    while stack:
        x = stack.pop()
        for a, b in edges:
            if a == x and b not in seen:
                seen.add(b)
                stack.append(b)
    return seen


def cycle_rank(nodes: set[str], edges: set[tuple[str, str]]) -> int:
    """Total independent circuits: sum of E - V + 1 over the non-trivial
    strongly connected components.

    This is the magnitude behind `check-fertility`'s CIRCULATES verdict --
    how many flow edges you would have to cut before matter stops going
    round. Deliberately NOT the first Betti number of the underlying
    undirected graph: a diamond (clay->brick, clay->tile, brick->wall,
    tile->wall) has b1 = 1 and still runs down, because matter only goes
    one way round it. Direction is not topological data and direction is
    what decides it. See examples/rhizome-demo/diamond.dmml.
    """
    reach = {n: _reach(edges, n) for n in nodes}
    comps: list[frozenset[str]] = []
    for a in nodes:
        if a not in reach[a]:
            continue
        comp = frozenset(b for b in nodes if b in reach[a] and a in reach[b])
        if comp not in comps:
            comps.append(comp)
    total = 0
    for c in comps:
        inner = [(a, b) for a, b in edges if a in c and b in c]
        total += len(inner) - len(c) + 1
    return total


def rank_delta(nodes: set[str], edges: set[tuple[str, str]], new: tuple[str, str]) -> int:
    """How many independent circuits adding this one edge would create.

    Computed, not judged. Whether an edge closes a circuit is an exact
    question about a digraph with an exact answer, and no amount of good
    taste substitutes for running it: 0 means the edge merely adds
    another one-way path, and the world still runs down with it.
    """
    if new in edges:
        return 0
    return cycle_rank(nodes | {new[0], new[1]}, edges | {new}) - cycle_rank(nodes, edges)


# Objects that are bookkeeping rather than things: the reachability
# convention's value, and anything standing as a machine's lifecycle
# state. Reading either as matter would be reading the plumbing.
NOT_MATTER_PREDS = {"state", "cleared"}


def latent_objects(state: RunState) -> list[tuple[str, str, str]]:
    """Things the world NAMES but never moves: (predicate, object, who says so).

    This is the gap every connective operator is blind to. `bridge`,
    `feed`, `replenish` and `vista` all read the flow graph, so in a
    world whose flow graph is EMPTY they have nothing to work on and
    nothing they can do about it. Measured: cannon-grow ran 40 rounds and
    49 machines with zero substance flows, and check-fertility's whole
    rhizome half was structurally silent -- not because the world was
    poor but because no operator could reach across into it.

    And the world was not poor. cannon-fanout's seed names draft/cold,
    water/running, rubble/fallen, light/daylight, three kinds of sound --
    seven things, all of them sitting in the FACT graph as static
    attributes of places, none of them matter. Nothing in the operator
    set can promote one. That is the boundary of the territory, and it is
    in the proposer's field of view rather than in the operators
    themselves: `cannon feed` builds the machine fine once someone thinks
    to ask for it.

    So this reads objects back out of committed facts. Nothing invented
    -- every one is a fact somebody wrote down. What is new is treating
    it as something that can move.
    """
    substances, _edges = flow_digraph(state)
    known = {x.split(" ", 1)[1] for x in substances}
    out: dict[tuple[str, str], str] = {}
    for wf in state.world_files:
        try:
            text = Path(wf).read_text()
        except OSError:
            continue
        for line in text.splitlines():
            m = FACT_RE.match(line)
            if not m:
                continue
            subj, pred, obj = m.group(1), m.group(2), m.group(3)
            if pred in NOT_MATTER_PREDS or "/" not in obj or obj in known:
                continue
            out.setdefault((pred, obj), subj)
    return sorted((pred, obj, subj) for (pred, obj), subj in out.items())


def guarded_substances(state: RunState) -> set[tuple[str, str]]:
    """Every (predicate, object) any machine currently GUARDS on, over a
    unit or a literal. What the machine layer can currently read."""
    out: set[tuple[str, str]] = set()
    for mf in state.machine_files:
        try:
            machine = parse_machine_text(Path(mf).read_text())
        except OSError:
            continue
        for t in machine["transitions"]:
            for g in t["guards"]:
                m = re.match(r"^(?:not\s+)?\S+\s+`([A-Za-z0-9_]+)`\s+(\S+)", g)
                if m:
                    out.add((m.group(1), m.group(2)))
    return out


def machine_nodes(state: RunState) -> set[str]:
    """Nodes that ARE machines -- anything the world gives a `state`.
    A machine is not a property of anything, so it has no business on
    the right-hand side of an implication; a `vista`'s `overlooks
    watch/keeper` fact would otherwise offer "anything cold is thereby
    also the keeper", which is not a proposition about the world."""
    out: set[str] = set()
    for wf in state.world_files:
        try:
            text = Path(wf).read_text()
        except OSError:
            continue
        for line in text.splitlines():
            m = FACT_RE.match(line)
            if m and m.group(2) == "state":
                out.add(m.group(1))
    return out


def unread_facts(state: RunState) -> list[tuple[str, str, str]]:
    """Facts the world states that NOTHING can currently read: their
    predicate appears in no guard anywhere in the machine layer.

    One level down from `latent_objects`, and the same kind of gap.
    That one found things the world names but never MOVES; this finds
    things the world says that nothing ever ASKS about. A `yields
    sound/echo` fact nobody guards on is inert -- committed, true, and
    causally invisible.
    """
    read_preds = {p for p, _o in guarded_substances(state)}
    machines = machine_nodes(state)
    out: dict[tuple[str, str], str] = {}
    for wf in state.world_files:
        try:
            text = Path(wf).read_text()
        except OSError:
            continue
        for line in text.splitlines():
            m = FACT_RE.match(line)
            if not m:
                continue
            subj, pred, obj = m.group(1), m.group(2), m.group(3)
            if pred in NOT_MATTER_PREDS or pred in read_preds or "/" not in obj:
                continue
            if obj in machines:
                continue
            out.setdefault((pred, obj), subj)
    return sorted((pred, obj, subj) for (pred, obj), subj in out.items())


def anchorable_nodes(state: RunState) -> set[str]:
    """Nodes something in the world can ever assert `cleared` on.

    The cannon anchors every room it fires with
    `guard <parent> `cleared` mark/yes`, so a node nothing ever clears is
    a node nothing can ever be built beyond. A machine that clears itself
    makes its own node anchorable; a fork makes the path nodes it opens
    anchorable; a machine that clears neither makes nothing anchorable,
    and anything anchored on it is unreachable architecture.

    Checking this is the LOOP doing its own job, not the chooser doing
    it. Jev is asked where it wants to go; whether a place can ever be
    reached is a structural fact about the machines, and making the
    chooser responsible for it was the mistake.
    """
    ok: set[str] = set()
    for mf in state.machine_files:
        try:
            machine = parse_machine_text(Path(mf).read_text())
        except OSError:
            continue
        for t in machine["transitions"]:
            for e in t["effects"]:
                parts = e.split()
                if len(parts) >= 4 and parts[0] == "assert" and parts[2] == "`cleared`":
                    ok.add(machine["node"] if parts[1] == "self" else parts[1])
    return ok


def growable_leaves(state: RunState) -> list[str]:
    """Every entity with no architecture built on it -- the places the
    WORLD can grow, as distinct from where the DELVE can currently walk.

    Those two were the same thing until 2026-09-18 and should not have
    been. Keying growth to `cleared` nodes meant the world could only
    take shape where the delve had already been, which quietly made the
    chooser responsible for keeping the generator alive: a run's length
    depended on whether Jev happened to pick transitions that opened new
    ground. That is a meta-burden a chooser should never carry. Jev's job
    is to want things, not to feed the cannon.

    So: any leaf is a valid place to build. Whether the delve can reach
    it yet is a separate question, and the delve's own frontier
    ('unmapped_frontier') stays the thing Jev is actually asked about.
    """
    built_on = guarded_nodes(state)
    reachable = anchorable_nodes(state)
    return [n for n in world_entities(state) if n not in built_on and n in reachable]


def unmapped_frontier(state: RunState) -> list[str]:
    """The live edge of the map: nodes the world records as cleared that
    NOTHING is built on yet.

    This is the demand signal, and it is the whole difference between a
    world generated on a clock and one generated the way written-world
    generates: there, arriving at an unmapped frontier point is what
    causes generation, and once a place is made it is never remade. Here
    the same thing, one articulation down -- written-world generates a
    room's DESCRIPTION on arrival, this generates a room's MECHANISM.

    A node the delve never opens is never built, which is the point:
    the only genuinely scarce resource is attention, and building where
    nobody went spends it on nothing.
    """
    built_on = guarded_nodes(state)
    return [n for n in frontier_nodes(state) if n not in built_on]


FACT_RE = re.compile(r"^\s+([A-Za-z0-9_.\-/]+)\s+`([A-Za-z0-9_]+)`\s+(.+?)\s*$")


def facts_about(node: str, state: RunState) -> list[tuple[str, str]]:
    """Every committed fact whose SUBJECT is this node, in commit order,
    later values winning. Read back, never invented."""
    seen: dict[str, str] = {}
    for wf in state.world_files:
        try:
            text = Path(wf).read_text()
        except OSError:
            continue
        for line in text.splitlines():
            m = FACT_RE.match(line)
            if m and m.group(1) == node:
                seen[m.group(2)] = m.group(3)
    return sorted(seen.items())


def describe_frontier_node(node: str, state: RunState) -> str:
    """What can be said about an edge of the map WITHOUT inventing it.

    Nothing has been built past this node, so there is nothing there to
    describe -- and making something up would be putting words in the
    mouth of architecture that does not exist yet. What is true and
    useful to a chooser: how this way was opened, and that it is unbuilt.
    """
    opener = None
    for wf in reversed(state.world_files):
        try:
            text = Path(wf).read_text()
        except OSError:
            continue
        if any(CLEARED_RE.match(ln) and CLEARED_RE.match(ln).group(1) == node for ln in text.splitlines()):
            opener = Path(wf).stem
            break
    how = f" (opened by {opener})" if opener else ""
    # Everything the world already SAYS about this node, `cleared` aside
    # (which is why it is on the list in the first place, so repeating it
    # tells a chooser nothing). Still no invention -- these are committed
    # facts, read back. Without them every unbuilt way reads identically
    # and a chooser asked to rank them is being asked to guess: the first
    # live interest run returned 0.43/0.44/0.46/0.44 over four ways whose
    # descriptions differed only in the name, which is the right answer
    # to a question carrying no information.
    # Prose first, raw facts only as a fallback -- see refresh_prose.
    sentences = state.prose.get(node)
    if sentences:
        known = " " + " ".join(sentences)
    else:
        said = [f"{pred} {val}" for pred, val in facts_about(node, state) if pred != "cleared"]
        known = f" The world says of it: {'; '.join(said)}." if said else ""
    return f"Press on past {node}{how}.{known} Nothing has been built beyond it yet."


def prose_binary() -> list[str]:
    import shlex

    return shlex.split(os.environ.get("RENDER_PROSE", "render-prose"))


def refresh_prose(state: RunState) -> int:
    """Render the whole world to sentences, once, for this round.

    Jev has been reading raw triples all along -- `describe_frontier_node`
    concatenated `pred obj` pairs and they happened to read as English
    because the demo vocabulary happened to be English verbs in the right
    form. `upstreamOf weir/low` is where that stops. This replaces the
    concatenation with the real closed-set selection path: same catalog,
    same `eligibleTemplates`, same `renderTemplateWith` that
    `check-prose-coverage` measures, so what Jev reads is exactly what
    that tool certifies.

    Fails SOFT and says so. A missing binary or an unparseable catalog
    leaves `state.prose` empty and every describer falls back to the
    triple -- a run that loses its prose should read worse, not stop.
    """
    state.prose = {}
    if not state.prose_catalogs:
        return 0
    cmd = prose_binary() + ["--json"]
    for c in state.prose_catalogs:
        cmd += ["--catalog", c]
    cmd += list(state.world_files)
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
    except (FileNotFoundError, subprocess.TimeoutExpired) as e:
        print(f"  prose: renderer unavailable ({e.__class__.__name__}); descriptions fall back to raw facts")
        return 0
    if proc.returncode != 0:
        print(f"  prose: renderer failed: {proc.stderr.strip()[:200]}; falling back to raw facts")
        return 0
    try:
        state.prose = {k: list(v) for k, v in json.loads(proc.stdout).items()}
    except json.JSONDecodeError:
        print("  prose: renderer output was not JSON; falling back to raw facts")
        return 0
    return len(state.prose)


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


def plan_extension(state: RunState, extend: ExtendPolicy, anchor: str, seq: int) -> tuple[str, list[str], str]:
    """Decide the next shot, deterministically, at an anchor CHOSEN
    ELSEWHERE. Returns (kind, cannon args, human provenance).

    The anchor used to be picked here, by cycling the frontier. That was
    the single most consequential knob in this file -- it decided the
    shape of the world -- and it was a modulo. It belongs to whoever is
    making decisions, which in this loop is Jev.

    STAMP while there is not yet enough material to cross, then BREED --
    and breed with the most recently MINTED machine as parent A, so each
    generation is crossed with the one before it rather than endlessly
    re-crossing the seed pair. Parent B cycles through everything else
    in scope, which keeps the lineage deep without making it narrow.
    """
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


def bridged_pairs(state: RunState) -> set[frozenset[str]]:
    """Pairs of nodes some machine already guards on BOTH of -- i.e.
    already related. Proposing a second corridor between the same two
    rooms is densification with no new information in it."""
    pairs: set[frozenset[str]] = set()
    for mf in state.machine_files:
        try:
            machine = parse_machine_text(Path(mf).read_text())
        except OSError:
            continue
        anchors = {
            g.split()[1] if g.split()[0] == "not" else g.split()[0]
            for t in machine["transitions"]
            for g in t["guards"]
            if g.split()
        }
        for a in anchors:
            for b in anchors:
                if a != b:
                    pairs.add(frozenset((a, b)))
        # A bridge's two ends are in DIFFERENT transitions, so collect
        # across the machine as a whole too.
        ends = {
            g.split()[0]
            for t in machine["transitions"]
            for g in t["guards"]
            if g.split() and g.split()[0] != "not"
        }
        for a in ends:
            for b in ends:
                if a != b:
                    pairs.add(frozenset((a, b)))
    return pairs


def connective_proposals(
    state: RunState, seq: int, cap: int = 2, regards: list[str] = (), bridge_cost: str = ""
) -> list[tuple[str, str, list[str]]]:
    """What relations could exist that do not yet, as (id, description,
    cannon args).

    This is the rhizome half of growth, and it asks a different question
    from the arborescent half. Stamping and breeding ask "where do I
    attach" -- a question about leaves, with as many answers as there are
    leaves. These ask "which two things that already exist should now
    relate" -- a question with N-squared answers, which is why the world
    densifies rather than exhausting, and equally why the CHOOSING
    matters more here than it did there.

    Deliberately proposes a handful rather than enumerating the whole
    N-squared space: the budget is attention, and handing a chooser two
    hundred indistinguishable corridors would spend it on nothing.
    """
    cleared = [n for n in frontier_nodes(state)]
    already = bridged_pairs(state)
    roots, terminals, _all_subs = substance_flow(state)
    out: list[tuple[str, str, list[str]]] = []

    # A bridge between two cleared places not already related: the one
    # move that puts a CYCLE in reachability, which no amount of stamping
    # or breeding can produce.
    unrelated = [
        (a, b)
        for i, a in enumerate(cleared)
        for b in cleared[i + 1 :]
        if frozenset((a, b)) not in already
    ]
    for a, b in unrelated[:cap]:
        node = f"corridor/c{seq}_{len(out)}"
        out.append(
            (
                f"bridge-{a}-{b}",
                f"Cut a corridor between {a} and {b}. Both already stand; this makes a second "
                f"way between them, so neither is reachable only one way any more.",
                ["bridge", node, a, b] + (bridge_cost.split() if bridge_cost else []),
            )
        )

    # Close a flow circuit. This used to propose exactly one feed --
    # sorted(terminals)[0] back to sorted(roots)[0] -- on the reasonable
    # but unchecked assumption that terminal-to-root is where the loop
    # wants closing. Two things wrong with that. It misses every feed
    # that would close a circuit somewhere in the middle of the graph,
    # and it cannot tell a feed that actually closes one from a feed that
    # just adds another one-way path.
    #
    # Whether an edge closes a circuit is an exact question about a
    # digraph. So it is COMPUTED, over every pair of substances sharing a
    # predicate (the only pairs `cannon feed` can actually connect), and
    # the ones that raise the rank are offered first.
    #
    # Note what is and is not delegated. The rank delta is not a matter of
    # taste and no chooser is asked to intuit it -- an LLM cannot know
    # whether adding an edge merges two strongly connected components,
    # and would guess fluently. What IS delegated is whether closing that
    # particular circuit is worth having, which is a question about the
    # world and not about the graph.
    fnodes, fedges = flow_digraph(state)
    feeds: list[tuple[int, str, str, str, str]] = []
    for a in sorted(fnodes):
        ap, ao = a.split(" ", 1)
        for b in sorted(fnodes):
            if a == b or (a, b) in fedges:
                continue
            bp, bo = b.split(" ", 1)
            if ap != bp:  # `cannon feed` moves a unit BETWEEN objects of one predicate
                continue
            feeds.append((rank_delta(fnodes, fedges, (a, b)), ap, ao, bo, b))
    feeds.sort(key=lambda f: (-f[0], f[2], f[3]))
    have_rank = cycle_rank(fnodes, fedges)
    for i, (delta, pred, frm, to, _b) in enumerate(feeds[:cap]):
        node = f"decay/d{seq}_{i}"
        if delta > 0:
            why = (
                f"This CLOSES A CIRCUIT: matter already travels {to} -> ... -> {frm} and stops there; "
                f"this sends it back, so the world would have {have_rank + delta} independent "
                f"circuit(s) where it now has {have_rank}. A world whose matter circulates does not "
                f"run out."
            )
        else:
            why = (
                f"This closes no circuit -- nothing currently leads from {to} back to {frm}, so "
                f"matter would still only go one way and the world would still run down. It would "
                f"be a new path, not a loop."
            )
        out.append((f"feed-{frm}-to-{to}", f"Let {frm} become {to}. {why}", ["feed", node, pred, frm, to]))

    # A source for something consumed and never made. Needs no cleared
    # node to anchor on -- a source is unconditioned by definition, which
    # also means this works in a world with no reachability convention at
    # all (a pure substance world has no `cleared` facts anywhere).
    for r in sorted(roots)[:1]:
        rp, ro = r.split(" ", 1)
        node = f"weather/w{seq}"
        out.append(
            (
                f"replenish-{ro}",
                f"Have more arrive from outside -- rain, silt, drift -- so that {rp} {ro} keeps "
                f"being true of new things. It is consumed here and made nowhere, so as the world "
                f"stands it can only ever run out.",
                ["replenish", node, rp, ro],
            )
        )

    # DETERRITORIALIZE. The one move that is not closed over what
    # already exists.
    #
    # Everything above redistributes: bridge, feed, replenish and vista
    # all read the flow graph and act on substances already in it. That
    # is territorializing by construction, and it has a measured
    # consequence -- DMML.Recombine's crossover never invents a
    # transition either, so the structure space is finite and
    # check-fertility's lineage walk is provably eventually periodic. A
    # real run bore that out: 11 bred rooms, 3 distinct shapes, fixpoint
    # at generation 1. No sequence of these operators leaves the space
    # they generate.
    #
    # The exit has to come from outside the operator set, and here it is
    # bounded but real: read matter out of the FACT graph, which no
    # operator looks at. The world already names draft/cold and
    # water/running and rubble/fallen; nothing treats any of them as
    # something that moves. Promoting one is a move no combination of the
    # four could ever make.
    #
    # And it is caught immediately on the way back down. Whatever this
    # opens gets measured by the same machinery as everything else: the
    # proposal is scored by interest like any other, and the moment the
    # flow graph is non-empty the rank-scored feeds above take over and
    # close circuits through it. Deterritorialize, then reterritorialize
    # -- neither half works alone, since pure enumeration is provably
    # periodic and pure generation is unverifiable.
    #
    # BOUNDED, and worth saying exactly how: this escapes the flow
    # graph's territory by reading the fact graph, not the world's
    # vocabulary as a whole. It can promote a thing the world already
    # names; it cannot name a new one. Unbounded would need a GENERATIVE
    # model, and this stack has a discriminator -- Jev's three primitives
    # (choice, score, noul) all select, none produce text. That is a real
    # limit of the architecture, not an oversight.
    if len(fnodes) < 2:
        latent = latent_objects(state)
        by_pred: dict[str, list[tuple[str, str]]] = {}
        for pred, obj, subj in latent:
            by_pred.setdefault(pred, []).append((obj, subj))
        pairs = [
            (pred, a, asaid, b, bsaid)
            for pred, objs in sorted(by_pred.items())
            for (a, asaid) in objs
            for (b, bsaid) in objs
            if a != b
        ]
        # Rotate which pairs get offered across rounds rather than always
        # showing the alphabetical first: with no flow graph there is no
        # rank to sort by yet, and a fixed order would mean this world
        # only ever gets asked one question.
        pairs.sort(key=lambda t: (drift(f"{t[0]}|{t[1]}|{t[3]}", seq, "transmute"), t[1], t[3]), reverse=True)
        for i, (pred, a, asaid, b, bsaid) in enumerate(pairs[:cap]):
            node = f"works/t{seq}_{i}"
            out.append(
                (
                    f"transmute-{a}-to-{b}",
                    f"Nothing in this world is made of anything yet -- it is all places and ways, "
                    f"and no matter moves through it at all. But the world does already say that "
                    f"{asaid} {pred} {a}, and that {bsaid} {pred} {b}. This would make {a} a thing "
                    f"that can BECOME {b}: the first matter here, and the first transformation. "
                    f"Nothing that exists can be rearranged into this -- it has to be introduced.",
                    ["feed", node, pred, a, b],
                )
            )
        for pred, obj, subj in latent[:1]:
            node = f"spring/s{seq}"
            out.append(
                (
                    f"wellspring-{obj}",
                    f"The world says {subj} {pred} {obj}, and treats it as a fixed attribute of a "
                    f"place. This would make {obj} something that ARRIVES instead -- new ones of "
                    f"it, from outside, unconditioned. A world with a source has somewhere for "
                    f"matter to come from.",
                    ["replenish", node, pred, obj],
                )
            )

    # OPEN THE TRANSITION-SHAPE VOCABULARY. One level below the
    # substance generator above, and aimed at a different fixpoint.
    #
    # That one opens what the world is MADE of. This opens what a machine
    # can BE. DMML.Recombine's crossover recombines the guards and
    # effects its parents already carry and never invents one, which is
    # exactly why check-fertility's lineage walk is provably eventually
    # periodic -- 11 bred rooms, 3 distinct shapes, fixpoint at
    # generation 1 in a real run. A new shape cannot come from
    # recombining old shapes, so it has to be introduced.
    #
    # `cannon imply` is that new shape: guard one predicate, assert a
    # DIFFERENT one about the same unit, consume nothing. No operator
    # here could previously express it -- feed crosses objects within one
    # predicate, vista relates places and touches no unit.
    #
    # Measured, on kiln + mason as the base pool (walk length before a
    # shape repeats, which is what periodicity means here):
    #
    #     2 seeds, baseline                    2 generations
    #     4 seeds, two DUPLICATE shapes        2 generations   <- no gain
    #     4 seeds, rain + imply               14 generations
    #
    # The duplicate control matters: adding seeds does not lengthen the
    # walk, adding SHAPES does. Seven times the reachable structure
    # space, from one new atom in the pool.
    unread = unread_facts(state)
    # Excluding the plumbing from the GUARD side too. `cleared mark/yes`
    # is guarded by every room the cannon ever fired, so it would
    # dominate this list -- and "anything that is cleared is thereby also
    # X" is a statement about reachability bookkeeping, not about matter.
    readable = sorted(
        {(p_, o_) for p_, o_ in guarded_substances(state) if p_ not in NOT_MATTER_PREDS}
        | {(p_, o_) for p_, o_, _s in latent_objects(state)}
    )
    if unread and readable:
        couplings = [
            (fp, fo, tp, to, said)
            for (fp, fo) in readable
            for (tp, to, said) in unread
            if fp != tp
        ]
        couplings.sort(key=lambda c: (drift(f"{c[0]}|{c[1]}|{c[2]}|{c[3]}", seq, "imply"), c[1], c[3]), reverse=True)
        for i, (fp, fo, tp, to, said) in enumerate(couplings[:cap]):
            node = f"works/i{seq}_{i}"
            out.append(
                (
                    f"imply-{fo}-{to}",
                    f"Right now `{tp}` is a dead word here: {said} {tp} {to} is true and nothing "
                    f"in this world can act on it, because no machine asks about `{tp}` at all. "
                    f"This would make being {fp} {fo} enough to also count as {tp} {to} -- which "
                    f"brings `{tp}` alive, so that everything the world has ever said with that "
                    f"word starts to matter and can be built on. Nothing is consumed or moved; "
                    f"what changes is what the world is able to notice about itself.",
                    ["imply", node, fp, fo, tp, to],
                )
            )

    # REGARD: two things that already exist come to stand in a relation
    # that is not spatial and moves nothing.
    #
    # The axis the world was missing. `bridge` relates PLACES and
    # `feed` moves MATTER; nothing related agents, so nothing could,
    # and the measurement said so -- predicate evenness 0.21, every
    # predicate acyclic, no reciprocity anywhere in a 134-machine world.
    #
    # Paired on a shared WITNESS predicate: both ends must already carry
    # the same kind of property, which is a real condition read off the
    # world and is what keeps this from becoming a second N-squared
    # flood. Two things that both have a `role` may come to admire each
    # other; a corridor and a role may not.
    if regards:
        by_witness: dict[str, list[str]] = {}
        for subj in world_entities(state):
            for pred, _obj in facts_about(subj, state):
                if pred in NOT_MATTER_PREDS or pred in ("name", "epithet", "description"):
                    continue
                by_witness.setdefault(pred, []).append(subj)
        pairs = [
            (w, a, b)
            for w, subjects in sorted(by_witness.items())
            for i, a in enumerate(sorted(set(subjects)))
            for b in sorted(set(subjects))[i + 1 :]
        ]
        pairs.sort(key=lambda t: (drift(f"{t[0]}|{t[1]}|{t[2]}", seq, "regard"), t[1], t[2]), reverse=True)
        for i, (witness, a, b) in enumerate(pairs[:cap]):
            rel = regards[(seq + i) % len(regards)]
            node = f"bond/b{seq}_{i}"
            out.append(
                (
                    f"regard-{a}-{rel}-{b}",
                    f"{a} and {b} have nothing to do with each other, except that the world says "
                    f"each of them has a `{witness}`. This would have {a} come to {rel} {b} -- "
                    f"nothing moved, nowhere new to go, just two things that now stand in a "
                    f"relation they did not. It can lapse again.",
                    ["regard", node, a, rel, b, witness],
                )
            )

    # A vista: relates without moving anything.
    if len(cleared) >= 2:
        node = f"tower/t{seq}"
        out.append(
            (
                f"vista-{cleared[-1]}",
                f"Raise something at {cleared[0]} that looks out over {cleared[-1]}. Nothing "
                f"passes between them; they simply become visible to each other.",
                ["vista", node, cleared[0], cleared[-1]],
            )
        )
    return out


def extend_world(
    state: RunState,
    extend: ExtendPolicy,
    world_dir: Path,
    round_no: int,
    anchor: str,
    bidden: bool,
) -> dict | None:
    """Fire the cannon once, at a named anchor, and fold what it mints
    into the run: machine file, seeded initial state, new candidates.

    Everything it adds is ORDINARY -- an ordinary Surface machine file,
    an ordinary world commit seeding its state, ordinary candidates
    through the same dry_fire/dedup/grouping path as the hand-authored
    ones. Nothing downstream knows or cares that a machine was minted
    mid-run rather than written into the config, which is the point.

    `bidden` records WHOSE desire caused this: the delve's (Jev pressed
    into that edge) or the world's own (the unbidden allocation). It
    changes nothing mechanically and is logged, because the difference
    between a world that answers you and one that also wants things is
    worth being able to read back out of an audit log.
    """
    if not extend.enabled:
        return None
    if state.minted_machines >= extend.max_minted_machines:
        print(f"  extend: at max_minted_machines={extend.max_minted_machines}, no more growth")
        return None

    seq = state.minted_machines
    kind, args, provenance = plan_extension(state, extend, anchor, seq)
    return mint(state, world_dir, round_no, kind, args, provenance, anchor, bidden)


def mint(
    state: RunState,
    world_dir: Path,
    round_no: int,
    kind: str,
    args: list[str],
    provenance: str,
    anchor: str,
    bidden: bool,
) -> dict | None:
    """Fire the cannon once and fold what it mints into the run.

    Shared by both halves of growth on purpose. An arborescent shot and a
    connective one differ entirely in WHAT they build and not at all in
    what happens next: a machine file, a seeded initial state, ordinary
    candidates through the same dry_fire/grouping path. Nothing
    downstream knows or cares which kind it was, which is the same
    property that let minted machines be ordinary in the first place.
    """
    out = run_cannon(args)
    if out is None:
        return None

    machine = parse_machine_text(out)
    if not machine["node"] or not machine["states"]:
        print(f"  extend: cannon output for {args} had no node/states -- refusing to register it")
        return None

    machine_file = world_dir / f"minted-{sanitize_node(machine['node'])}.dmml"
    machine_file.write_text(out)
    state.machine_files.append(str(machine_file))
    state.minted_machine_files.append(str(machine_file))
    state.minted_machines += 1

    # A machine's current state is mutable world data, not structural
    # definition -- app/Cannon.hs deliberately does not emit it, so the
    # caller seeds it. Its FIRST declared state is its initial one, the
    # same lifecycle-order convention DMML.Recombine's state alignment
    # already relies on.
    seed = world_dir / f"{round_no:03d}-extend-{sanitize_node(machine['node'])}.dmml"
    seed.write_text(f"commit extends\n  {machine['node']} `state` {machine['states'][0]}\n")
    state.world_files.append(str(seed))
    # Count cannon-minted nodes against the SAME max_minted_nodes cap
    # firings are counted against. Growth is the dominant source of new
    # world once `extend` is on, so a node budget that quietly stopped
    # covering it would be a cap that reads as a bound and is not one.
    fresh = set(NODE_TOKEN_RE.findall(out)) - state.known_nodes
    state.known_nodes |= fresh
    state.minted_nodes += len(fresh)

    registered, skipped, minted_params, vestigial = [], [], [], []
    for t in machine["transitions"]:
        guarded, minting, opaque = classify_params(t)
        # A param the engine could resolve from the world is a decision,
        # and this file does not make decisions about which thing to act
        # on -- that is the binding question's job. A param nothing can
        # account for is skipped for the same reason. A MINTING param is
        # neither: nothing exists to choose among, so naming it is not a
        # choice being taken away from anyone.
        if guarded:
            skipped.append(f"{t['ident']}({', '.join(guarded)})")
            continue
        if opaque:
            # Not the same complaint. These params are declared and then
            # used in no way this reader can account for -- breeding
            # produces them routinely, since a mode can take one parent's
            # signature and the other's effects. Reported separately so
            # "needs a binding" does not get said about a param nothing
            # needs.
            vestigial.append(f"{t['ident']}({', '.join(opaque)})")
            continue
        cid = f"{sanitize_node(machine['node'])}-{t['ident']}"
        if cid in state.candidates:
            continue
        description = describe_minted(machine, t, provenance)
        if minting:
            description += (
                " Something new arrives each time this fires ("
                + "; ".join(f"a fresh {obj} by `{pred}`" for pred, _, obj in
                            (m.partition(" ") for m in minting.values()))
                + "), so firing it again is not a repeat."
            )
        state.candidates[cid] = Candidate(
            id=cid,
            machine=str(machine_file),
            transition=t["ident"],
            verb="breaches",
            params={},
            description=description,
            minting_params=minting,
        )
        registered.append(cid)
        if minting:
            minted_params.append(f"{cid}({', '.join(sorted(minting))})")

    whose = "pressed into" if bidden else "UNBIDDEN at"
    print(f"  extend: {whose} {anchor} -> {machine['node']} ({kind}; {provenance})")
    print(f"          registered {len(registered)} candidate(s): {registered}")
    if minted_params:
        print(f"          {len(minted_params)} transition(s) MINT a new thing each firing: {minted_params}")
    if skipped:
        print(f"          skipped {len(skipped)} transition(s) whose param is a binding decision: {skipped}")
    if vestigial:
        print(f"          skipped {len(vestigial)} transition(s) with a param used nowhere: {vestigial}")
    return {
        "kind": kind,
        "bidden": bidden,
        "anchor": anchor,
        "node": machine["node"],
        "provenance": provenance,
        "cannon_args": args,
        "machine_file": str(machine_file),
        "registered_candidates": registered,
        "minting_candidates": minted_params,
        "skipped_parameterized": skipped,
        "skipped_vestigial": vestigial,
    }


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
    for k in candidate.bound_params:
        candidate.params.pop(k, None)
    candidate.bound_params.clear()
    return len(new_nodes)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("config", type=Path)
    ap.add_argument("--world-dir", type=Path, default=None, help="scratch dir for accumulating firings (default: mkdtemp)")
    ap.add_argument("--audit-log", type=Path, default=None)
    ap.add_argument(
        "--dry-run",
        action="store_true",
        help="skip the Jev call; choose by a deterministic Perlin drift field instead "
        "(reproducible, order-independent, and coherent across rounds -- see 'The dry-run chooser')",
    )
    ap.add_argument("--api-key", default=os.environ.get("TYPESAFE_API_KEY"))
    args = ap.parse_args()

    state, budget, extend, interest, jev_cfg = load_config(args.config)

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

        # Every source names what it brings BEFORE anything is dry-fired,
        # because a minting param is an argument to the firing, not a
        # result of it -- the scan has to see the name the fire would use.
        n_prose = refresh_prose(state)
        if n_prose:
            print(f"  prose: {n_prose} node(s) have sentences this round")

        minted_names = refresh_minting_params(state)
        if minted_names:
            print(f"  arriving this round: {minted_names}")

        scanned, pending = scan_candidates(state)
        legal = [(c, out) for c, out in scanned if c.firings < budget.max_firings_per_candidate]
        pending = [p for p in pending if p[0].firings < budget.max_firings_per_candidate]
        # An unmapped edge is DEMAND, not exhaustion. The old loop
        # stopped the moment nothing was legal; that treated the frontier
        # running dry as the end of the run, when it is precisely the
        # signal to build. The run is only really over when there is
        # nothing to do AND nowhere left that anyone opened and never
        # entered.
        unmapped = unmapped_frontier(state) if extend.enabled else []
        leaves = growable_leaves(state) if extend.enabled else []
        # A pending binding is a QUESTION, not an absence of work. Left
        # out of this condition the loop stops with a decision sitting
        # unasked on the table -- which is exactly the bug a first
        # dry run of this scenario showed.
        if not legal and not unmapped and not pending and not leaves:
            if extend.enabled:
                print(
                    f"=== round {round_no}: fixpoint -- nothing legal, and no unmapped edge left to "
                    f"press into ({state.minted_machines} machine(s) minted). Stopping cleanly ==="
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

        # Only ASK when there is a real decision. One unmapped edge is
        # not a choice, it is the only way on -- spending a question on
        # it would burn the scarce thing to be told what we already know.
        # A binder the engine refused to resolve is a decision waiting to
        # be made, so it rides in this same batched call alongside the
        # action choices. The answer is applied as a --param, which
        # pre-binds the binder, and the candidate becomes legal in the
        # NEXT round's pass -- a one-round lag, taken deliberately over
        # resolving mid-round, since a candidate's legality depends on
        # the binding and it cannot be in this round's action groups
        # until it has one.
        binding_q = [
            (f"bind_{c.id}", var, f"{c.description} Which one do you take?", opts)
            for c, var, opts in pending
        ]
        if binding_q:
            state_summary += f" {len(binding_q)} choice(s) of WHICH thing to act on are open."

        # The rhizome question, asked alongside everything else: which
        # two things that already exist should now relate? Only asked
        # when growth is on and there is more than one answer -- a lone
        # proposal is not a choice, and spending a question on it would
        # burn the scarce thing to be told what we already know.
        connect_q = (
            connective_proposals(
                state, state.minted_machines, extend.proposals_per_kind,
                extend.regards, extend.bridge_cost,
            )
            if extend.enabled
            else []
        )
        if len(connect_q) > 1:
            state_summary += f" {len(connect_q)} relation(s) could be made between things that already exist."
        elif connect_q:
            state_summary += " One relation could be made between things that already exist."

        frontier_q: list[tuple[str, str]] = []
        if len(unmapped) > 1:
            frontier_q = [(n, describe_frontier_node(n, state)) for n in unmapped]
            state_summary += (
                f" {len(unmapped)} opened ways stand unbuilt; whichever you press into is where"
                " the dungeon takes shape next."
            )

        # INTEREST: one independent yes/no per option, growth first.
        # Asked over the whole unmapped frontier, not just when there is
        # more than one -- "is this worth building at all" is a real
        # question even with a single edge, and it is the question the
        # old `choice` could not ask (a choice over one option has
        # probability 1 by construction, so it always says yes).
        interest_q, interest_over = (
            interest_questions(interest, legal, unmapped, connect_q, state)
            if interest.enabled
            else ([], 0)
        )
        if interest_q:
            state_summary += (
                f" {len(interest_q)} option(s) carry an independent interest score this round;"
                " several may be worth acting on at once, or none may be."
            )
        scores: dict[str, float] = {}

        if not groups and not frontier_q and not binding_q and len(connect_q) < 2 and not interest_q:
            # Nothing to decide: no legal action, and at most one opened
            # way, which is not a choice but the only way on. Calling Jev
            # here would spend the one genuinely scarce resource to ask
            # an empty question -- exactly the leak this loop is supposed
            # to be careful about. Build and move on.
            winners = []
            pressed = unmapped[0] if unmapped else None
            connected = connect_q[0] if connect_q else None
            jev_response = {"skipped": "nothing to decide"}
            print(f"round {round_no}: nothing to decide -- the world takes shape without a question")
        elif args.dry_run:
            # Every choice a live run would put to Jev, made by the drift
            # field instead -- same four decisions, no thumb on the list
            # order. Each gets its own salt so the four fields are
            # independent: where the delve presses should not be
            # correlated with which rock it takes.
            winners = [
                max(group, key=lambda co: (drift(co[0].id, round_no, f"gen{i}"), co[0].id))
                for i, group in enumerate(groups)
            ]
            pressed = drift_pick(unmapped, round_no, "frontier") if unmapped else None
            connected = (
                max(connect_q, key=lambda q: (drift(q[0], round_no, "connect"), q[0]))
                if connect_q
                else None
            )
            for c, var, opts in pending:
                pick = drift_pick(opts, round_no, f"bind|{c.id}|{var}")
                c.params[var] = pick
                c.bound_params.add(var)
                print(f"round {round_no}: bound {c.id}'s ?{var} = {pick} (dry-run: drift)")
            for key, _statement in interest_q:
                scores[key] = drift_interest(key, round_no, "interest")
            jev_response = {"dry_run": True, "chooser": "perlin-drift"}
        else:
            jev_response = call_jev_batch(
                args.api_key,
                jev_cfg["model"],
                jev_cfg["instructions"],
                state_summary,
                groups,
                frontier_q,
                binding_q,
                connect_q,
                interest_q,
            )
            answers = jev_response.get("answers") if isinstance(jev_response, dict) else None
            if not isinstance(answers, dict):
                print(
                    f"fatal: Jev's round {round_no} response didn't have the expected shape (answers): "
                    f"raw response: {json.dumps(jev_response)}",
                    file=sys.stderr,
                )
                sys.exit(4)
            # Interest is read BEFORE anything else is validated: a
            # missing score is not fatal the way a missing choice is.
            # A choice not answered means the round cannot proceed; an
            # interest not answered means one option goes unscored, and
            # falling over for that would make the observability more
            # brittle than the thing it observes.
            for key, _statement in interest_q:
                v = read_interest(answers, key)
                if v is not None:
                    scores[key] = v
            missing = len(interest_q) - len(scores)
            if missing:
                print(f"  interest: {missing} of {len(interest_q)} question(s) came back unscored")

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

            # Which thing each ambiguous action acts on. A malformed or
            # off-menu answer is fatal for the same reason an action
            # answer is: silently picking would put the choice back with
            # the engine, which is exactly what refusing it was for.
            for (key, var, _desc, opts), (c, _v, _o) in zip(binding_q, pending):
                try:
                    pick = answers[key]["choice"]
                except (KeyError, TypeError) as e:
                    print(
                        f"fatal: Jev's round {round_no} response missing/malformed {key!r}: {e!r}",
                        file=sys.stderr,
                    )
                    sys.exit(4)
                if pick not in opts:
                    print(
                        f"fatal: Jev chose {pick!r} for {key!r}, not among {opts}",
                        file=sys.stderr,
                    )
                    sys.exit(4)
                c.params[var] = pick
                c.bound_params.add(var)
                print(f"round {round_no}: bound {c.id}'s ?{var} = {pick}")

            # Which relation becomes real.
            connected = None
            if len(connect_q) > 1:
                try:
                    pick = answers["connect"]["choice"]
                except (KeyError, TypeError) as e:
                    print(f"fatal: round {round_no} missing/malformed 'connect' answer: {e!r}", file=sys.stderr)
                    sys.exit(4)
                match = [c for c in connect_q if c[0] == pick]
                if not match:
                    print(f"fatal: Jev chose relation {pick!r}, not among {[c[0] for c in connect_q]}", file=sys.stderr)
                    sys.exit(4)
                connected = match[0]
            elif connect_q:
                connected = connect_q[0]

            # Where the delve presses on. One unmapped edge needs no
            # question; several do, and a malformed answer is fatal for
            # the same reason a malformed action answer is -- silently
            # picking for Jev would put this file's thumb back on the
            # single knob the whole change exists to hand over.
            if len(unmapped) > 1:
                try:
                    pressed = answers["frontier"]["choice"]
                except (KeyError, TypeError) as e:
                    print(
                        f"fatal: Jev's round {round_no} response missing/malformed 'frontier' answer: {e!r}\n"
                        f"raw response: {json.dumps(jev_response)}",
                        file=sys.stderr,
                    )
                    sys.exit(4)
                if pressed not in unmapped:
                    print(
                        f"fatal: Jev chose to press into {pressed!r}, which is not among the unmapped "
                        f"edges {unmapped}",
                        file=sys.stderr,
                    )
                    sys.exit(4)
            else:
                pressed = unmapped[0] if unmapped else None

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

        # TWO DESIRES.
        #
        # The delve's, first: build where Jev chose to press on. Pull,
        # not push -- the world appears because someone went there, and
        # the edges nobody chose stay unbuilt and still open. That is not
        # an optimization, it is the only honest way to spend a budget
        # whose real denomination is attention: building where nobody
        # went spends the scarce thing on nothing.
        #
        # Then the world's own, on an explicit allocation: every Nth
        # round, build somewhere that was NOT chosen. A world that only
        # ever grows where you look is summoned rather than inhabited --
        # nothing arrives unbidden, nothing else in the dungeon wants
        # anything. Buying that back costs real tokens, so it is a line
        # item, deliberately spent and reported, never free.
        minted_machines = []
        # Every anchor this round's growth has already spent on. The
        # unbidden allocation below must avoid all of them, not just one:
        # it read a single `anchor` variable that only the pre-fan-out
        # path ever assigned, so with interest driving growth the name
        # was simply never bound. It crashed at the first round where
        # `unbidden_every` came due -- in the first run that ever had
        # both switched on at once, which is exactly the kind of bug a
        # full run finds and a feature-at-a-time run cannot.
        anchors_used: list[str] = []
        state.interest_seen.extend(scores.values())
        # Record what the chooser WANTED, separately from what it picked.
        for c, _out in legal:
            v = scores.get(f"interest_action|{c.id}")
            if v is not None:
                c.interest = v

        if not stop_reason and extend.enabled:
            # THE FAN-OUT. Everything above the bar, in one pass.
            #
            # This is the change interest exists for. The loop used to
            # build at exactly one anchor per round, because it asked a
            # `choice` and a choice returns one answer. Interest is
            # independent per option, so six open edges can all be worth
            # pressing into and all six get built now -- and, just as
            # importantly, six edges can all come back cold and NOTHING
            # gets built, which no `choice` over those same six could
            # ever have said.
            # Relations before edges when density is the goal. The
            # allowance is the same either way; what changes is which
            # kind of growth gets starved when it runs out, and until now
            # that was always relations.
            edge_room = (
                max(0, interest.max_growth_per_round - len(connect_q))
                if extend.relations_first
                else interest.max_growth_per_round
            )
            rated = sorted(
                ((v, n) for n in unmapped for v in [scores.get(f"interest_frontier|{n}")] if v is not None),
                reverse=True,
            )
            wanted, cold = pick_wanted(rated, state, interest, round_no, "frontier")

            if interest.enabled and rated:
                _n, mu, sd = state.interest_baseline(interest)
                if wanted:
                    print(
                        f"  interest: {len(wanted)} of {len(rated)} open way(s) taken "
                        f"(run baseline {mu:.2f}+-{sd:.2f}) -- "
                        + ", ".join(
                            f"{n} {v:.2f} p={interest_probability(v, state, interest):.2f}"
                            for v, n in wanted[: interest.max_growth_per_round]
                        )
                        + (f" | left: {', '.join(f'{n} {v:.2f}' for v, n in cold[:3])}" if cold else "")
                    )
                else:
                    print(
                        f"  interest: nothing taken -- every open way sits at or below this run's "
                        f"own baseline {mu:.2f}+-{sd:.2f} (best {rated[0][1]} at {rated[0][0]:.2f}); "
                        "the world does not grow this round"
                    )
                for v, n in wanted[:edge_room]:
                    m = extend_world(state, extend, world_dir, round_no, n, bidden=True)
                    if m:
                        m["interest"] = v
                        minted_machines.append(m)
                        anchors_used.append(n)
            else:
                # No interest signal: the original single-anchor path,
                # kept intact so a config without `interest` behaves
                # exactly as it did before any of this.
                anchor = pressed or (leaves[0] if leaves else None)
                if anchor:
                    m = extend_world(state, extend, world_dir, round_no, anchor, bidden=bool(pressed))
                    if m:
                        minted_machines.append(m)
                        anchors_used.append(anchor)

            # Relations, same shape: all of them that are wanted, not the
            # one that won a popularity contest among them.
            if interest.enabled and connect_q:
                room = max(0, interest.max_growth_per_round - len(minted_machines))
                by_id = {cid: (desc, cargs) for cid, desc, cargs in connect_q}
                rel_rated = sorted(
                    (
                        (v, cid)
                        for cid in by_id
                        for v in [scores.get(f"interest_connect|{cid}")]
                        if v is not None
                    ),
                    reverse=True,
                )
                rel_wanted, _rel_cold = pick_wanted(rel_rated, state, interest, round_no, "connect")
                made = [(v, cid, *by_id[cid]) for v, cid in rel_wanted][:room]
                if not made and rel_rated:
                    # Two different reasons for building no relation, and
                    # conflating them made the log lie: the first live
                    # fan-out printed "no relation reaches the floor 0.35
                    # (best 0.48)" when 0.48 clears 0.35 easily and the
                    # real reason was that growth had already spent the
                    # round's whole allowance on frontier edges.
                    if rel_wanted:
                        print(
                            f"  interest: {len(rel_wanted)} relation(s) wanted but this round's growth "
                            f"allowance ({interest.max_growth_per_round}) is already spent"
                        )
                    else:
                        print(
                            f"  interest: no relation taken (best {rel_rated[0][1]} at "
                            f"{rel_rated[0][0]:.2f}, below this run's baseline)"
                        )
                for v, cid, desc, cargs in made:
                    m = mint(state, world_dir, round_no, cargs[0], cargs, desc, cargs[2], bidden=True)
                    if m:
                        m["relation"] = cid
                        m["interest"] = v
                        minted_machines.append(m)
                        print(f"  connect: {cid} (interest {v:.2f})")
            elif connected:
                # The relation, if one was chosen. Deliberately AFTER the
                # arborescent shot: extending and thickening are different
                # moves and a round may legitimately do both.
                cid, desc, cargs = connected
                m = mint(state, world_dir, round_no, cargs[0], cargs, desc, cargs[2], bidden=True)
                if m:
                    m["relation"] = cid
                    minted_machines.append(m)
                    print(f"  connect: {cid}")

            # THE SHAPE ALLOCATION. Spent, not chosen.
            if extend.shape_every and round_no % extend.shape_every == 0:
                already = {m.get("relation") for m in minted_machines}
                openers = [q for q in connect_q if q[0] not in already]
                # `imply` first -- it is the one that opens the
                # TRANSITION-SHAPE vocabulary, and the one interest most
                # reliably refuses. The substance-vocabulary openers are
                # the fallback: still outside what recombination can
                # reach, still unpriceable locally.
                ranked = [q for q in openers if q[0].startswith("imply")] or [
                    q for q in openers if q[0].startswith(("transmute", "wellspring"))
                ]
                if ranked:
                    # Interest breaks the tie and nothing more. Which
                    # opener is spent on is worth asking; WHETHER to
                    # spend is not, because that is the question the
                    # signal cannot answer.
                    cid, desc, cargs = max(
                        ranked, key=lambda q: (scores.get(f"interest_connect|{q[0]}", 0.0), q[0])
                    )
                    v = scores.get(f"interest_connect|{cid}")
                    m = mint(state, world_dir, round_no, cargs[0], cargs, desc, cargs[2], bidden=False)
                    if m:
                        m["relation"] = cid
                        m["allocation"] = "shape"
                        if v is not None:
                            m["interest"] = v
                        minted_machines.append(m)
                        said = f"interest {v:.2f}" if v is not None else "unscored"
                        print(
                            f"  shape allocation: built {cid} ({said}) -- spent, not chosen. "
                            "A new shape makes no round better; it makes every later round wider."
                        )
                else:
                    print("  shape allocation: due, but nothing left to open this round")

            if extend.unbidden_every and round_no % extend.unbidden_every == 0:
                # Somewhere the delve did NOT choose, if there is such a
                # place; otherwise any standing edge. Deterministic, so a
                # --dry-run rehearsal spends the allocation exactly where
                # a live run will.
                remaining = [n for n in growable_leaves(state) if n not in anchors_used]
                elsewhere = remaining[0] if remaining else None
                if elsewhere:
                    m = extend_world(state, extend, world_dir, round_no, elsewhere, bidden=False)
                    if m:
                        minted_machines.append(m)
                else:
                    print("  extend: unbidden allocation due, but nowhere anchorable to spend it")

        record = {
            "round": round_no,
            "state_summary": state_summary,
            "legal_candidate_ids": [c.id for c, _ in legal],
            "groups": [[c.id for c, _ in g] for g in groups],
            "fact_native_discovered": fact_native,
            "jev_response": jev_response,
            "chosen": [w.id for w, _ in applied],
            "minted_nodes_this_round": minted_total,
            "unmapped_frontier": unmapped,
            "growable_leaves": leaves,
            "pressed_into": pressed,
            "relations_offered": [c[0] for c in connect_q],
            "relation_made": connected[0] if connected else None,
            "bindings_resolved": {c.id: dict(c.params) for c, _, _ in pending},
            "minted_machines_this_round": minted_machines,
            # Every option's interest this round, keyed exactly as asked
            # (interest_frontier|node, interest_connect|id,
            # interest_action|id). Logged whole rather than summarized:
            # the interesting reading afterwards is usually what was
            # WANTED and not taken, which a summary throws away.
            "interest": scores,
            "interest_unasked": interest_over,
            "jev_perf": (jev_response or {}).get("_perf"),
            "total_firings": state.total_firings,
        }
        audit.write(json.dumps(record) + "\n")
        audit.flush()

        if stop_reason:
            print(f"=== round {round_no}: {stop_reason} ===")
            break
        round_no += 1

    if extend.enabled:
        print(f"minted {state.minted_machines} machine(s) mid-run, where the delve pressed and where it didn't")
    print(f"world dir: {world_dir}")
    print(f"audit log: {audit_path}")
    audit.close()


def subprocess_mkdtemp() -> str:
    import tempfile

    return tempfile.mkdtemp(prefix="jev-driver-")


if __name__ == "__main__":
    main()
