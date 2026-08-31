#!/usr/bin/env python3
"""Follow-up to Round 11: the GA found every genome near-ceiling
(fitness ~1.00) under controlled conditions -- including round9-
negation and round10-positive, the exact texts that scored 68.6%/71.3%
could-not-form-commit in the real 90-second arena. That means the
controlled setup removed whatever was actually causing the collapse;
the GA's search had no real landscape to climb. Two confounds got
removed at once when isolating the framing paragraph: (1) the fixed
intro was shortened (dropped "other agents may be acting... the world
can change... that's expected, not an error"), and (2) every trial used
the simple seed state (5 param-less legal actions) instead of the
actual arena's evolving states, some of which offer parameterized
actions (mix/build/add_roof, nested params objects).

This isolates which confound matters, cheaply, before running a much
more expensive "faithful" GA: same genome text (round9-negation, the
one real historical text with a known 84%-failure real-world rate),
four conditions, 6 trials each.
"""
import json
import os
import socket
import subprocess
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor

API_KEY = os.environ["OPENROUTER_API_KEY"]
MODEL = "deepseek/deepseek-v4-flash-0731"
ARENA_PORT = 7882
DMML_REPO = "/home/user/dmml"
NODE_REF_PATTERN = r"^[A-Za-z0-9][A-Za-z0-9_.]*(/[A-Za-z0-9][A-Za-z0-9_.]*)*$"
TRIALS = 6

GENOME = "There is no single goal here; take any action that seems worthwhile, interesting, or that develops the world further. A legal move that turns out badly is fine -- that's just what happened, not a failure to avoid at all costs."

SHORT_INTRO = "You are one of several agents ({model}) acting in the shared world of Valinor."
FULL_INTRO = "You are one of several agents ({model}) acting concurrently in the shared world of Valinor -- other agents may be acting at the same time as you, and the world can change between when you read this and when your action actually lands."

CLOSING_SHORT = "Choose ONE action, and respond with only the JSON object matching the schema."
CLOSING_FULL = "Choose ONE action. Respond with only the JSON object matching the schema -- the schema enumerates every action that was legal the moment you queried; by the time it lands, it may no longer be, and that's an expected part of acting in a live, shared world, not an error."


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


def call_model(prompt, schema):
    body = {
        "model": MODEL, "max_tokens": 1500,
        "messages": [{"role": "user", "content": prompt}],
        "reasoning": {"enabled": False},
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
            content = data["choices"][0]["message"].get("content")
            if content:
                return content
        except Exception:
            time.sleep(2 * (attempt + 1))
    return None


def trial(intro, closing, state, legal_actions, schema):
    prompt = f"{intro.format(model=MODEL)}\n\n{GENOME}\n\nCurrent world state:\n{format_state(state)}\n\n{closing}"
    raw = call_model(prompt, schema)
    if not raw:
        return False
    cleaned = raw.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
    try:
        choice = json.loads(cleaned)
    except Exception:
        return False
    return is_schema_conformant(choice, legal_actions)


def eval_condition(name, intro, closing, state, legal_actions, schema):
    with ThreadPoolExecutor(max_workers=TRIALS) as pool:
        results = list(pool.map(lambda _: trial(intro, closing, state, legal_actions, schema), range(TRIALS)))
    rate = sum(results) / len(results)
    print(f"  {name}: {sum(results)}/{len(results)} = {rate:.2f}")
    return rate


def arena_query_once(port):
    server = subprocess.Popen(["cargo", "run", "-q", "-p", "dmml", "--example", "episode_arena", "--", str(port)], cwd=DMML_REPO)
    for _ in range(30):
        try:
            socket.create_connection(("127.0.0.1", port), timeout=1).close()
            break
        except OSError:
            time.sleep(1)
    return server


def arena_call(port, req):
    s = socket.create_connection(("127.0.0.1", port), timeout=5)
    s.sendall((json.dumps(req) + "\n").encode())
    resp = json.loads(s.makefile().readline())
    s.close()
    return resp


def main():
    server = arena_query_once(ARENA_PORT)
    seed = arena_call(ARENA_PORT, {"query": True})
    print(f"Seed state: {len(seed['legal_actions'])} legal actions, all param-less.\n")

    # Build a real state that offers a parameterized action: raise, uplift,
    # quarry, grind (sand), wash, well_up -- then mix becomes legal with real params.
    for action in [
        {"node": "Valinor", "transition": "raise", "params": None},
        {"node": "Valinor", "transition": "uplift", "params": None},
        {"node": "Valinor/quarry", "transition": "quarry", "params": None},
        {"node": "Valinor/quarry", "transition": "grind", "params": None},
        {"node": "Valinor/streambed", "transition": "wash", "params": None},
        {"node": "Valinor/spring", "transition": "well_up", "params": None},
    ]:
        arena_call(ARENA_PORT, {"actor": "setup", **action})
    param_state = arena_call(ARENA_PORT, {"query": True})
    print(f"Param state: {len(param_state['legal_actions'])} legal actions, including: {[a for a in param_state['legal_actions'] if a['params']]}\n")

    server.terminate()
    server.wait(timeout=10)

    seed_schema = build_schema(seed["legal_actions"])
    param_schema = build_schema(param_state["legal_actions"])

    print("=== Isolating the confound: same genome text, 4 conditions ===\n")
    results = {}
    results["short_intro + seed_state"] = eval_condition("short_intro + seed_state", SHORT_INTRO, CLOSING_SHORT, seed["state"], seed["legal_actions"], seed_schema)
    results["full_intro + seed_state"] = eval_condition("full_intro  + seed_state", FULL_INTRO, CLOSING_FULL, seed["state"], seed["legal_actions"], seed_schema)
    results["short_intro + param_state"] = eval_condition("short_intro + param_state", SHORT_INTRO, CLOSING_SHORT, param_state["state"], param_state["legal_actions"], param_schema)
    results["full_intro + param_state"] = eval_condition("full_intro  + param_state", FULL_INTRO, CLOSING_FULL, param_state["state"], param_state["legal_actions"], param_schema)

    out_path = os.path.join(os.path.dirname(__file__), "PROMPT-EVOLUTION-FOLLOWUP-2026-08-31.json")
    json.dump(results, open(out_path, "w"), indent=2)
    print(f"\nWritten to {out_path}")


if __name__ == "__main__":
    main()
