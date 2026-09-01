#!/usr/bin/env python3
"""Dispatch scenarios.json to the same three light models used for the
JSON compliance checkpoint, but authoring against the NEW Haskell-styled
text surface (dmml-hs/SURFACE.md) instead of JSON/GRAMMAR.md.

Usage:
    OPENROUTER_API_KEY=... python3 dispatch.py

Writes one JSON line per (model, scenario) to results/dispatch.ndjson,
in exactly the shape dmml-hs's compliance-check-surface expects on
stdin: {"id", "model", "scenario", "reply"}.
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

# Same three models as the JSON checkpoint (compliance/dispatch.py),
# for a real apples-to-apples comparison between the two surfaces.
MODELS = [
    "google/gemini-3.7-flash",
    "z-ai/glm-5.3-flash",
    "moonshotai/kimi-k2.5",
]

# Kimi needs reasoning disabled explicitly or it returns nothing.
# glm-5.3-flash and gemini-3.7-flash reject reasoning.effort:"none"
# outright (mandatory reasoning) -- verified live during the JSON
# checkpoint, see written-world/MODELS.md. Same MAX_TOKENS headroom
# reasoning applies here for the same reason.
NEEDS_REASONING_NONE = {"moonshotai/kimi-k2.5"}
# Bumped from 8000 after a real run: gemini-3.7-flash hit the same
# reasoning-exhausts-the-budget artifact on the adversarial scenario
# that glm-5.3-flash hit in the JSON checkpoint (content: null, no
# error) -- the "trap" prompt seems to reliably provoke longer
# reasoning across models, not model-specific.
MAX_TOKENS = 12000

SYSTEM_PROMPT_TEMPLATE = """You are authoring content for a DMML (Desiring-Machine Markup Language) \
world, using its NEW text authoring syntax. Below is the complete, current syntax reference -- read \
it carefully, it reflects the real parser exactly, not an approximation. This syntax is scoped to \
single commits only (no machine/reference/batching support yet).

--- SURFACE.md ---
{surface}
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
                except Exception as e:  # noqa: BLE001 -- any transport/parse failure is a real,
                    # scorable "rejected" outcome, not a reason to abort the whole run.
                    reply = f"[dispatch error: {e}]"
                    print(f"  ERROR: {e}", file=sys.stderr)
                record = {"id": sid, "model": model, "scenario": scenario["title"], "reply": reply}
                out.write(json.dumps(record) + "\n")
                out.flush()
                time.sleep(1)  # be polite to the API

    print(f"wrote {OUT_PATH}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
