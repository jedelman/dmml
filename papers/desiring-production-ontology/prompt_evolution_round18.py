#!/usr/bin/env python3
"""Round 18: "YES. evolve the prompt." (Jason, 2026-08-31), directly
after Round 17 found that a maximally minimal prompt regresses badly
under real live-arena conditions (73-75% could-not-form-commit) even
though Round 11's isolated GA never once found a genome scoring below
0.83 out of 5 candidates across 3 generations.

That gap is the real methodological problem this file exists to fix,
not politely worked around: Round 11's GA (`prompt_evolution.py`)
evaluated every genome against ONE deepseek call, ONE frozen static
seed state, no other agents, no mutex, no live arena at all -- exactly
the isolated condition Round 12 later showed produces near-ceiling
conformance for EVERY prompt variant tried, regardless of content.
Checked directly just now: Round 11's own logged generation-1 fitnesses
were 1.00, 1.00, 1.00, 1.00, 1.00 -- five genuinely different prompts,
zero variance, nothing for selection to act on. The GA wasn't finding
good prompts; it was running in a condition with no fitness gradient at
all, because the isolated harness can't reproduce the collapse Rounds
9/15/17 actually measured. Optimizing there was optimizing noise.

This version fixes that: fitness is now measured INSIDE a real,
live `episode_arena` session, under the exact mutex-protected,
multi-agent condition Round 15-17 showed the effect actually lives in.
Three background agents (`glm-4.7-flash`, `gemini-2.5-flash-lite`,
`gemini-3.1-flash-lite`) run continuously for the whole GA's duration,
using Round 15/16's already-established good fixed framing, sharing
the SAME `asyncio.Lock` turn-taking mutex every other Round 15+ script
uses. `deepseek` is the one model whose framing text evolves: each
candidate genome gets evaluated by having deepseek take K=6 REAL turns
-- competing fairly for the mutex against the three live background
agents, seeing real, live, other-agent-caused world drift -- and
fitness is its schema-conformance rate over those real turns. This is
slower and more expensive than Round 11's isolated version (a live
multi-agent session has to run for every genome, not a batch of
isolated calls), but it's testing the actual mechanism, not a
condition where the mechanism is absent.

Initial population spans the spectrum this project has now actually
tested, not five wordings of the same paragraph:
  empty          -- Round 17's finding, tested directly: zero framing
  one-sentence   -- the precise next test Round 17 named but hadn't run
  round15-full   -- the exact framing Round 15/16 already used
  round9-negation / round10-positive -- kept as known reference points

Second bug, caught on the FIRST real run of this file and fixed before
trusting any result from it: sharing ONE long-running arena across the
whole GA (one world, one set of background agents, for all 3
generations) meant the house-world -- finite, and reachably dead-ended
(`overgather` firing before `make_frame` permanently blocks the roof/
house chain, confirmed back in Round 7) -- actually got exhausted
partway through generation 1. Real timestamps confirm it: the last
landed commit was at t=80.6s into an otherwise-long run; every trial
after that point, across the rest of gen 1 and the entirety of gens 2
and 3, was being asked to choose from `legal_actions: []` -- an
unsatisfiable schema (`{"anyOf": []}`) that fails near-instantly and
different in kind from a genuine could-not-form-commit failure. That
run's fitnesses (`PROMPT-EVOLUTION-2026-08-31-round18-BROKEN-world-
exhausted.json`, kept rather than deleted) are not a real result --
almost every genome scored a meaningless 0.00 because almost every
genome was tested against a broken world, not because of anything about
its text. Fixed by giving every genome its own fresh, isolated arena
instance (own port, own seed, own three background agents, torn down
after its K trials) -- no genome is ever measured against a world
another genome's own traffic exhausted.

Genetic operators are the same mechanical text transforms Round 11
used (crossover splices sentences, mutation is a fixed set of
programmatic transforms) -- unchanged, since the flaw was in what
fitness was measured against, not in the operators themselves.
"""
import asyncio
import json
import os
import random
import re
import socket
import subprocess
import sys
import time
import urllib.request
import urllib.error

API_KEY = os.environ["OPENROUTER_API_KEY"]
ARENA_PORT = 7883
DMML_REPO = "/home/user/dmml"
NODE_REF_PATTERN = r"^[A-Za-z0-9][A-Za-z0-9_.]*(/[A-Za-z0-9][A-Za-z0-9_.]*)*$"

EVOLVING_MODEL = "deepseek/deepseek-v4-flash-0731"
EVOLVING_REASONING = {"enabled": False}
BACKGROUND_MODELS = {
    "z-ai/glm-4.7-flash": {"enabled": False},
    "google/gemini-2.5-flash-lite": {"enabled": False},
    "google/gemini-3.1-flash-lite": {"enabled": False},
}
FIXED_BACKGROUND_FRAMING = "You are one of several agents ({model}) building and shaping the living world of Valinor together, all acting at the same time. Explore freely and take whichever action feels most worthwhile or interesting right now -- anything that grows, builds, or develops the world further is a good choice."

POPULATION_SIZE = 5
GENERATIONS = 3
TRIALS_PER_GENOME = 6
RNG_SEED = 20260831

INITIAL_POPULATION = [
    ("empty", ""),
    ("one-sentence", "You are choosing one action in a shared, live world."),
    ("round15-full", FIXED_BACKGROUND_FRAMING),
    ("round9-negation", "There is no single goal here; take any action that seems worthwhile, interesting, or that develops the world further. A legal move that turns out badly is fine -- that's just what happened, not a failure to avoid at all costs."),
    ("round10-positive", "Explore freely and take whichever action feels most worthwhile or interesting right now -- anything that grows, builds, or develops the world further is a good choice."),
]


def build_schema(legal_actions):
    branches = []
    for action in legal_actions:
        node, transition, params = action["node"], action["transition"], action["params"]
        param_names = sorted(params.keys()) if params else []
        props = {"node": {"type": "string", "const": node}, "transition": {"type": "string", "const": transition}}
        required = ["node", "transition"]
        if param_names:
            param_props = {p: {"type": "string", "pattern": NODE_REF_PATTERN} for p in param_names}
            props["params"] = {"type": "object", "properties": param_props, "required": param_names, "additionalProperties": False}
            required.append("params")
        else:
            props["params"] = {"type": "null"}
            required.append("params")
        branches.append({"type": "object", "properties": props, "required": required, "additionalProperties": False})
    return {"anyOf": branches}


def is_schema_conformant(choice, legal_actions):
    for a in legal_actions:
        if choice.get("node") != a["node"] or choice.get("transition") != a["transition"]:
            continue
        if (choice.get("params") or {}) == (a["params"] or {}):
            return True
    return False


def format_state(state):
    return "\n".join(f"  {node:<20} state: {value}" for node, value in sorted(state.items()))


def format_drift(changes):
    if not changes:
        return ""
    lines = []
    for c in changes:
        before = c["before"]["Node"] if c["before"] else "(none)"
        after = c["after"]["Node"] if c["after"] else "(retracted)"
        lines.append(f"  {c['subject']}: {before} -> {after}")
    return "Changed since you last looked (not caused by you):\n" + "\n".join(lines) + "\n\n"


def build_prompt(model, framing_text, state, drift):
    framing = framing_text.format(model=model) if framing_text else ""
    sep = "\n\n" if framing else ""
    return f"""{framing}{sep}{format_drift(drift)}Current world state:
{format_state(state)}

Choose ONE action, and respond with only the JSON object matching the
schema."""


def call_model(model, prompt, schema, reasoning):
    body = {
        "model": model, "max_tokens": 1500,
        "messages": [{"role": "user", "content": prompt}],
        "reasoning": reasoning,
        "response_format": {"type": "json_schema", "json_schema": {"name": "action", "strict": True, "schema": schema}},
    }
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
    )
    for attempt in range(3):
        try:
            with urllib.request.urlopen(req, timeout=45) as resp:
                data = json.loads(resp.read())
            if "error" in data:
                raise RuntimeError(data["error"])
            content = data["choices"][0]["message"].get("content")
            if not content:
                raise RuntimeError("empty content")
            return content, None
        except urllib.error.HTTPError as e:
            last_err = f"HTTP {e.code}: {e.read().decode(errors='replace')[:200]}"
        except Exception as e:
            last_err = str(e)
        time.sleep(2 * (attempt + 1))
    return None, last_err


def arena_call(port, req):
    s = socket.create_connection(("127.0.0.1", port), timeout=10)
    s.sendall((json.dumps(req) + "\n").encode())
    resp = s.makefile().readline()
    s.close()
    return json.loads(resp)


async def background_agent_loop(port, model, reasoning, stop_event, loop, turn_lock):
    """Keeps the world alive and genuinely multi-agent for one genome's
    evaluation window -- fixed, already-known-good framing, not
    evolving. Failures are silent here (this isn't what's being
    measured); the point is real, ongoing, other-agent-caused drift for
    the evolving genome to be tested against. Stops treating an empty
    legal_actions list (world exhausted -- a real, permanent dead end
    this bounded house-world can genuinely reach, confirmed the hard
    way in this file's first version) as anything to act on; it just
    waits for the next generation's fresh world instead of hammering an
    unsatisfiable schema."""
    while not stop_event.is_set():
        async with turn_lock:
            try:
                query = await loop.run_in_executor(None, arena_call, port, {"query": True, "actor": model})
                if "state" not in query or not query.get("legal_actions"):
                    continue
                state, legal_actions, drift = query["state"], query["legal_actions"], query.get("changed_since_you_last_looked", [])
                schema = build_schema(legal_actions)
                prompt = build_prompt(model, FIXED_BACKGROUND_FRAMING, state, drift)
                raw, err = await loop.run_in_executor(None, call_model, model, prompt, schema, reasoning)
                if err or not raw:
                    continue
                cleaned = raw.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
                choice = json.loads(cleaned)
                node, transition = choice["node"], choice["transition"]
                params = choice.get("params") or {}
                await loop.run_in_executor(None, arena_call, port, {"actor": model, "node": node, "transition": transition, "params": params})
            except Exception:
                continue


async def evaluate_trial(port, genome_text, loop, turn_lock):
    async with turn_lock:
        query = await loop.run_in_executor(None, arena_call, port, {"query": True, "actor": EVOLVING_MODEL})
        if "state" not in query:
            return None
        legal_actions = query["legal_actions"]
        if not legal_actions:
            # Real dead end, not a fitness signal (confirmed the hard
            # way: the first version of this file scored every genome
            # after commit 13 against exactly this, meaninglessly).
            return None
        state, drift = query["state"], query.get("changed_since_you_last_looked", [])
        schema = build_schema(legal_actions)
        prompt = build_prompt(EVOLVING_MODEL, genome_text, state, drift)
        raw, err = await loop.run_in_executor(None, call_model, EVOLVING_MODEL, prompt, schema, EVOLVING_REASONING)
        if err or not raw:
            return False
        try:
            cleaned = raw.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
            choice = json.loads(cleaned)
        except Exception:
            return False
        conformant = is_schema_conformant(choice, legal_actions)
        if conformant:
            params = choice.get("params") or {}
            await loop.run_in_executor(None, arena_call, port, {"actor": EVOLVING_MODEL, "node": choice["node"], "transition": choice["transition"], "params": params})
        return conformant


async def evaluate_genome(genome_text, gen, idx, loop):
    """Every genome gets a fully fresh, isolated arena instance --
    own port, own seed world, own three background agents, torn down
    afterward -- so no genome is ever measured against a world another
    genome's trials (or its own background traffic) already exhausted.
    The house-world is finite and reachably dead-ended (confirmed:
    overgather firing before make_frame permanently blocks the roof/
    house chain), so sharing one long-running world across the whole
    GA run was never a fair comparison in the first place."""
    port = ARENA_PORT + 1 + (gen * 10 + idx)
    server = subprocess.Popen(["cargo", "run", "-q", "-p", "dmml", "--example", "episode_arena", "--", str(port)], cwd=DMML_REPO, stderr=subprocess.DEVNULL)
    for _ in range(30):
        try:
            socket.create_connection(("127.0.0.1", port), timeout=1).close()
            break
        except OSError:
            await asyncio.sleep(1)
    else:
        server.terminate()
        raise RuntimeError(f"arena on port {port} never came up")

    loop_ = loop
    turn_lock = asyncio.Lock()
    stop_event = asyncio.Event()
    background_tasks = [asyncio.create_task(background_agent_loop(port, m, r, stop_event, loop_, turn_lock)) for m, r in BACKGROUND_MODELS.items()]

    try:
        results = []
        while len(results) < TRIALS_PER_GENOME:
            outcome = await evaluate_trial(port, genome_text, loop_, turn_lock)
            if outcome is None:
                # World exhausted before this genome got its full K
                # trials -- stop early rather than pad with meaningless
                # empty-schema measurements; note it in the result.
                break
            results.append(outcome)
    finally:
        stop_event.set()
        for t in background_tasks:
            t.cancel()
        server.terminate()
        server.wait(timeout=10)

    if not results:
        return 0.0, 0
    return sum(results) / len(results), len(results)


# ---- Genetic operators: same mechanical text transforms as Round 11 ----

def split_sentences(text):
    if not text:
        return []
    parts = re.split(r"(?<=[.!?])\s+", text.strip())
    return [p for p in parts if p]


SYNONYMS = {
    "worthwhile": "meaningful", "interesting": "impactful", "explore": "build",
    "freely": "boldly", "grows": "advances", "develops": "extends", "choice": "move",
}


def mutate(genome_text, rng):
    if not genome_text:
        return genome_text, "noop-empty"
    op = rng.choice(["synonym_swap", "sentence_shuffle", "append_schema_reminder", "prepend_emphasis", "trim_to_half"])
    sentences = split_sentences(genome_text)
    if op == "synonym_swap":
        words = genome_text.split(" ")
        words = [SYNONYMS.get(w.strip(".,"), w) if rng.random() < 0.5 else w for w in words]
        return " ".join(words), op
    if op == "sentence_shuffle" and len(sentences) > 1:
        rng.shuffle(sentences)
        return " ".join(sentences), op
    if op == "append_schema_reminder":
        return genome_text + " Match the node and transition names exactly as offered.", op
    if op == "prepend_emphasis":
        return "Above all: " + genome_text[0].lower() + genome_text[1:], op
    if op == "trim_to_half" and len(sentences) > 1:
        return " ".join(sentences[: max(1, len(sentences) // 2)]), op
    return genome_text, "noop"


def crossover(parent_a, parent_b, rng):
    sents_a, sents_b = split_sentences(parent_a), split_sentences(parent_b)
    if not sents_a or not sents_b:
        return parent_a or parent_b
    cut_a = max(1, len(sents_a) // 2)
    return " ".join(sents_a[:cut_a] + sents_b[len(sents_b) // 2:])


async def main():
    rng = random.Random(RNG_SEED)
    loop = asyncio.get_event_loop()

    population = [{"id": gid, "text": text, "parents": []} for gid, text in INITIAL_POPULATION]
    history = []

    for gen in range(1, GENERATIONS + 1):
        print(f"=== Generation {gen} ===")
        for idx, ind in enumerate(population):
            fitness, n = await evaluate_genome(ind["text"], gen, idx, loop)
            ind["fitness"] = fitness
            ind["trials_completed"] = n
            label = ind["text"][:70] + ("..." if len(ind["text"]) > 70 else "") if ind["text"] else "(empty)"
            short_note = "" if n == TRIALS_PER_GENOME else f"  [world exhausted after {n}/{TRIALS_PER_GENOME} trials]"
            print(f"  {ind['id']:<24} fitness={ind['fitness']:.2f}  \"{label}\"{short_note}")

        history.append({"generation": gen, "population": [dict(i) for i in population]})

        population.sort(key=lambda i: i["fitness"], reverse=True)
        elites = population[:2]

        if gen == GENERATIONS:
            break

        offspring = []
        child_text = crossover(elites[0]["text"], elites[1]["text"], rng)
        offspring.append({"id": f"gen{gen}-crossover", "text": child_text, "parents": [elites[0]["id"], elites[1]["id"]]})
        for i in range(2):
            parent = rng.choice(elites)
            mutated, op = mutate(parent["text"], rng)
            offspring.append({"id": f"gen{gen}-mutant{i}-{op}", "text": mutated, "parents": [parent["id"]]})

        population = [{"id": e["id"], "text": e["text"], "parents": e["parents"]} for e in elites] + offspring

    out_dir = os.path.dirname(__file__)
    out_path = os.path.join(out_dir, "PROMPT-EVOLUTION-2026-08-31-round18-fresh-world-per-genome.json")
    json.dump(history, open(out_path, "w"), indent=2)

    best = max(history[-1]["population"], key=lambda i: i["fitness"])
    print(f"\n=== Best genome after {GENERATIONS} generations: {best['id']} (fitness={best['fitness']:.2f}) ===")
    print(f'"{best["text"] or "(empty)"}"')
    gen1 = {i["id"]: i["fitness"] for i in history[0]["population"]}
    print("\nGeneration 1 spread (real live-arena fitness, not the isolated-harness ceiling Round 11 hit):")
    for gid, text in INITIAL_POPULATION:
        print(f"  {gid:<20} {gen1.get(gid):.2f}")
    print(f"\nFull history written to {out_path}")


if __name__ == "__main__":
    asyncio.run(main())
