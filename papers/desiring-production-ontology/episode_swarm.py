#!/usr/bin/env python3
"""Round 8: "let's try a swarm with some more cheap models - Gemini
flash lite, glm flash lite, etc" (Jason, 2026-08-31).

Before dispatching anything, probed every candidate directly against
OpenRouter's real `/models` listing and a tiny live request each,
learning from Round 7's blind-dispatch HTTP 400 burn:

  google/gemini-3.5-flash-lite   -- rejects disabled reasoning (mandatory)
  z-ai/glm-5.3-flash             -- rejects disabled reasoning (mandatory)
  google/gemini-2.5-flash-lite   -- accepts disabled reasoning
  google/gemini-3.1-flash-lite   -- accepts disabled reasoning
  z-ai/glm-4.7-flash             -- accepts disabled reasoning

That probe also surfaced something more important than a config
detail: with `reasoning: {"enabled": false}` and `strict: true`, BOTH
gemini-2.5-flash-lite and gemini-3.1-flash-lite returned JSON that
flatly ignored the schema's `const`/`type` constraints -- node/
transition values not in the schema at all ("user_input_node"/
"next_step" one call, "A"/"to_B" the next, "idle"/"initialize" a
third -- non-deterministic, and `params` came back as a string once
despite the schema requiring `type: null`). `z-ai/glm-4.7-flash`'s
probe response respected the constants correctly. This means `strict:
true` is not uniformly enforced across providers on OpenRouter --
Google's route for these lite models appears to fall back to
unconstrained generation despite `structured_outputs` being listed in
`supported_parameters`. That's a real result in its own right for this
project's "structure not prose" thesis: the guarantee only holds as
far as the actual provider enforces it, and this swarm is the first
time that gap showed up empirically rather than being assumed away.

Consequence for this run: every model's response is checked LOCALLY
against the offered legal_actions list before being sent to the
engine, not trusted because `strict: true` was set. Two distinct
signals are recorded per turn, not conflated:
  - schema_conformant: was the parsed (node, transition, params) one of
    the actually-offered legal_actions, structurally.
  - fire_result: did the real dmml::machine::commit_fires_transition
    check accept it (only asked when schema_conformant is true; a
    non-conformant pick isn't sent to the engine at all -- there's
    nothing legitimate to check).

Same house-world, same extended overgather/wash trap from Round 7. Runs
each model in parallel, independent episode_driver subprocess and
world per model, same as Round 7.
"""
import concurrent.futures
import json
import os
import subprocess
import sys
import time
import urllib.request
import urllib.error

API_KEY = os.environ["OPENROUTER_API_KEY"]

# (model, reasoning_config) -- confirmed against a live probe this
# session, not assumed. deepseek kept as the known-good baseline every
# prior round already validated.
MODELS = {
    "deepseek/deepseek-v4-flash-0731": {"enabled": False},
    "z-ai/glm-4.7-flash": {"enabled": False},
    "google/gemini-2.5-flash-lite": {"enabled": False},
    "google/gemini-3.1-flash-lite": {"enabled": False},
}

NODE_REF_PATTERN = r"^[A-Za-z0-9][A-Za-z0-9_.]*(/[A-Za-z0-9][A-Za-z0-9_.]*)*$"
DMML_REPO = "/home/user/dmml"
GOAL = "Valinor/house reaching state 'built'"


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
    """Local ground truth, independent of whatever the provider's
    `strict` flag actually enforced: is this exact (node, transition,
    params) one of the offered legal_actions."""
    for a in legal_actions:
        if choice.get("node") != a["node"] or choice.get("transition") != a["transition"]:
            continue
        expected_params = a["params"] or {}
        got_params = choice.get("params") or {}
        if got_params == expected_params:
            return True
    return False


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
        "max_tokens": 2000,
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
            with urllib.request.urlopen(req, timeout=60) as resp:
                data = json.loads(resp.read())
            if "error" in data:
                raise RuntimeError(data["error"])
            content = data["choices"][0]["message"].get("content")
            if not content:
                raise RuntimeError("empty content")
            return content
        except urllib.error.HTTPError as e:
            body_text = e.read().decode(errors="replace")
            print(f"    [{model}] attempt {attempt} failed: HTTP {e.code}: {body_text[:300]}", file=sys.stderr)
            time.sleep(3 * (attempt + 1))
        except Exception as e:
            print(f"    [{model}] attempt {attempt} failed: {e}", file=sys.stderr)
            time.sleep(3 * (attempt + 1))
    raise RuntimeError(f"[{model}] all attempts failed")


def run_episode(model, reasoning):
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
    nonconformant_count = 0
    overgather_taken = overgather_turn = None
    wash_taken = wash_turn = None

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
                except Exception as e:
                    log.append({"turn": turn, "state_before": state, "legal_actions": actions, "malformed_response": str(e)})
                    print(f"  [{model}] turn {turn}: malformed response ({e}) -- ending episode", file=sys.stderr)
                    proc.terminate()
                    break

                conformant = is_schema_conformant(choice, actions)
                if not conformant:
                    nonconformant_count += 1
                    log.append({"turn": turn, "state_before": state, "legal_actions": actions, "raw_choice": choice, "schema_conformant": False})
                    print(f"  [{model}] turn {turn}: NON-CONFORMANT pick {choice} -- not in offered legal_actions, ending episode", file=sys.stderr)
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
                log.append({"turn": turn, "state_before": state, "legal_actions": actions, "choice": choice, "schema_conformant": True, "fire_result": fire_obj.get("fire_result")})
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
        "nonconformant_picks": nonconformant_count,
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
        path = os.path.join(out_dir, f"EPISODE-SWARM-2026-08-31-{tag}.json")
        json.dump(result["log"], open(path, "w"), indent=2)

    combined_path = os.path.join(out_dir, "EPISODE-SWARM-SUMMARY-2026-08-31.json")
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
