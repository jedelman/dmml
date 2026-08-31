#!/usr/bin/env python3
"""Round 11: "can we run an evolutionary algorithm on the prompt
itself?" (Jason, 2026-08-31), after Round 10 ruled out the specific
"it's the negation" hypothesis by direct test (68.6% -> 71.3%
could-not-form-commit, flat/worse, not fixed). Rather than hand-
guessing more prompt variants one at a time, this runs a real GA over
the text of the framing paragraph that sits between the fixed intro and
the fixed closing instruction -- the exact piece of prompt that
differed between Round 9 and Round 10.

What's fixed across every genome (isolating the variable, same
discipline as controlling for state in Round 6-10's comparisons):
  intro   = "You are one of several agents ({model}) acting in the
             shared world of Valinor."
  <genome>  <- this is what evolves
  state   = the real, live-queried seed-state block (queried once from
             a real episode_arena instance, not hardcoded, same
             discipline as every prior round's "real numbers, not
             copied" checks)
  closing = "Choose ONE action, and respond with only the JSON object
             matching the schema."

Fitness = schema-conformance rate over K real dispatched trials per
genome per generation (the same is_schema_conformant check Round 8
introduced: does the parsed response exactly match one of the offered
legal_actions -- node, transition, params all correct). This is
literally the metric Round 9/10 were fighting over, made the direct
optimization target instead of guessed at by hand.

Scoped tightly on purpose: ONE model (deepseek/deepseek-v4-flash-0731,
this project's best-understood baseline, reasoning disabled -- the
model whose 84%/79% could-not-form-commit rate in Rounds 9-10 was the
least noisy of the four), ONE fixed state snapshot (the real seed,
queried once and reused for every trial so every genome is compared
under identical conditions), K=6 trials per genome per generation,
population 5, 3 generations. A real, if small-scale, run -- not a
simulation of one.

Genetic operators are mechanical text transforms, not another LLM call
asked to "make this prompt better" (that would just be prose-guessing
again, one level removed) -- crossover splices sentences from two
parents, mutation applies one of a fixed set of programmatic
transforms (synonym swap, sentence shuffle, schema-reminder append,
emphasis prepend, truncation). Selection keeps the top-2 by fitness
each generation and re-evaluates them fresh (not just carrying forward
their old score) since fitness here is measured against a stochastic
model and carrying a stale score would be trusting noise.
"""
import json
import os
import random
import re
import subprocess
import socket
import sys
import time
import urllib.request
import urllib.error
from concurrent.futures import ThreadPoolExecutor

API_KEY = os.environ["OPENROUTER_API_KEY"]
MODEL = "deepseek/deepseek-v4-flash-0731"
REASONING = {"enabled": False}
ARENA_PORT = 7881
DMML_REPO = "/home/user/dmml"
NODE_REF_PATTERN = r"^[A-Za-z0-9][A-Za-z0-9_.]*(/[A-Za-z0-9][A-Za-z0-9_.]*)*$"

POPULATION_SIZE = 5
GENERATIONS = 3
TRIALS_PER_GENOME = 6
RNG_SEED = 20260831
random.seed(RNG_SEED)

FIXED_INTRO = "You are one of several agents ({model}) acting in the shared world of Valinor."
FIXED_CLOSING = "Choose ONE action, and respond with only the JSON object matching the schema."

INITIAL_POPULATION = [
    ("round9-negation", "There is no single goal here; take any action that seems worthwhile, interesting, or that develops the world further. A legal move that turns out badly is fine -- that's just what happened, not a failure to avoid at all costs."),
    ("round10-positive", "Explore freely and take whichever action feels most worthwhile or interesting right now -- anything that grows, builds, or develops the world further is a good choice."),
    ("explicit-goal", "Your goal is to build a house here. Work toward that goal by choosing whichever legal step brings the house closer to completion."),
    ("schema-literal", "Pick exactly one of the currently available actions below, using precisely the node and transition names as given. Do not invent new actions or fields."),
    ("worked-example", "For example, a valid choice looks like: node: \"Valinor\", transition: \"raise\", params: null. Pick one real action in that same exact shape."),
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


def build_prompt(genome_text, state):
    state_block = f"Current world state:\n{format_state(state)}"
    return f"{FIXED_INTRO.format(model=MODEL)}\n\n{genome_text}\n\n{state_block}\n\n{FIXED_CLOSING}"


def call_model(prompt, schema):
    body = {
        "model": MODEL, "max_tokens": 1500,
        "messages": [{"role": "user", "content": prompt}],
        "reasoning": REASONING,
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
            return content
        except Exception as e:
            time.sleep(2 * (attempt + 1))
            last_err = e
    raise RuntimeError(f"dispatch failed: {last_err}")


def evaluate_trial(genome_text, state, legal_actions, schema):
    prompt = build_prompt(genome_text, state)
    try:
        raw = call_model(prompt, schema)
        cleaned = raw.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
        choice = json.loads(cleaned)
        return is_schema_conformant(choice, legal_actions)
    except Exception:
        return False


def evaluate_genome(genome_text, state, legal_actions, schema):
    with ThreadPoolExecutor(max_workers=TRIALS_PER_GENOME) as pool:
        results = list(pool.map(lambda _: evaluate_trial(genome_text, state, legal_actions, schema), range(TRIALS_PER_GENOME)))
    return sum(results) / len(results)


# ---- Genetic operators: mechanical text transforms, not another LLM call ----

def split_sentences(text):
    parts = re.split(r"(?<=[.!?])\s+", text.strip())
    return [p for p in parts if p]


SYNONYMS = {
    "worthwhile": "meaningful", "interesting": "impactful", "explore": "build",
    "freely": "boldly", "grows": "advances", "develops": "extends", "choice": "move",
}


def mutate(genome_text, rng):
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
        return parent_a
    cut_a = max(1, len(sents_a) // 2)
    return " ".join(sents_a[:cut_a] + sents_b[len(sents_b) // 2:])


def main():
    rng = random.Random(RNG_SEED)

    print("Starting a throwaway episode_arena just to query the real seed state/legal_actions...")
    server = subprocess.Popen(["cargo", "run", "-q", "-p", "dmml", "--example", "episode_arena", "--", str(ARENA_PORT)], cwd=DMML_REPO)
    for _ in range(30):
        try:
            socket.create_connection(("127.0.0.1", ARENA_PORT), timeout=1).close()
            break
        except OSError:
            time.sleep(1)
    else:
        raise RuntimeError("arena never came up")

    s = socket.create_connection(("127.0.0.1", ARENA_PORT), timeout=5)
    s.sendall((json.dumps({"query": True}) + "\n").encode())
    query = json.loads(s.makefile().readline())
    s.close()
    server.terminate()
    server.wait(timeout=10)

    state, legal_actions = query["state"], query["legal_actions"]
    schema = build_schema(legal_actions)
    print(f"Real seed state queried: {len(legal_actions)} legal actions.\n")

    population = [{"id": gid, "text": text, "parents": []} for gid, text in INITIAL_POPULATION]
    history = []

    for gen in range(1, GENERATIONS + 1):
        print(f"=== Generation {gen} ===")
        for ind in population:
            ind["fitness"] = evaluate_genome(ind["text"], state, legal_actions, schema)
            print(f"  {ind['id']:<24} fitness={ind['fitness']:.2f}  parents={ind['parents']}")
            print(f"      \"{ind['text'][:100]}{'...' if len(ind['text']) > 100 else ''}\"")

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
    out_path = os.path.join(out_dir, "PROMPT-EVOLUTION-2026-08-31.json")
    json.dump(history, open(out_path, "w"), indent=2)

    best = max(history[-1]["population"], key=lambda i: i["fitness"])
    print(f"\n=== Best genome after {GENERATIONS} generations: {best['id']} (fitness={best['fitness']:.2f}) ===")
    print(f'"{best["text"]}"')
    print(f"\nBaseline (round9-negation, gen1) vs round10-positive (gen1) vs best:")
    gen1 = {i["id"]: i["fitness"] for i in history[0]["population"]}
    print(f"  round9-negation:  {gen1.get('round9-negation'):.2f}")
    print(f"  round10-positive: {gen1.get('round10-positive'):.2f}")
    print(f"  best (gen {GENERATIONS}):     {best['fitness']:.2f}")
    print(f"\nFull history written to {out_path}")


if __name__ == "__main__":
    main()
