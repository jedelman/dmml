#!/usr/bin/env python3
"""The parallel/head-to-head checkpoint: the same tightened scenarios,
the same three models, dispatched TWICE per (model, scenario) -- once
asking for JSON (GRAMMAR.md as reference), once asking for the new
Surface syntax (SURFACE.md as reference) -- so the two authoring
surfaces can actually be compared on identical tasks, not just each
scored well in isolation.

Usage:
    OPENROUTER_API_KEY=... python3 dispatch.py

Writes one JSON line per (model, scenario, format) to
results/dispatch.ndjson: {"id", "model", "scenario", "format", "reply"}.
"""
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
GRAMMAR_PATH = HERE.parent / "dmml-agent-nucleus" / "GRAMMAR.md"
SURFACE_PATH = HERE.parent / "dmml-hs" / "SURFACE.md"
SCENARIOS_PATH = HERE / "scenarios.json"
OUT_PATH = HERE / "results" / "dispatch.ndjson"

MODELS = [
    "google/gemini-3.7-flash",
    "z-ai/glm-5.3-flash",
    "moonshotai/kimi-k2.5",
]

NEEDS_REASONING_NONE = {"moonshotai/kimi-k2.5"}
# The higher of the two ceilings each standalone checkpoint separately
# needed to bump to after hitting a reasoning-exhausts-budget artifact
# (content: null, no error) -- glm-5.3-flash hit it at 4000 in the JSON
# checkpoint, gemini-3.7-flash hit it at 8000 in the Surface one.
MAX_TOKENS = 12000

JSON_SYSTEM_PROMPT_TEMPLATE = """You are authoring content for a DMML (Desiring-Machine Markup Language) \
world. JSON is the ONLY authoring surface -- there is no text grammar. Below is the complete, \
current grammar reference. Read it carefully; it reflects the real parser exactly, not an \
approximation.

--- GRAMMAR.md ---
{ref}
--- end GRAMMAR.md ---

Respond with exactly ONE fenced code block (```json ... ```) containing a single JSON object \
matching the top-level `{{"update": [...]}}` shape described above -- even for a single commit, \
wrap it as one batch of one: `{{"update": [{{"commits": [...]}}]}}`. You may add brief prose \
outside the fence, but the fence must contain nothing except the JSON object."""

SURFACE_SYSTEM_PROMPT_TEMPLATE = """You are authoring content for a DMML (Desiring-Machine Markup Language) \
world, using its NEW text authoring syntax. Below is the complete, current syntax reference -- read \
it carefully, it reflects the real parser exactly, not an approximation. This syntax is scoped to \
single commits only (no machine/reference/batching support yet).

--- SURFACE.md ---
{ref}
--- end SURFACE.md ---

Respond with exactly ONE fenced code block containing a single DMML commit in the syntax above -- \
starting with `commit <verb>` and using indentation to show what belongs to the commit body, exactly \
as the examples in SURFACE.md show. You may add brief prose outside the fence, but the fence must \
contain nothing except the DMML commit text -- no JSON, no other format."""


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
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
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

    grammar = GRAMMAR_PATH.read_text()
    surface = SURFACE_PATH.read_text()
    scenarios = json.loads(SCENARIOS_PATH.read_text())
    json_system_prompt = JSON_SYSTEM_PROMPT_TEMPLATE.format(ref=grammar)
    surface_system_prompt = SURFACE_SYSTEM_PROMPT_TEMPLATE.format(ref=surface)

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUT_PATH.open("w") as out:
        for model in MODELS:
            for scenario in scenarios:
                sid, task, title = scenario["id"], scenario["task"], scenario["title"]
                for fmt, sys_prompt in (("json", json_system_prompt), ("surface", surface_system_prompt)):
                    print(f"dispatching {model} / {sid} / {fmt} ...", file=sys.stderr)
                    try:
                        reply = call_openrouter(api_key, model, sys_prompt, task)
                    except Exception as e:  # noqa: BLE001
                        reply = f"[dispatch error: {e}]"
                        print(f"  ERROR: {e}", file=sys.stderr)
                    record = {"id": sid, "model": model, "scenario": title, "format": fmt, "reply": reply}
                    out.write(json.dumps(record) + "\n")
                    out.flush()
                    time.sleep(1)

    print(f"wrote {OUT_PATH}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
