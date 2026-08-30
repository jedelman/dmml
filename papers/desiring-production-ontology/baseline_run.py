#!/usr/bin/env python3
"""Baseline for the amber-cracks convergence measurement: the SAME task,
SAME model, SAME seed image, SAME 5-turn speaker order (gamma, alpha,
beta, delta, gamma) as the real DMML run in
RUN-2026-08-30-deepseek-amber-cracks.transcript.jsonl -- but with no DMML
grammar at all. Each agent sees the accumulated free text (not a
structured fact digest) and adds 1-2 short sentences continuing it. This
is the "same task without DMML" baseline Jason asked for: convergence
measured here isolates what the shared graph/vocabulary structure adds,
if anything, over plain shared-context prose continuation with the same
model under the same conditions.
"""
import json
import os
import sys
import time
import urllib.request

API_KEY = os.environ["OPENROUTER_API_KEY"]
MODEL = "deepseek/deepseek-v4-flash-0731"
SPEAKERS = ["gamma", "alpha", "beta", "delta", "gamma"]
SEED = "A room glimmers with faint amber light from the cracks in the stone floor."

PROMPT_TMPL = """You are "{label}", one of several people taking turns continuing a shared piece of collaborative fiction. No one is coordinating with anyone else beyond reading what's already been written.

Story so far:
{story}

Add 1-2 short sentences continuing it, in the same spirit. Don't restate what's already there. Respond with ONLY the new sentences, no preamble, no quotes, no attribution."""


def call_model(prompt):
    body = json.dumps({
        "model": MODEL,
        "reasoning": {"enabled": False},
        "max_tokens": 300,
        "messages": [{"role": "user", "content": prompt}],
    }).encode()
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=body,
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
    )
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                data = json.loads(resp.read())
            if "error" in data:
                raise RuntimeError(data["error"])
            return data["choices"][0]["message"]["content"].strip()
        except Exception as e:
            print(f"  attempt {attempt} failed: {e}", file=sys.stderr)
            time.sleep(5 * (attempt + 1))
    raise RuntimeError("all attempts failed")


def main():
    story = SEED
    turns = [{"speaker": "seed", "text": SEED}]
    for i, speaker in enumerate(SPEAKERS, start=1):
        prompt = PROMPT_TMPL.format(label=speaker, story=story)
        text = call_model(prompt)
        print(f"--- turn {i} ({speaker}) ---\n{text}\n")
        turns.append({"speaker": speaker, "text": text})
        story = story + "\n" + text

    with open("BASELINE-2026-08-30-amber-cracks.json", "w") as f:
        json.dump(turns, f, indent=2)
    print("Wrote BASELINE-2026-08-30-amber-cracks.json")


if __name__ == "__main__":
    main()
