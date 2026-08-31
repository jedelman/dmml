#!/usr/bin/env python3
"""Tightening the schema itself instead of the prompt (Jason, 2026-08-30:
"can we tighten our tool schema to be enough for the model to one shot
it? all constraints should be structural"). The real bug every failed
attempt shared: `has_content` ("a transition needs at least one of a
guard, a from+to pair, or an effect") lived ONLY in prose -- our own
JSON Schema let a transition with none of the three validate at the
schema level, and only our downstream Rust validator caught it. A model
under `strict: false` structured output has no obligation to satisfy
prose at all; even under `strict: true`, a schema that doesn't ENCODE
the constraint can't enforce it.

This schema makes `has_content` structural: a TransitionInput is an
`anyOf` of three required-shaped branches (guard-bearing, from+to-
bearing, effect-bearing) rather than one object with everything
optional. A transition missing all three is now something the schema
itself cannot represent, not just something the prompt asks the model
to avoid.

Two more real tightenings, same spirit ("all constraints should be
structural," not prose):
  - Every ident/node_ref field gets its real regex pattern (already had
    these from schema.rs, kept here).
  - `strict: true` this time, not `strict: false` -- real constrained
    decoding per OpenRouter's `structured_outputs` support (confirmed
    supported for this model), not an advisory hint. Every object gets
    `additionalProperties: false` and every property listed `required`
    (nullable where genuinely optional) per the strict-schema
    convention this requires.
  - The schema is scoped to ONLY what the Vala needs (`{"update":
    [{"machines": [...]}]}`) -- no `commits`, no `refs`, no `consumes`.
    Structural tightening isn't just adding constraints, it's also
    removing surface area a model could misuse that it never needs for
    this task.

Same model, same low reasoning effort, same real validator -- only the
schema and prompt strictness changed.
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

IDENT_PATTERN = "^[A-Za-z][A-Za-z0-9_]*$"
NODE_REF_PATTERN = r"^[A-Za-z0-9][A-Za-z0-9_.]*(/[A-Za-z0-9][A-Za-z0-9_.]*)*$"


def nullable(schema):
    """Strict mode requires every property to be listed in `required`
    (always present); a genuinely optional field becomes "present, but
    may be null" instead of "may be absent" -- this wraps a schema to
    allow null."""
    return {"anyOf": [schema, {"type": "null"}]}


def strict_object(properties, required_present, additional=False):
    """Strict-mode object: every key in `properties` must be listed in
    `required` (nullable ones still go there) -- OpenAI/OpenRouter's
    strict-schema convention, not optional at the schema level at all."""
    return {
        "type": "object",
        "properties": properties,
        "required": list(properties.keys()),
        "additionalProperties": additional,
    }


def build_schema():
    pattern_term = {
        "anyOf": [
            strict_object({"kind": {"const": "self"}}, []),
            strict_object({"kind": {"const": "param"}, "value": {"type": "string", "pattern": IDENT_PATTERN}}, []),
            strict_object({"kind": {"const": "var"}, "value": {"type": "string", "pattern": IDENT_PATTERN}}, []),
            strict_object({"kind": {"const": "node"}, "value": {"type": "string", "pattern": NODE_REF_PATTERN}}, []),
        ]
    }

    pattern_hop = strict_object({
        "predicate": {"type": "string", "pattern": IDENT_PATTERN, "description": "NEVER \"a\" or \"rdf:type\" -- state checks always use the literal predicate \"state\"."},
        "term": pattern_term,
    }, [])

    exists_input = strict_object({
        "anchor": pattern_term,
        "hops": {"type": "array", "minItems": 1, "items": pattern_hop},
    }, [])

    guard_input = strict_object({
        "negated": {"type": "boolean"},
        "exists": exists_input,
    }, [])

    effect_input = {
        "anyOf": [
            strict_object({"kind": {"const": "assert"}, "ident": {"type": "string", "pattern": IDENT_PATTERN}}, []),
            strict_object({"kind": {"const": "retract"}, "ident": {"type": "string", "pattern": IDENT_PATTERN}}, []),
        ]
    }

    # The structural fix: TransitionInput is anyOf three required-shaped
    # branches, not one object where everything is independently
    # optional. Every branch still lists every field (strict mode
    # requires that), but each branch forces a DIFFERENT field to be
    # genuinely non-null/non-empty -- that's what makes "at least one of
    # guard/from+to/effect" a structural fact instead of a prose rule.
    common_props = lambda guards_required, from_to_required, effects_required: {
        "ident": {"type": "string", "pattern": IDENT_PATTERN},
        "params": nullable({"type": "array", "items": {"type": "string", "pattern": IDENT_PATTERN}}),
        "from": ({"type": "string", "pattern": IDENT_PATTERN} if from_to_required else nullable({"type": "string", "pattern": IDENT_PATTERN})),
        "to": ({"type": "string", "pattern": IDENT_PATTERN} if from_to_required else nullable({"type": "string", "pattern": IDENT_PATTERN})),
        "guards": ({"type": "array", "minItems": 1, "items": guard_input} if guards_required else nullable({"type": "array", "items": guard_input})),
        "effects": ({"type": "array", "minItems": 1, "items": effect_input} if effects_required else nullable({"type": "array", "items": effect_input})),
    }

    transition_guard_branch = strict_object(common_props(True, False, False), [])
    transition_fromto_branch = strict_object(common_props(False, True, False), [])
    transition_effect_branch = strict_object(common_props(False, False, True), [])

    transition_input = {
        "anyOf": [transition_guard_branch, transition_fromto_branch, transition_effect_branch],
        "description": "A transition MUST match at least one branch: non-empty guards, OR a real from+to pair, OR non-empty effects. A transition with all three null/empty matches NONE of these branches and is invalid.",
    }

    state_input = strict_object({"ident": {"type": "string", "pattern": IDENT_PATTERN}}, [])

    machine_input = strict_object({
        "node": {"type": "string", "pattern": NODE_REF_PATTERN},
        "states": {"type": "array", "items": state_input},
        "transitions": {"type": "array", "items": transition_input},
    }, [])

    batch_input = strict_object({"machines": {"type": "array", "items": machine_input}}, [])

    return strict_object({"update": {"type": "array", "items": batch_input}}, [])


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
  NEGATED guard (Valinor/forest must NOT be depleted).
- Valinor/roof: unroofed -> roofed. `add_roof($wall_source,
  $frame_source)`, a third two-input join -- the capstone, a roofed house.

Every guard checks REAL prior state, using the predicate "state" (NEVER
"a"/rdf:type). Every transition's "verb" IS its ident."""

VALA_PROMPT = f"""You are one of the Valar -- one of the shaping powers who gives this
world new machinery for others to later work within, not a player-agent
operating turn by turn.

{WORLD_SO_FAR}

Your task: propose ONE new machine that this world is still missing.
Use your own judgment -- something that extends the existing chain
(consumes from it, gates on it, or opens a new resource line), not a
decorative reskin. Favor real cross-node or $param consumption over
narration.

Respond with ONLY the raw JSON object matching the schema. Every
transition you write is checked against a schema that requires at least
one of: real guards, a real from+to pair, or real effects -- the schema
itself will reject anything else, so make sure whichever one(s) you
intend are actually filled in, not left null."""


def call_model(prompt, schema):
    body = {
        "model": MODEL,
        "max_tokens": 24000,
        "messages": [{"role": "user", "content": prompt}],
        "reasoning": {"effort": REASONING_EFFORT},
        "response_format": {
            "type": "json_schema",
            "json_schema": {"name": "update", "strict": True, "schema": schema},
        },
        "include_reasoning": True,
    }
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        data = json.loads(resp.read())
    return data


def main():
    schema = build_schema()
    schema_path = os.path.join(os.path.dirname(__file__), "STRICT-SCHEMA-2026-08-30.json")
    json.dump(schema, open(schema_path, "w"), indent=2)
    print(f"Wrote schema to {schema_path} ({len(json.dumps(schema))} chars)\n", file=sys.stderr)

    data = call_model(VALA_PROMPT, schema)
    if "error" in data:
        print("API ERROR (schema itself may have been rejected):")
        print(json.dumps(data["error"], indent=2))
        return

    msg = data["choices"][0]["message"]
    content = msg.get("content")
    reasoning = msg.get("reasoning_content") or msg.get("reasoning")

    if reasoning:
        print(f"=== Reasoning ({len(reasoning)} chars) ===")
        print(reasoning[:2000])
        print()

    if not content:
        print(f"EMPTY CONTENT (reasoning present: {bool(reasoning)}, len {len(reasoning) if reasoning else 0})")
        return

    cleaned = content.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
    print("=== Raw output ===")
    print(cleaned)

    out_path = os.path.join(os.path.dirname(__file__), "VALAR-STRICT-2026-08-30.json")
    with open(out_path, "w") as f:
        f.write(cleaned)
    print(f"\nWrote {out_path}")

    print("\n=== Validating against dmml::from_json (real check) ===\n")
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
        print("*** VALIDATION PASSED -- ONE-SHOT SUCCESS ***")


if __name__ == "__main__":
    main()
