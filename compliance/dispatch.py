#!/usr/bin/env python3
"""Dispatch scenarios.json to a set of light models via OpenRouter and
record raw replies for scoring.

First real run: 2026-08-31, after the user dropped auto mode and
approved the outbound curl calls directly (an earlier attempt under
auto mode was blocked by the permission classifier before any request
went out). Result: 15/15 accepted against the real production
`update_from_json` boundary -- see results/report.md. That run also
found two real bugs in this script, now fixed: GRAMMAR_PATH pointed one
directory too deep, and glm-5.3-flash/gemini-3.7-flash both reject
`reasoning.effort: "none"` outright (mandatory reasoning, unlike Kimi) --
MAX_TOKENS is sized generously to leave room for real content after it.

Usage:
    OPENROUTER_API_KEY=... python3 dispatch.py

Writes one JSON line per (model, scenario) to results/dispatch.ndjson,
in exactly the shape dmml/examples/compliance_check.rs expects on
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
GRAMMAR_PATH = HERE.parent / "dmml-agent-nucleus" / "GRAMMAR.md"
SCENARIOS_PATH = HERE / "scenarios.json"
OUT_PATH = HERE / "results" / "dispatch.ndjson"

# The three "light model" candidates worth checking as a first
# checkpoint: the two production in-product DMML-authoring targets
# named in written-world/CLAUDE.md's "In-product authoring agents"
# section (Gemini-Flash, GLM), plus the dev-tooling pipeline's Kimi
# role as a known-hallucination-prone stress test (its own doc warns it
# "will get things wrong that don't show up as compile errors" when not
# given exact signatures -- GRAMMAR.md is exactly that exactness test).
MODELS = [
    "google/gemini-3.7-flash",
    "z-ai/glm-5.3-flash",
    "moonshotai/kimi-k2.5",
]

# Kimi is documented (written-world/CLAUDE.md) as reasoning-on-by-default
# with no visible output otherwise -- silently burns the whole budget and
# returns nothing unless told not to reason. Verified directly against
# the live API (not just from the doc) that glm-5.3-flash and
# gemini-3.7-flash are the OPPOSITE: both reject `reasoning.effort:
# "none"` outright ("Reasoning is mandatory for this endpoint and cannot
# be disabled") -- CLAUDE.md's own caveat that glm-5.3-flash's
# reasoning support "was not yet verified" turned out to matter. Their
# reasoning tokens count against max_tokens same as Kimi's would, so
# MAX_TOKENS below is sized generously rather than tight, to leave room
# for real content after mandatory reasoning.
NEEDS_REASONING_NONE = {"moonshotai/kimi-k2.5"}
# Bumped from 4000 after a real run: glm-5.3-flash's mandatory reasoning
# hit exactly that ceiling on one scenario (867-1000+ reasoning tokens
# observed even on a much simpler isolated prompt) and returned
# content: null with no error -- a token-budget artifact, not a
# comprehension failure, but one this checkpoint needs enough headroom
# to not manufacture.
MAX_TOKENS = 8000

SYSTEM_PROMPT_TEMPLATE = """You are authoring content for a DMML (Desiring-Machine Markup Language) \
world. JSON is the ONLY authoring surface -- there is no text grammar. Below is the complete, \
current grammar reference. Read it carefully; it reflects the real parser exactly, not an \
approximation.

--- GRAMMAR.md ---
{grammar}
--- end GRAMMAR.md ---

Respond with exactly ONE fenced code block (```json ... ```) containing a single JSON object \
matching the top-level `{{"update": [...]}}` shape described above -- even for a single commit, \
wrap it as one batch of one: `{{"update": [{{"commits": [...]}}]}}`. You may add brief prose \
outside the fence, but the fence must contain nothing except the JSON object."""


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
    scenarios = json.loads(SCENARIOS_PATH.read_text())
    system_prompt = SYSTEM_PROMPT_TEMPLATE.format(grammar=grammar)

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUT_PATH.open("w") as out:
        for model in MODELS:
            for scenario in scenarios:
                sid, task = scenario["id"], scenario["task"]
                print(f"dispatching {model} / {sid} ...", file=sys.stderr)
                try:
                    reply = call_openrouter(api_key, model, system_prompt, task)
                except Exception as e:  # noqa: BLE001 -- any transport/parse failure is a real,
                    # scorable "unparseable" outcome, not a reason to abort the whole run
                    # (http.client.IncompleteRead on a mid-transfer drop was observed live).
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
