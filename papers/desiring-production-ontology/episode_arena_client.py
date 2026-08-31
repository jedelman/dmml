#!/usr/bin/env python3
"""Round 9's client half: connects N cheap models to `episode_arena.rs`
concurrently, each in its own unbounded loop, no turn-taking, no fixed
order. Jason's framing after Round 8's first real goal-failure:
"models can make mistakes in the world! that's okay! it makes the
world interesting! it's when they can't form commits that we lose
their contributions!" -- so this client does NOT gate a model's
proposal against the offered legal_actions before submitting, unlike
`episode_swarm.py`'s `is_schema_conformant`. Whatever a model proposes
gets sent straight to the arena server, which is the real arbiter
(`dmml::machine::commit_fires_transition`, same as every prior round).
A structurally-legal-but-bad move stays in the world as real content;
only a response that can't even be parsed into an action at all is
logged as a lost contribution and skipped.

"Parallel race - new commits get broadcast" (Jason's own phrasing,
picked over round-robin and over all-propose-one-fires): every model
runs its own asyncio loop -- query current state, dispatch for one
proposed action, submit it, repeat -- with no synchronization between
loops at all. The server's mutex around "check the guard, then apply"
(see `episode_arena.rs`'s own doc comment) is the actual race
resolution: whichever proposal's commit attempt acquires that lock
first, while its guard still holds against the live world, wins;
everyone else just sees the world has moved by the time they check
again, exactly like a real concurrent system, not a turn-based
simulation of one.

Runs for a fixed wall-clock duration, not a fixed step count -- this is
open-ended by design (no goal is stated in the prompt), so bounding by
turns doesn't mean anything here the way it did in the earlier,
goal-directed rounds.

Extended 2026-08-31, Round 14: Round 13 triangulated the schema-
conformance collapse down to state changing for reasons the querying
agent didn't cause and couldn't predict (not concurrency, not mere
evolution over time -- both ruled out separately). The mitigation named
there, "give it slightly more, but structured, not narrated," is now
real: `episode_arena.rs` tracks each actor's own last-seen snapshot
server-side and returns `changed_since_you_last_looked` on every query,
computed via the actual `dmml::interpret::diverges` primitive (already
proven in `drift_machine.rs`), not a bare fresh snapshot and not a
prose warning that the world might have moved. This client now sends
`actor` on every query (previously only `Act` requests carried it) and
renders that drift block into the prompt, compactly, as data -- a list
of `subject: before -> after` lines, never narrated into a sentence.
Round 14 result: it didn't help (79% could-not-form-commit, worse than
baseline).

Extended again, Round 15: Jason -- "maybe a mutex is the correct
primitive - they really have to take turns?" A real mutex
(`asyncio.Lock`) now wraps each agent's ENTIRE query-decide-act cycle,
not just the arena's own commit step (which was already
mutex-protected, but only for the instant of firing). While one agent
holds the lock, no other agent can query or act -- the world cannot
move between when an agent looks and when its action lands, by
construction, the same guarantee Rounds 6/8's single-agent runs had
for free (nothing else was ever acting there) and Round 9's "parallel
race" deliberately gave up.
"""
import asyncio
import json
import os
import socket
import subprocess
import sys
import time
import urllib.request
import urllib.error

API_KEY = os.environ["OPENROUTER_API_KEY"]
ARENA_PORT = 7880
DURATION_SECONDS = 90
DMML_REPO = "/home/user/dmml"
NODE_REF_PATTERN = r"^[A-Za-z0-9][A-Za-z0-9_.]*(/[A-Za-z0-9][A-Za-z0-9_.]*)*$"

# Same cheap roster as Round 8's swarm, same reasoning configs (probed
# live that round, not re-derived here).
MODELS = {
    "deepseek/deepseek-v4-flash-0731": {"enabled": False},
    "z-ai/glm-4.7-flash": {"enabled": False},
    "google/gemini-2.5-flash-lite": {"enabled": False},
    "google/gemini-3.1-flash-lite": {"enabled": False},
}


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


def build_prompt(model, state, legal_actions, drift):
    # Round 10 revision (2026-08-31): Round 9's prompt was negation-heavy
    # ("there is no single goal", "may already be stale", "not a failure
    # to avoid", "may no longer be legal", "not an error"). Jason's
    # direct hypothesis on seeing the 69% could-not-form-commit result:
    # "I think the 'no' is what's throwing them off! positivity only for
    # these lil guys." Rewritten to state everything affirmatively, with
    # nothing about staleness/failure/error mentioned at all -- the
    # schema and the engine already handle a stale or bad pick correctly
    # on their own, so the prompt doesn't need to warn about it.
    #
    # Round 14 addition: `drift` is real, computed structured data (see
    # module docstring) rendered as plain subject/before/after lines --
    # never folded into a narrated sentence, per the same "structure,
    # not prose" discipline this whole project applies everywhere else.
    return f"""You are one of several agents ({model}) building and shaping the
living world of Valinor together, all acting at the same time. Explore
freely and take whichever action feels most worthwhile or interesting
right now -- anything that grows, builds, or develops the world further
is a good choice.

{format_drift(drift)}Current world state:
{format_state(state)}

Choose ONE action, and respond with only the JSON object matching the
schema -- it lists every action that's live and available to you right
now."""


def call_model(model, prompt, schema, reasoning):
    body = {
        "model": model,
        "max_tokens": 1500,
        "messages": [{"role": "user", "content": prompt}],
        "reasoning": reasoning,
        "response_format": {"type": "json_schema", "json_schema": {"name": "action", "strict": True, "schema": schema}},
    }
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read())
        if "error" in data:
            return None, str(data["error"])
        content = data["choices"][0]["message"].get("content")
        if not content:
            return None, "empty content"
        return content, None
    except urllib.error.HTTPError as e:
        return None, f"HTTP {e.code}: {e.read().decode(errors='replace')[:300]}"
    except Exception as e:
        return None, str(e)


def arena_call(req):
    s = socket.create_connection(("127.0.0.1", ARENA_PORT), timeout=10)
    s.sendall((json.dumps(req) + "\n").encode())
    resp = s.makefile().readline()
    s.close()
    return json.loads(resp)


async def agent_loop(model, reasoning, stop_event, log, loop, turn_lock):
    attempts = 0
    while not stop_event.is_set():
        # Round 15: a real mutex held across the WHOLE query-decide-act
        # cycle, not just around the commit step (which was already
        # mutex-protected, but only for the instant of firing). While
        # this agent holds the lock, no other agent can query or act --
        # the world genuinely cannot move between when this agent looks
        # and when its action lands, by construction, not by asking it
        # to trust a snapshot.
        async with turn_lock:
            attempts += 1
            query = await loop.run_in_executor(None, arena_call, {"query": True, "actor": model})
            if "state" not in query:
                print(f"  [{model}] ARENA PROTOCOL ERROR on query: {query!r}", file=sys.stderr)
                continue
            state, legal_actions, drift = query["state"], query["legal_actions"], query.get("changed_since_you_last_looked", [])

            schema = build_schema(legal_actions)
            prompt = build_prompt(model, state, legal_actions, drift)
            raw, err = await loop.run_in_executor(None, call_model, model, prompt, schema, reasoning)

            if err:
                entry = {"t": time.time(), "actor": model, "outcome": "dispatch_error", "detail": err}
                log.append(entry)
                print(f"  [{model}] dispatch error: {err}", file=sys.stderr)
                continue

            cleaned = raw.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
            try:
                choice = json.loads(cleaned)
                node, transition = choice["node"], choice["transition"]
            except Exception as e:
                entry = {"t": time.time(), "actor": model, "outcome": "could_not_form_commit", "raw": raw, "detail": str(e)}
                log.append(entry)
                print(f"  [{model}] COULD NOT FORM A COMMIT: {raw!r} ({e})", file=sys.stderr)
                continue

            params = choice.get("params") or {}
            result = await loop.run_in_executor(None, arena_call, {"actor": model, "node": node, "transition": transition, "params": params})
            if "fire_result" not in result:
                entry = {"t": time.time(), "actor": model, "outcome": "arena_protocol_error", "sent": {"node": node, "transition": transition, "params": params}, "got": result}
                log.append(entry)
                print(f"  [{model}] ARENA PROTOCOL ERROR, sent={{'node':{node!r},'transition':{transition!r},'params':{params!r}}} got={result!r}", file=sys.stderr)
                continue
            entry = {
                "t": time.time(), "actor": model, "outcome": "submitted",
                "node": node, "transition": transition, "params": params,
                "fire_result": result["fire_result"], "commit_index": result["commit_index"],
            }
            log.append(entry)
            won = result["fire_result"] == "PASS"
            print(f"  [{model}] {node} :: {transition}({params}) -> {result['fire_result']}{'  <-- landed' if won else ''}")

    return attempts


async def main():
    print(f"Starting episode_arena server on port {ARENA_PORT}...")
    server = subprocess.Popen(
        ["cargo", "run", "-q", "-p", "dmml", "--example", "episode_arena", "--", str(ARENA_PORT)],
        cwd=DMML_REPO,
    )
    for _ in range(30):
        try:
            socket.create_connection(("127.0.0.1", ARENA_PORT), timeout=1).close()
            break
        except OSError:
            await asyncio.sleep(1)
    else:
        raise RuntimeError("episode_arena server never came up")

    print(f"Running {len(MODELS)} agents, taking real turns (mutex-serialized), for {DURATION_SECONDS}s: {list(MODELS)}\n")
    loop = asyncio.get_event_loop()
    stop_event = asyncio.Event()
    turn_lock = asyncio.Lock()
    log = []

    tasks = [asyncio.create_task(agent_loop(m, r, stop_event, log, loop, turn_lock)) for m, r in MODELS.items()]

    async def timer():
        await asyncio.sleep(DURATION_SECONDS)
        stop_event.set()

    await asyncio.gather(timer(), *tasks)

    final = await loop.run_in_executor(None, arena_call, {"query": True, "actor": "__final_summary__"})
    server.terminate()
    server.wait(timeout=10)

    out_dir = os.path.dirname(__file__)
    log_path = os.path.join(out_dir, "EPISODE-ARENA-2026-08-31-round15-mutex-turns.json")
    json.dump(log, open(log_path, "w"), indent=2)

    landed = [e for e in log if e.get("fire_result") == "PASS"]
    by_actor = {}
    for e in log:
        by_actor.setdefault(e["actor"], {"attempts": 0, "landed": 0, "could_not_form_commit": 0, "dispatch_error": 0})
        by_actor[e["actor"]]["attempts"] += 1
        if e.get("fire_result") == "PASS":
            by_actor[e["actor"]]["landed"] += 1
        if e["outcome"] == "could_not_form_commit":
            by_actor[e["actor"]]["could_not_form_commit"] += 1
        if e["outcome"] == "dispatch_error":
            by_actor[e["actor"]]["dispatch_error"] += 1

    print(f"\n=== Arena summary ({final['history_len']} total commits in history) ===\n")
    for actor, stats in by_actor.items():
        print(f"{actor}: {stats}")
    print(f"\nFinal state:\n{format_state(final['state'])}")
    print(f"\nFull log written to {log_path}")


if __name__ == "__main__":
    asyncio.run(main())
