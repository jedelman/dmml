#!/usr/bin/env python3
"""Round 12: "let's see if we can reproduce the collapse. is it the
concurrency?" (Jason, 2026-08-31), following directly from Round 11's
open question -- negation, framing-paragraph content, intro length,
and state/param complexity are all ruled out now; what's left
unconfirmed is (a) the growing "Actions taken so far" history block the
real arena client appends per-agent turn by turn, and (b) something
about sustained, concurrent, multi-model dispatch load itself.

This isolates (b) specifically, cleanly separated from (a): same four
models as Round 9/10 (deepseek, glm-4.7-flash, gemini-2.5-flash-lite,
gemini-3.1-flash-lite), each in its own unsynchronized loop, dispatched
concurrently for 90 seconds -- structurally identical to
`episode_arena_client.py`'s real run. But the world is FROZEN: every
single call, every model, every trial uses the exact same static seed
state and the exact same genome (`round9-negation`, the real text with
a known 84% real-world failure rate for deepseek). No live engine, no
`episode_arena` connection at all -- nothing changes between calls, and
no history block is ever appended (each call is independent, as if it
were the agent's first turn, every time). If conformance collapses here
anyway, the driver is sustained concurrent request volume/API-layer
effects, not the live-changing world or the growing history. If it
stays near-ceiling like every isolated single-shot test in Round 11,
the growing history block becomes the last real candidate standing.

Also buckets results by elapsed time (15s windows) to check for a
volume/fatigue trend specifically -- conformance degrading as the run
progresses would be real evidence of load-related effects even if the
overall rate looks acceptable in aggregate.
"""
import asyncio
import json
import os
import sys
import time
import urllib.request
import urllib.error

API_KEY = os.environ["OPENROUTER_API_KEY"]
DURATION_SECONDS = 90
NODE_REF_PATTERN = r"^[A-Za-z0-9][A-Za-z0-9_.]*(/[A-Za-z0-9][A-Za-z0-9_.]*)*$"

MODELS = {
    "deepseek/deepseek-v4-flash-0731": {"enabled": False},
    "z-ai/glm-4.7-flash": {"enabled": False},
    "google/gemini-2.5-flash-lite": {"enabled": False},
    "google/gemini-3.1-flash-lite": {"enabled": False},
}

# Frozen seed state -- identical every single call, no engine, no history.
SEED_STATE = {
    "Valinor": "unformed", "Valinor/quarry": "untouched", "Valinor/streambed": "bare",
    "Valinor/spring": "dry", "Valinor/mortar": "unmixed", "Valinor/wall": "unbuilt",
    "Valinor/forest": "full", "Valinor/carpentry": "no_frame", "Valinor/roof": "unroofed",
    "Valinor/house": "unbuilt",
}
SEED_LEGAL_ACTIONS = [
    {"node": "Valinor", "transition": "raise", "params": None},
    {"node": "Valinor/carpentry", "transition": "make_frame", "params": None},
    {"node": "Valinor/forest", "transition": "gather", "params": None},
    {"node": "Valinor/spring", "transition": "well_up", "params": None},
    {"node": "Valinor/streambed", "transition": "wash", "params": None},
]

GENOME = "There is no single goal here; take any action that seems worthwhile, interesting, or that develops the world further. A legal move that turns out badly is fine -- that's just what happened, not a failure to avoid at all costs."
FULL_INTRO = "You are one of several agents ({model}) acting concurrently in the shared world of Valinor -- other agents may be acting at the same time as you, and the world can change between when you read this and when your action actually lands."
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


SCHEMA = build_schema(SEED_LEGAL_ACTIONS)


def call_model(model, prompt, reasoning):
    body = {
        "model": model, "max_tokens": 1500,
        "messages": [{"role": "user", "content": prompt}],
        "reasoning": reasoning,
        "response_format": {"type": "json_schema", "json_schema": {"name": "action", "strict": True, "schema": SCHEMA}},
    }
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read())
        content = data["choices"][0]["message"].get("content")
        return content, None
    except urllib.error.HTTPError as e:
        return None, f"HTTP {e.code}: {e.read().decode(errors='replace')[:200]}"
    except Exception as e:
        return None, str(e)


async def agent_loop(model, reasoning, stop_event, log, loop, start_time):
    prompt = f"{FULL_INTRO.format(model=model)}\n\n{GENOME}\n\nCurrent world state:\n{format_state(SEED_STATE)}\n\n{CLOSING_FULL}"
    while not stop_event.is_set():
        raw, err = await loop.run_in_executor(None, call_model, model, prompt, reasoning)
        elapsed = time.time() - start_time
        if err:
            log.append({"t": elapsed, "actor": model, "outcome": "dispatch_error", "detail": err})
            continue
        cleaned = raw.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
        try:
            choice = json.loads(cleaned)
        except Exception as e:
            log.append({"t": elapsed, "actor": model, "outcome": "could_not_form_commit", "raw": raw, "detail": str(e)})
            continue
        conformant = is_schema_conformant(choice, SEED_LEGAL_ACTIONS)
        log.append({"t": elapsed, "actor": model, "outcome": "conformant" if conformant else "non_conformant", "choice": choice})


async def main():
    print(f"Running {len(MODELS)} models concurrently for {DURATION_SECONDS}s against a FROZEN static state -- no engine, no history, isolating concurrent request volume alone.\n")
    loop = asyncio.get_event_loop()
    stop_event = asyncio.Event()
    log = []
    start_time = time.time()

    tasks = [asyncio.create_task(agent_loop(m, r, stop_event, log, loop, start_time)) for m, r in MODELS.items()]

    async def timer():
        await asyncio.sleep(DURATION_SECONDS)
        stop_event.set()

    await asyncio.gather(timer(), *tasks)

    out_path = os.path.join(os.path.dirname(__file__), "CONCURRENCY-ISOLATION-2026-08-31.json")
    json.dump(log, open(out_path, "w"), indent=2)

    print("=== Overall conformance rate per model ===\n")
    by_actor = {}
    for e in log:
        by_actor.setdefault(e["actor"], {"total": 0, "conformant": 0, "could_not_form_commit": 0, "dispatch_error": 0})
        by_actor[e["actor"]]["total"] += 1
        if e["outcome"] == "conformant":
            by_actor[e["actor"]]["conformant"] += 1
        elif e["outcome"] == "could_not_form_commit":
            by_actor[e["actor"]]["could_not_form_commit"] += 1
        elif e["outcome"] == "dispatch_error":
            by_actor[e["actor"]]["dispatch_error"] += 1
    for actor, s in by_actor.items():
        rate = s["conformant"] / s["total"] if s["total"] else 0
        print(f"  {actor}: {s['conformant']}/{s['total']} = {rate:.2f}  (could_not_form_commit={s['could_not_form_commit']}, dispatch_error={s['dispatch_error']})")

    print("\n=== Conformance rate by 15s time bucket (across all models) ===\n")
    buckets = {}
    for e in log:
        b = int(e["t"] // 15) * 15
        buckets.setdefault(b, {"total": 0, "conformant": 0})
        buckets[b]["total"] += 1
        if e["outcome"] == "conformant":
            buckets[b]["conformant"] += 1
    for b in sorted(buckets):
        s = buckets[b]
        rate = s["conformant"] / s["total"] if s["total"] else 0
        print(f"  t={b:>3}-{b+15:<3}s: {s['conformant']}/{s['total']} = {rate:.2f}")

    print(f"\nFull log written to {out_path}")


if __name__ == "__main__":
    asyncio.run(main())
