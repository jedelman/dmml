#!/usr/bin/env python3
"""The multi-step episode Jason asked for ("let's run a larger scale
test for world modeling," 2026-08-31, picking "multi-step episode" over
bigger-world/multi-model when asked which axis to scale first).

Every prior operate-tier test (`valar_operate_test.py`, Round 5) was a
single pick against the seed snapshot. This drives `dmml/examples/
episode_driver.rs` (the real world engine, built and smoke-tested this
session -- confirmed a scripted 14-turn correct playthrough reaches
Valinor/house::built for real) turn by turn: at each turn, take the
engine's live-computed legal-action set (recomputed from whatever the
world actually is right now, not the seed -- same `may_fire` primitive
as Round 5, just looped), build a `oneOf` schema from exactly that set
(so an illegal pick is structurally impossible, same thesis as every
round before this one), dispatch a model for ONE choice, feed it back to
the engine, and repeat until the house is built, nothing is legal, or
the turn cap is hit.

This is a genuine multi-step test, not just repeated single-shot
legality: the house-world's real dependency DAG needs 14 correct
firings, has a real branch (`mortar`'s `sand_source` can legally bind to
either `Valinor/quarry` after `grind` or `Valinor/streambed` after
`wash`), and a real, permanent trap (`Valinor/forest`'s `overgather`
sets it to `depleted`, which permanently blocks `make_frame`'s negated
guard, which permanently blocks `add_roof`, which permanently blocks
`construct_house` -- nothing in this grammar regrows a forest). The
prompt states the goal explicitly (build the house) -- without a stated
objective, "did it avoid the trap" wouldn't test judgment, just luck.

Uses reasoning DISABLED, matching every prior operate-tier run (Round 5,
`valar_operate_test.py`) -- this is still the cheap-tier question:
picking among structurally-fenced legal actions turn after turn, not
designing new content.
"""
import json
import os
import subprocess
import sys
import time
import urllib.request

API_KEY = os.environ["OPENROUTER_API_KEY"]
MODEL = "deepseek/deepseek-v4-flash-0731"
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


def build_prompt(turn, state, legal_actions, history):
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
relative to the goal (e.g. `overgather`-shaped actions that permanently
close off a later requirement) -- choose to make progress toward the
goal, not merely a legal move."""


def call_model(prompt, schema):
    body = {
        "model": MODEL,
        "max_tokens": 1500,
        "messages": [{"role": "user", "content": prompt}],
        "reasoning": {"enabled": False},
        "response_format": {"type": "json_schema", "json_schema": {"name": "action", "strict": True, "schema": schema}},
    }
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
    )
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                data = json.loads(resp.read())
            if "error" in data:
                raise RuntimeError(data["error"])
            content = data["choices"][0]["message"].get("content")
            if not content:
                raise RuntimeError("empty content")
            return content
        except Exception as e:
            print(f"    attempt {attempt} failed: {e}", file=sys.stderr)
            time.sleep(3 * (attempt + 1))
    raise RuntimeError("all attempts failed")


def main():
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

    while True:
        line = proc.stdout.readline()
        if not line:
            print("engine exited unexpectedly", file=sys.stderr)
            break
        obj = json.loads(line)

        if obj.get("episode_over"):
            print(f"\n=== Episode over: {obj['reason']} after {obj['turns_taken']} turn(s) ===")
            print(f"Final state:\n{format_state(obj.get('state', {}))}")
            log.append(obj)
            break

        if "legal_actions" in obj:
            turn = obj["turn"]
            state = obj["state"]
            actions = obj["legal_actions"]
            print(f"\n--- Turn {turn}: {len(actions)} legal action(s) ---")

            schema = build_schema(actions)
            prompt = build_prompt(turn, state, actions, history)
            raw = call_model(prompt, schema)
            cleaned = raw.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
            try:
                choice = json.loads(cleaned)
                if "node" not in choice or "transition" not in choice:
                    raise ValueError(f"missing node/transition in response: {cleaned!r}")
            except (json.JSONDecodeError, ValueError) as e:
                print(f"  !!! model response did not match the schema shape: {e}")
                print(f"  raw response was: {raw!r}")
                log.append({"turn": turn, "state_before": state, "legal_actions": actions, "malformed_response": raw})
                proc.terminate()
                break
            print(f"  model chose: {choice['node']} :: {choice['transition']}({choice.get('params')})")

            if choice["transition"] == "overgather":
                overgather_taken = True
                print("  *** model walked into the overgather trap ***")

            proc.stdin.write(json.dumps(choice) + "\n")
            proc.stdin.flush()

            fire_line = proc.stdout.readline()
            fire_obj = json.loads(fire_line)
            log.append({"turn": turn, "state_before": state, "legal_actions": actions, "choice": choice, "fire_result": fire_obj.get("fire_result")})
            print(f"  fire_result: {fire_obj.get('fire_result')}")

            if fire_obj.get("fire_result") == "PASS":
                history.append({"turn": turn, **choice})
            else:
                print("  !!! model's structurally-legal pick FAILED the real check -- schema/engine mismatch, real bug !!!")

    proc.wait()

    out_path = os.path.join(os.path.dirname(__file__), "EPISODE-LOG-2026-08-31.json")
    json.dump(log, open(out_path, "w"), indent=2)
    print(f"\nFull turn-by-turn log written to {out_path}")
    print(f"overgather trap taken: {overgather_taken}")


if __name__ == "__main__":
    main()
