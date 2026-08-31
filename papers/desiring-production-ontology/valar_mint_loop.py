#!/usr/bin/env python3
"""Same Vala design task as valar_mint.py, but with a real feedback loop
instead of one single-shot call. Jason's hypothesis, 2026-08-30: "I'm
wondering if an agentic loop -- sandboxed filesystem, with multiple tool
calls for fixes and refinement -- is required for this task. a single
typo can sink it." Four straight single-shot failures (deepseek x3,
glm-5.3 x1) support exactly that reading -- gpt-5.2-pro needed a human
to apply two one-line fixes by hand, which is itself evidence a loop
would have closed the gap on its own.

This is the actual test: same model (deepseek/deepseek-v4-flash-0731,
low reasoning effort, matching the last real comparison point), same
prompt, but on each turn the REAL validator (`cargo run -p dmml
--example validate_machines`) is run against whatever the model just
produced, and if it fails, the model gets the real error message back
and one more turn to fix it -- up to MAX_ITERATIONS times. No hand
correction anywhere in this loop; if it converges, it converges on its
own.
"""
import json
import os
import subprocess
import sys
import time
import urllib.request

API_KEY = os.environ["OPENROUTER_API_KEY"]
MODEL = "deepseek/deepseek-v4-flash-0731"
REASONING_EFFORT = "low"
MAX_ITERATIONS = 5
SCHEMA_PATH = os.path.join(os.path.dirname(__file__), "real_schema.json")

WORLD_SO_FAR = """The world (Valinor) as it stands, machine by machine:

- Valinor (terrain): unformed -> hills -> mountains. `raise`, `uplift`.
- Valinor/quarry (material differentiation): untouched -> stone -> sand ->
  clay -> brick. `quarry` (gated: Valinor itself must be mountains),
  `grind`, `wet`, `fire`.
- Valinor/streambed: bare -> sand. `wash`.
- Valinor/spring: dry -> flowing. `well_up`.
- Valinor/mortar: unmixed -> mixed. `mix($sand_source, $water_source)`,
  gated on BOTH cited sources (sand AND flowing) -- a real two-input join.
- Valinor/wall: unbuilt -> built. `build($brick_source, $mortar_source)`,
  the same two-input shape one level up (brick AND mixed mortar).
- Valinor/forest: full -> thinned -> depleted. `gather`, `overgather`.
- Valinor/carpentry: no_frame -> framed. `make_frame`, gated by a
  NEGATED guard (Valinor/forest must NOT be depleted -- accepts full OR
  thinned, the only clean way to express that without an OR in the
  guard grammar).
- Valinor/roof: unroofed -> roofed. `add_roof($wall_source,
  $frame_source)`, a third two-input join (wall built AND frame made) --
  the capstone, a roofed house.

Every guard checks REAL prior state, never narration. Every transition's
"verb" IS its ident -- an agent operating this world doesn't invent a
predicate, it fires a transition. `refs`/`consumes`/params name the
specific things a transition acts on, never a free-floating assertion."""

VALA_PROMPT = f"""You are one of the Valar -- not a player-agent operating turn by turn
inside this world, but one of the shaping powers who gives the world new
machinery for others to later work within. Your medium is DMML machines:
real states, transitions, guards, and effects, checked by
dmml::machine::commit_fires_transition exactly like everything already
built. You are not narrating an event or minting a fact -- you are
designing a MECHANISM: something with real preconditions (guards against
live world state) and real consequences (a state change), that other,
simpler agents will later be bounded by, the same way an agent can only
say "raise" or "quarry," never invent a verb from nothing.

{WORLD_SO_FAR}

Your task: propose ONE new machine (or, if it genuinely needs a
companion machine to make sense -- e.g. a resource it consumes that
doesn't yet exist -- two, but no more) that this world is still missing.
Use your own judgment about what kind of production, constraint, or
transformation would be interesting here -- something that extends the
existing chain (consumes from it, gates on it, or opens a new resource
line entirely), not a decorative reskin of something that already
exists. Favor real consumption over narration: a guard checking live
state, ideally against ANOTHER node's state (like quarry's mountain
check) or against a specific cited target ($param, like mortar's two
inputs), not just your own machine's own prior state in isolation.

DMML machine grammar, exactly:
- A machine: {{"node": "<node_ref>", "states": [{{"ident": "<ident>"}}, ...],
  "transitions": [...]}}
- A transition: {{"ident": "<ident>", "params": ["<ident>", ...] (optional),
  "from": "<state ident>" (optional), "to": "<state ident>" (optional),
  "guards": [...] (optional), "effects": [...] (optional)}} -- must have
  at least one of: a guard, a from+to pair, or an effect.
- A guard: {{"negated": true|false (optional, default false), "exists": {{
  "anchor": <PatternTerm>, "hops": [{{"predicate": "<ident>", "term":
  <PatternTerm>}}, ...]}}}} -- at least one hop required.
- A PatternTerm: {{"kind": "self"}} | {{"kind": "param", "value": "<ident>"}}
  | {{"kind": "var", "value": "<ident>"}} | {{"kind": "node", "value":
  "<node_ref>"}}. `self` means the machine's own node; `param` means a
  value the firing commit bound; `var` is a free existential (matches
  anything); `node` is a fixed, specific node name you write literally.
- An explicit effect (rarely needed -- from/to sugar covers most cases):
  {{"kind": "assert"|"retract", "ident": "<state ident>"}}.
- Every ident: letters/digits/underscore only, NEVER a hyphen. node_ref:
  slash-separated segments of the same.

CRITICAL: whatever precondition you reason your way to -- "this should
require the wall and roof," "this should need flowing water," anything
of that shape -- MUST become an actual "guards" entry in the JSON
transition, not just something you considered. A transition with no
guards, no from+to pair, and no effects is REJECTED outright by the
validator (`has_content` check) -- and a transition with a from+to but
no guard means anyone can fire it with zero preconditions, which is
almost never what your own reasoning will have concluded you want. If
you found yourself thinking "this needs X to be true first," that
thought is not finished until it is a guard clause in the JSON.

ONE MORE THING, stated explicitly because a prior Vala guessed wrong on
exactly this: every guard that checks a machine's own state uses the
predicate "state" -- NEVER "a" and NEVER "rdf:type". "a" is reserved for
ordinary fact-authoring's rdf:type shorthand (asserting what KIND of
thing a node is), a completely different concern from a machine's
current STATE. A guard hop checking "is $target currently in state
'brick'" is {{"predicate": "state", "term": {{"kind": "node", "value":
"brick"}}}} -- never {{"predicate": "a", ...}}.

Respond with ONLY the raw JSON object matching the schema below. Leave
"commits" as an empty array -- you are minting machinery, not facts."""


def call_model(messages, schema):
    body = {
        "model": MODEL,
        "max_tokens": 24000,
        "messages": messages,
        "reasoning": {"effort": REASONING_EFFORT},
        "response_format": {
            "type": "json_schema",
            "json_schema": {"name": "update", "strict": False, "schema": schema},
        },
        "include_reasoning": True,
    }
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
    )
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                data = json.loads(resp.read())
            if "error" in data:
                raise RuntimeError(data["error"])
            msg = data["choices"][0]["message"]
            content = msg.get("content")
            reasoning = msg.get("reasoning_content") or msg.get("reasoning")
            if not content:
                rlen = len(reasoning) if reasoning else 0
                raise RuntimeError(f"empty content (reasoning present: {bool(reasoning)}, reasoning length: {rlen} chars)")
            return content, reasoning
        except Exception as e:
            print(f"  attempt {attempt} failed: {e}", file=sys.stderr)
            time.sleep(6 * (attempt + 1))
    raise RuntimeError("all attempts failed")


def validate(json_text):
    """Runs the REAL validator, returns (is_valid, output_text)."""
    tmp_path = os.path.join(os.path.dirname(__file__), "_loop_candidate.json")
    with open(tmp_path, "w") as f:
        f.write(json_text)
    result = subprocess.run(
        ["cargo", "run", "-p", "dmml", "--example", "validate_machines", "--", tmp_path],
        cwd="/home/user/dmml",
        capture_output=True,
        text=True,
    )
    combined = result.stdout + result.stderr
    return result.returncode == 0, combined


def main():
    schema = json.load(open(SCHEMA_PATH))
    messages = [{"role": "user", "content": VALA_PROMPT}]

    for iteration in range(1, MAX_ITERATIONS + 1):
        print(f"\n{'=' * 20} ITERATION {iteration} {'=' * 20}\n", file=sys.stderr)
        content, reasoning = call_model(messages, schema)

        if reasoning:
            print(f"[reasoning, {len(reasoning)} chars, first 800:]", file=sys.stderr)
            print(reasoning[:800], file=sys.stderr)

        cleaned = content.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
        print(f"\n[iteration {iteration} candidate JSON]")
        print(cleaned)

        is_valid, validator_output = validate(cleaned)
        print(f"\n[validator output]\n{validator_output}")

        if is_valid:
            out_path = os.path.join(os.path.dirname(__file__), "VALAR-LOOP-2026-08-30-SUCCESS.json")
            with open(out_path, "w") as f:
                f.write(cleaned)
            print(f"\n*** CONVERGED after {iteration} iteration(s). Wrote {out_path} ***")
            return

        messages.append({"role": "assistant", "content": cleaned})
        messages.append({
            "role": "user",
            "content": (
                f"That failed validation with this exact error:\n\n{validator_output}\n\n"
                "Fix it and resubmit the COMPLETE corrected JSON object (not a diff, "
                "not just the fixed part -- the entire update object again, corrected)."
            ),
        })

    print(f"\n*** DID NOT CONVERGE after {MAX_ITERATIONS} iterations. ***")


if __name__ == "__main__":
    main()
