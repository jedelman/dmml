#!/usr/bin/env python3
"""The "informed vs. blind" A/B: same task, same three models, once
with NO world context (blind -- the model has to invent a predicate
name for a relationship that, unbeknownst to it, already has one) and
once WITH a real materialized snapshot of examples/shrine-genesis.dmml
handed to it as context (informed -- the snapshot shows the relation
already exists, named 'accepts').

This tests the actual claim under scrutiny: does handing agents a
materialized subset of the world prevent predicate-name drift (two
different names ending up used for the same relationship across
independently-authored content) -- not just "does it parse."

Usage:
    OPENROUTER_API_KEY=... python3 dispatch.py

Writes one JSON line per (model, scenario, condition) to
results/dispatch.ndjson: {"id", "model", "scenario", "condition", "reply"}.
"""
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
DMML_HS = HERE.parent / "dmml-hs"
SURFACE_PATH = DMML_HS / "SURFACE.md"
GENESIS_PATH = DMML_HS / "examples" / "shrine-genesis.dmml"
SCENARIOS_PATH = HERE / "scenarios.json"
OUT_PATH = HERE / "results" / "dispatch.ndjson"

MODELS = [
    "google/gemini-3.7-flash",
    "z-ai/glm-5.3-flash",
    "moonshotai/kimi-k2.5",
]

NEEDS_REASONING_NONE = {"moonshotai/kimi-k2.5"}
MAX_TOKENS = 12000

BASE_SYSTEM_PROMPT = """You are authoring content for a DMML (Desiring-Machine Markup Language) \
world, using its text authoring syntax. Below is the complete, current syntax reference -- read it \
carefully, it reflects the real parser exactly, not an approximation.

--- SURFACE.md ---
{surface}
--- end SURFACE.md ---
{snapshot_block}
Respond with exactly ONE fenced code block containing a single DMML commit in the syntax above. You \
may add brief prose outside the fence, but the fence must contain nothing except the DMML commit \
text -- no JSON, no other format."""

SNAPSHOT_BLOCK_TEMPLATE = """
Below is the CURRENT STATE of the world, already materialized from commits other agents have \
already authored. Before declaring any new relation or attribute predicate, check whether one \
already exists here for what you need -- reuse it by name instead of inventing a new one if it \
already covers the relationship you're authoring.

--- CURRENT WORLD STATE ---
{world_state}
--- end CURRENT WORLD STATE ---
"""


def build_snapshot() -> str:
    binary = DMML_HS / "render-snapshot"
    subprocess.run(
        ["ghc", "-isrc", "-iapp", "-O0", "app/RenderSnapshot.hs", "-o", str(binary)],
        cwd=DMML_HS,
        check=True,
        capture_output=True,
        text=True,
    )
    result = subprocess.run([str(binary), str(GENESIS_PATH)], capture_output=True, text=True, check=True)
    return result.stdout


def call_openrouter(api_key: str, model: str, system_prompt: str, user_prompt: str) -> str:
    payload = {
        "model": model,
        "max_tokens": MAX_TOKENS,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ],
    }
    if model in NEEDS_REASONING_NONE:
        payload["reasoning"] = {"effort": "none"}

    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=200) as resp:
        body = json.loads(resp.read().decode("utf-8"))
    return body["choices"][0]["message"]["content"]


def main() -> int:
    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        print("OPENROUTER_API_KEY not set in environment", file=sys.stderr)
        return 1

    surface = SURFACE_PATH.read_text()
    world_state = build_snapshot()
    scenarios = json.loads(SCENARIOS_PATH.read_text())

    blind_prompt = BASE_SYSTEM_PROMPT.format(surface=surface, snapshot_block="")
    informed_prompt = BASE_SYSTEM_PROMPT.format(
        surface=surface, snapshot_block=SNAPSHOT_BLOCK_TEMPLATE.format(world_state=world_state)
    )

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUT_PATH.open("w") as out:
        for model in MODELS:
            for scenario in scenarios:
                sid, task, title = scenario["id"], scenario["task"], scenario["title"]
                for condition, sys_prompt in (("blind", blind_prompt), ("informed", informed_prompt)):
                    print(f"dispatching {model} / {sid} / {condition} ...", file=sys.stderr)
                    try:
                        reply = call_openrouter(api_key, model, sys_prompt, task)
                    except Exception as e:  # noqa: BLE001
                        reply = f"[dispatch error: {e}]"
                        print(f"  ERROR: {e}", file=sys.stderr)
                    record = {"id": sid, "model": model, "scenario": title, "condition": condition, "reply": reply}
                    out.write(json.dumps(record) + "\n")
                    out.flush()
                    time.sleep(1)

    print(f"wrote {OUT_PATH}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
