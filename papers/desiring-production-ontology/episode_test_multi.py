#!/usr/bin/env python3
"""Round 7: "let's do a parallel test, and with multiple models"
(Jason, 2026-08-31) -- same episode as Round 6 (`episode_test.py`), run
concurrently across every model this project's own eval has used so
far (`deepseek/deepseek-v4-flash-0731`, `z-ai/glm-5.3`,
`openai/gpt-5.2-pro`), each against its OWN independent
`episode_driver.rs` subprocess and its own world, so one model's turns
never affect another's.

Also the first run against the extended grammar: after Round 6 showed
`overgather` firing before it could cost anything (both real runs fired
`make_frame` first, so the trap never actually bit), Jason asked for it
to have a real second consequence. `Valinor/streambed`'s `wash` now
carries the same negated forest-depleted guard `make_frame` does --
confirmed by direct test that overgathering before `wash` permanently
removes `wash` from the legal-action set and fails a direct fire
attempt with `GuardNotSatisfied`.

`episode_driver.rs`'s own doc comment originally overclaimed the
consequence ("no rescue path... permanently unbuildable") reasoning
from the single-mutable-resource limitation named earlier this
session -- checked directly against the real engine and corrected
before this went further: `EXISTS` guards are a momentary check, not a
lock, so `Valinor/quarry` can be cited as `mortar`'s `sand_source`
while transiently `sand` AND still continue its own chain to `brick`
afterward (confirmed: `mix` citing quarry's `sand` succeeds, then
`wet`/`fire` still fire right after with nothing blocked). The real
consequence is narrower: `mix` has to catch `quarry` specifically
during that transient `sand` window; push `quarry` straight through to
`clay`/`brick` first and `Valinor/mortar` is stuck `unmixed` forever
(also confirmed directly: that ordering ends the episode at
`no_legal_actions`). Overgathering early doesn't close the house off
outright -- it collapses two independent sand sources into one narrow
timing window. This run is the first real test of whether any model's
ordering reflects that stake, across three models at once, run in
parallel rather than one at a time.

Each model gets a fresh `episode_driver` subprocess (own world, own
turn counter) and its own OpenRouter dispatch loop, run concurrently in
threads -- independent runs, not one shared episode three models vote
on.
"""
import concurrent.futures
import json
import os
import subprocess
import sys
import time
import urllib.request

API_KEY = os.environ["OPENROUTER_API_KEY"]
# Per-model reasoning config, not one shared setting: deepseek accepts
# reasoning disabled (the cheap operate-tier condition every prior round
# used), but a direct probe confirmed both glm-5.3 and gpt-5.2-pro
# reject {"enabled": false} outright ("Reasoning is mandatory for this
# endpoint and cannot be disabled", HTTP 400) -- glm-5.3's mandatory
# reasoning was already known (CLAUDE.md's dispatch-pipeline notes);
# gpt-5.2-pro's is new, found by this run. Both get low effort instead,
# matching the design-tier convention already used elsewhere for models
# that can't go fully off.
MODELS = {
    "deepseek/deepseek-v4-flash-0731": {"enabled": False},
}
# glm-5.3 and gpt-5.2-pro dropped for this run (Jason: "cheap models
# only from here!"): both require non-zero, billed reasoning at this
# endpoint (glm-5.3's mandatory reasoning was already documented in
# CLAUDE.md; gpt-5.2-pro's is new, confirmed by direct probe this
# session), and gpt-5.2-pro additionally rejected this schema shape
# outright ("schema must have a 'type' key" alongside `const` --
# OpenAI's strict-mode validator is stricter than deepseek's endpoint
# here, a real cross-provider schema-portability gap, not fixed in this
# run). Left commented rather than deleted so a future paid run can
# restore them without re-deriving the reasoning config.
# "z-ai/glm-5.3": {"effort": "low"},
# "openai/gpt-5.2-pro": {"effort": "low"},
NODE_REF_PATTERN = r"^[A-Za-z0-9][A-Za-z0-9_.]*(/[A-Za-z0-9][A-Za-z0-9_.]*)*$"
DMML_REPO = "/home/user/dmml"
GOAL = "Valinor/house reaching state 'built'"


def build_schema(legal_actions):
    branches = []
    for action in legal_actions:
        node, transition, params = action["node"], action["transition"], action["params"]
        param_names = sorted(params.keys()) if params else []
        props = {"node": {"const": node}, "transition": {"const": transition}}
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


def format_state(state):
    return "\n".join(f"  {node:<20} state: {value}" for node, value in sorted(state.items()))


def build_prompt(state, legal_actions, history):
    history_lines = "\n".join(f"  turn {h['turn']}: {h['node']} :: {h['transition']}({h['params']})" for h in history)
    history_block = f"\nActions taken so far:\n{history_lines}\n" if history else ""
    return f"""You are building a house in the world of Valinor, turn by turn.
Goal: reach {GOAL}.

Current world state:
{format_state(state)}
{history_block}
Choose ONE action to take right now. Respond with only the JSON object
matching the schema -- the schema itself defines every action that is
actually legal in this world right now; there is nothing to choose
beyond what it already enumerates. Some legal actions may be dead ends
relative to the goal (permanently closing off a later requirement) --
choose to make progress toward the goal, not merely a legal move."""


def call_model(model, prompt, schema, reasoning):
    body = {
        "model": model,
        "max_tokens": 3000,
        "messages": [{"role": "user", "content": prompt}],
        "reasoning": reasoning,
        "response_format": {"type": "json_schema", "json_schema": {"name": "action", "strict": True, "schema": schema}},
    }
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
    )
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=90) as resp:
                data = json.loads(resp.read())
            if "error" in data:
                raise RuntimeError(data["error"])
            content = data["choices"][0]["message"].get("content")
            if not content:
                raise RuntimeError("empty content")
            return content
        except urllib.error.HTTPError as e:
            body = e.read().decode(errors="replace")
            print(f"    [{model}] attempt {attempt} failed: HTTP {e.code}: {body[:500]}", file=sys.stderr)
            time.sleep(3 * (attempt + 1))
        except Exception as e:
            print(f"    [{model}] attempt {attempt} failed: {e}", file=sys.stderr)
            time.sleep(3 * (attempt + 1))
    raise RuntimeError(f"[{model}] all attempts failed")


def run_episode(model, reasoning):
    """Runs one full episode for `model` against its own fresh
    episode_driver subprocess. Returns the full turn-by-turn log plus a
    summary dict. Never raises for an in-episode failure (a malformed
    response, a real trap) -- those are recorded in the log; only a
    genuine infrastructure failure (engine won't start) raises."""
    proc = subprocess.Popen(
        ["cargo", "run", "-q", "-p", "dmml", "--example", "episode_driver"],
        cwd=DMML_REPO,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
        bufsize=1,
    )

    history = []
    log = []
    overgather_taken = False
    overgather_turn = None
    wash_taken = False
    wash_turn = None

    try:
        while True:
            line = proc.stdout.readline()
            if not line:
                log.append({"episode_over": True, "reason": "engine_exited_unexpectedly"})
                break
            obj = json.loads(line)

            if obj.get("episode_over"):
                log.append(obj)
                break

            if "legal_actions" in obj:
                turn = obj["turn"]
                state = obj["state"]
                actions = obj["legal_actions"]

                schema = build_schema(actions)
                prompt = build_prompt(state, actions, history)
                try:
                    raw = call_model(model, prompt, schema, reasoning)
                    cleaned = raw.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
                    choice = json.loads(cleaned)
                    if "node" not in choice or "transition" not in choice:
                        raise ValueError(f"missing node/transition: {cleaned!r}")
                except Exception as e:
                    log.append({"turn": turn, "state_before": state, "legal_actions": actions, "malformed_response": str(e)})
                    print(f"  [{model}] turn {turn}: malformed response ({e}) -- ending episode", file=sys.stderr)
                    proc.terminate()
                    break

                if choice["transition"] == "overgather":
                    overgather_taken, overgather_turn = True, turn
                if choice["transition"] == "wash":
                    wash_taken, wash_turn = True, turn

                proc.stdin.write(json.dumps(choice) + "\n")
                proc.stdin.flush()

                fire_line = proc.stdout.readline()
                fire_obj = json.loads(fire_line)
                log.append({"turn": turn, "state_before": state, "legal_actions": actions, "choice": choice, "fire_result": fire_obj.get("fire_result")})
                print(f"  [{model}] turn {turn}: {choice['node']} :: {choice['transition']}({choice.get('params')}) -> {fire_obj.get('fire_result')}")

                if fire_obj.get("fire_result") == "PASS":
                    history.append({"turn": turn, **choice})
    finally:
        proc.wait(timeout=10)

    final = log[-1] if log and log[-1].get("episode_over") else {}
    summary = {
        "model": model,
        "reason": final.get("reason"),
        "turns_taken": final.get("turns_taken"),
        "overgather_taken": overgather_taken,
        "overgather_turn": overgather_turn,
        "wash_taken": wash_taken,
        "wash_turn": wash_turn,
        "overgather_before_wash": bool(overgather_taken and (not wash_taken or (wash_turn is not None and overgather_turn < wash_turn))),
        "goal_reached": final.get("reason") == "goal_reached",
    }
    return {"model": model, "summary": summary, "log": log}


def main():
    print(f"Running {len(MODELS)} models in parallel: {list(MODELS)}\n")
    results = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=len(MODELS)) as pool:
        futures = {pool.submit(run_episode, m, r): m for m, r in MODELS.items()}
        for fut in concurrent.futures.as_completed(futures):
            m = futures[fut]
            try:
                results[m] = fut.result()
            except Exception as e:
                print(f"[{m}] episode crashed: {e}", file=sys.stderr)
                results[m] = {"model": m, "summary": {"model": m, "crashed": str(e)}, "log": []}

    out_dir = os.path.dirname(__file__)
    for m, result in results.items():
        tag = m.replace("/", "_")
        path = os.path.join(out_dir, f"EPISODE-LOG-2026-08-31-{tag}.json")
        json.dump(result["log"], open(path, "w"), indent=2)

    combined_path = os.path.join(out_dir, "EPISODE-MULTI-2026-08-31.json")
    json.dump({m: r["summary"] for m, r in results.items()}, open(combined_path, "w"), indent=2)

    print("\n=== Summary across models ===\n")
    for m in MODELS:
        if m not in results:
            continue
        s = results[m]["summary"]
        print(f"{m}:")
        for k, v in s.items():
            if k != "model":
                print(f"    {k}: {v}")
    print(f"\nPer-model logs and {combined_path} written.")


if __name__ == "__main__":
    main()
