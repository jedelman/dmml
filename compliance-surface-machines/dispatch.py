#!/usr/bin/env python3
"""Dispatch machine-authoring scenarios to the same three light models,
against the Surface syntax's machine grammar (added to SURFACE.md
2026-09-01, right after a hand-authored example -- shrine/threshold, not
included in these scenarios -- was verified to parse correctly first).

Usage:
    OPENROUTER_API_KEY=... python3 dispatch.py

Writes one JSON line per (model, scenario) to results/dispatch.ndjson,
in the shape dmml-hs's compliance-check-surface expects for machine
records: {"id", "model", "scenario", "reply", "kind": "machine"}.
"""
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
SURFACE_PATH = HERE.parent / "dmml-hs" / "SURFACE.md"
SCENARIOS_PATH = HERE / "scenarios.json"
OUT_PATH = HERE / "results" / "dispatch.ndjson"

MODELS = [
    "google/gemini-3.7-flash",
    "z-ai/glm-5.3-flash",
    "moonshotai/kimi-k2.5",
]

NEEDS_REASONING_NONE = {"moonshotai/kimi-k2.5"}
MAX_TOKENS = 12000  # per written-world/MODELS.md's recorded floor for the two mandatory-reasoning models

SYSTEM_PROMPT_TEMPLATE = """You are authoring content for a DMML (Desiring-Machine Markup Language) \
world, using its text authoring syntax -- specifically a MACHINE this time, not a commit. Below is \
the complete, current syntax reference, including the machine grammar section -- read it carefully, \
it reflects the real parser exactly, not an approximation.

--- SURFACE.md ---
{surface}
--- end SURFACE.md ---

Respond with exactly ONE fenced code block containing a single DMML machine in the syntax above -- \
starting with `machine <node_ref>` and using indentation to show what belongs to the machine body \
(a states block, and one or more transition blocks), exactly as the machine example in SURFACE.md \
shows. You may add brief prose outside the fence, but the fence must contain nothing except the \
DMML machine text -- no JSON, no other format, and no commit content."""


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

    surface = SURFACE_PATH.read_text()
    scenarios = json.loads(SCENARIOS_PATH.read_text())
    system_prompt = SYSTEM_PROMPT_TEMPLATE.format(surface=surface)

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUT_PATH.open("w") as out:
        for model in MODELS:
            for scenario in scenarios:
                sid, task = scenario["id"], scenario["task"]
                print(f"dispatching {model} / {sid} ...", file=sys.stderr)
                try:
                    reply = call_openrouter(api_key, model, system_prompt, task)
                except Exception as e:  # noqa: BLE001
                    reply = f"[dispatch error: {e}]"
                    print(f"  ERROR: {e}", file=sys.stderr)
                record = {
                    "id": sid,
                    "model": model,
                    "scenario": scenario["title"],
                    "reply": reply,
                    "kind": "machine",
                }
                out.write(json.dumps(record) + "\n")
                out.flush()
                time.sleep(1)

    print(f"wrote {OUT_PATH}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
