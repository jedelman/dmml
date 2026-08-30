#!/usr/bin/env python3
"""A Vala mints a new machine for Valinor.

Hoists the process (Jason, 2026-08-30): every machine so far
(valinor.rs/door.rs/quarry.rs/wall.rs/house.rs) was hand-designed by
Claude, then hand-validated against dmml::machine::commit_fires_
transition. This is the other tier -- a distinct agent, reasoning left
ON (the opposite of the fast/cheap acting-agent harness earlier this
session, which explicitly disabled it), given a genuinely different
prompt: not "operate within the world," but "shape it." A Vala isn't
asked to play a turn; it's asked to design real machinery other agents
will later be bounded by, exercising its own creative judgment about
what this world still needs -- while still being held to the same real
DMML grammar as everything else. Output is validated for real via
`cargo run -p dmml --example validate_machines`, not trusted on its own
say-so.

Uses reasoning left enabled (no `reasoning: {"enabled": false}` in the
request) and real structured output against the full UpdateInput schema
(real_schema.json, already generated from dmml::schema this session) --
the Vala is free to leave "commits" empty and mint only "machines".
"""
import json
import os
import subprocess
import sys
import time
import urllib.request

API_KEY = os.environ["OPENROUTER_API_KEY"]
MODEL = "deepseek/deepseek-v4-flash-0731"
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

Respond with ONLY the raw JSON object matching the schema below. Leave
"commits" as an empty array -- you are minting machinery, not facts.
No prose, no markdown fences, no explanation outside the JSON itself."""


def call_model(prompt, schema):
    body = {
        "model": MODEL,
        "max_tokens": 3000,
        "messages": [{"role": "user", "content": prompt}],
        "response_format": {
            "type": "json_schema",
            "json_schema": {"name": "update", "strict": False, "schema": schema},
        },
        # Deliberately NOT setting reasoning: {"enabled": false} here --
        # this is the one call in this session's whole harness lineage
        # meant to think, not react.
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
                raise RuntimeError(f"empty content (reasoning present: {bool(reasoning)})")
            return content, reasoning
        except Exception as e:
            print(f"  attempt {attempt} failed: {e}", file=sys.stderr)
            time.sleep(6 * (attempt + 1))
    raise RuntimeError("all attempts failed")


def main():
    schema = json.load(open(SCHEMA_PATH))
    print("Dispatching to the Vala (reasoning enabled)...\n", file=sys.stderr)
    content, reasoning = call_model(VALA_PROMPT, schema)

    if reasoning:
        print("=== Reasoning trace ===")
        print(reasoning[:4000])
        print("=== (truncated if longer) ===\n")

    cleaned = content.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
    out_path = os.path.join(os.path.dirname(__file__), "VALAR-MINTED-2026-08-30.json")
    with open(out_path, "w") as f:
        f.write(cleaned)
    print(f"Wrote {out_path}\n")

    try:
        parsed = json.loads(cleaned)
        print(json.dumps(parsed, indent=2))
    except json.JSONDecodeError as e:
        print(f"WARNING: not even valid JSON: {e}")
        return

    print("\n=== Validating against dmml::from_json (real check, not trust) ===\n")
    result = subprocess.run(
        ["cargo", "run", "-p", "dmml", "--example", "validate_machines", "--", out_path],
        cwd="/home/user/dmml",
        capture_output=True,
        text=True,
    )
    print(result.stdout)
    if result.returncode != 0:
        print("VALIDATION FAILED:")
        print(result.stderr[-3000:])
    else:
        print("Validation passed.")


if __name__ == "__main__":
    main()
