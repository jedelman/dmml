#!/usr/bin/env python3
"""The untested half of Jason's framing: "models can operate machines
but not create them." Every prior test in this thread was the DESIGN
tier (mint new machinery). This is the OPERATE tier: given the real,
already-built Valinor world (from `valinor_house.rs`), can a cheap model
pick ONE legal action and fire it -- with the legal-action-space itself
enforced structurally (a real `oneOf` of const-tagged branches, one per
actual transition), not described in a prompt as a menu to please choose
from correctly?

Same thesis as VALAR-EVAL-2026-08-30.md's Round 4, applied to the
simpler tier: "no logic should live in prose - it's decorative. it
should live in the structure." The model literally CANNOT propose a
transition that doesn't exist, or supply the wrong param names for a
given transition -- the schema has one branch per (node, transition)
pair in the real machine catalog, each with exactly that transition's
own declared params as required fields. There is no "pick from this
list" instruction because there doesn't need to be one; the union type
IS the list.

Snapshot used: right after the seed commit (before anything has fired).
Several transitions are legitimately available with no unmet
preconditions: `raise` (Valinor), `wash` (streambed), `well_up`
(spring), `gather` (forest) -- none has a guard beyond its own implicit
from-state, all real, structurally distinct choices.

Uses reasoning DISABLED (the original fast/cheap acting-agent
condition from earlier this session) -- operate is supposed to be the
inexpensive tier, unlike Round 4's low-but-nonzero design-tier effort.
"""
import json
import os
import subprocess
import sys
import time
import urllib.request

API_KEY = os.environ["OPENROUTER_API_KEY"]
MODEL = "deepseek/deepseek-v4-flash-0731"
IDENT_PATTERN = "^[A-Za-z][A-Za-z0-9_]*$"
NODE_REF_PATTERN = r"^[A-Za-z0-9][A-Za-z0-9_.]*(/[A-Za-z0-9][A-Za-z0-9_.]*)*$"

# The real catalog, transcribed from valinor_house.rs's own seed world --
# every (node, transition, params) triple that actually exists. This IS
# the legal action space; nothing outside this list is representable.
CATALOG = [
    ("Valinor", "raise", []),
    ("Valinor", "uplift", []),
    ("Valinor/quarry", "quarry", []),
    ("Valinor/quarry", "grind", []),
    ("Valinor/quarry", "wet", []),
    ("Valinor/quarry", "fire", []),
    ("Valinor/streambed", "wash", []),
    ("Valinor/spring", "well_up", []),
    ("Valinor/mortar", "mix", ["sand_source", "water_source"]),
    ("Valinor/wall", "build", ["brick_source", "mortar_source"]),
    ("Valinor/forest", "gather", []),
    ("Valinor/forest", "overgather", []),
    ("Valinor/carpentry", "make_frame", []),
    ("Valinor/roof", "add_roof", ["wall_source", "frame_source"]),
    ("Valinor/house", "construct_house", []),
]

WORLD_SNAPSHOT = """The current, real state of the world (nothing has happened yet -- this
is the seed):

  Valinor           state: unformed
  Valinor/quarry    state: untouched
  Valinor/streambed state: bare
  Valinor/spring    state: dry
  Valinor/mortar    state: unmixed
  Valinor/wall      state: unbuilt
  Valinor/forest    state: full
  Valinor/carpentry state: no_frame
  Valinor/roof      state: unroofed
  Valinor/house     state: unbuilt"""

PROMPT = f"""{WORLD_SNAPSHOT}

Choose ONE action to take right now. Respond with only the JSON object
matching the schema -- the schema itself defines every action that
actually exists in this world; there is nothing to choose beyond what
it already enumerates."""


def build_schema():
    branches = []
    for node, transition, params in CATALOG:
        props = {
            "node": {"const": node},
            "transition": {"const": transition},
        }
        required = ["node", "transition"]
        if params:
            param_props = {p: {"type": "string", "pattern": NODE_REF_PATTERN} for p in params}
            props["params"] = {
                "type": "object",
                "properties": param_props,
                "required": params,
                "additionalProperties": False,
            }
            required.append("params")
        else:
            props["params"] = {"type": "null"}
            required.append("params")
        branches.append({"type": "object", "properties": props, "required": required, "additionalProperties": False})
    return {"anyOf": branches}


def call_model(prompt, schema):
    body = {
        "model": MODEL,
        "max_tokens": 2000,
        "messages": [{"role": "user", "content": prompt}],
        "reasoning": {"enabled": False},
        "response_format": {
            "type": "json_schema",
            "json_schema": {"name": "action", "strict": True, "schema": schema},
        },
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
            print(f"  attempt {attempt} failed: {e}", file=sys.stderr)
            time.sleep(5 * (attempt + 1))
    raise RuntimeError("all attempts failed")


def main():
    schema = build_schema()
    schema_path = os.path.join(os.path.dirname(__file__), "OPERATE-SCHEMA-2026-08-30.json")
    json.dump(schema, open(schema_path, "w"), indent=2)
    print(f"Schema: {len(CATALOG)} branches, {len(json.dumps(schema))} chars\n")

    raw = call_model(PROMPT, schema)
    cleaned = raw.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
    print("=== Model's chosen action ===")
    print(cleaned)

    action = json.loads(cleaned)
    node, transition, params = action["node"], action["transition"], action.get("params") or {}

    catalog_match = any(c[0] == node and c[1] == transition for c in CATALOG)
    print(f"\nStructurally valid (in the real catalog): {catalog_match}")
    print(f"Chosen: {node} :: {transition}({params})")

    out_path = os.path.join(os.path.dirname(__file__), "OPERATE-CHOICE-2026-08-30.json")
    json.dump(action, open(out_path, "w"), indent=2)

    print("\n=== Firing it for real against dmml::machine::commit_fires_transition ===\n")
    result = subprocess.run(
        ["cargo", "run", "-p", "dmml", "--example", "operate_check", "--", out_path],
        cwd="/home/user/dmml",
        capture_output=True,
        text=True,
    )
    print(result.stdout)
    if result.returncode != 0:
        print(result.stderr[-2000:])


if __name__ == "__main__":
    main()
