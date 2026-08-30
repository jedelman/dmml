#!/usr/bin/env python3
"""Second baseline, the controlled one: identical to baseline_run.py --
same model, same seed, same 5-turn speaker order -- except the prompt
now ALSO carries the DMML prompt's explicit "reuse existing
vocabulary/imagery" instruction. This isolates the variable the first
baseline confounded: does DMML's structure (declared predicates, a fact
digest) drive the higher convergence measured in CONVERGENCE-2026-08-30-
stats.json, or was it just the explicit reuse instruction, which the
first free-text baseline never got? If this run converges close to the
DMML run, the instruction was doing the work. If it stays close to the
first (uninstructed) baseline, the instruction alone isn't sufficient and
something about DMML's structure is contributing.
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

Add 1-2 short sentences continuing it, in the same spirit. REUSE the existing images and vocabulary already established above where they fit, rather than introducing unrelated new ones -- don't restate what's already there, but stay close to the same sensory register (the same light, the same sounds, the same textures) instead of drifting to new imagery. Respond with ONLY the new sentences, no preamble, no quotes, no attribution."""


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

    with open("BASELINE-REUSE-2026-08-30-amber-cracks.json", "w") as f:
        json.dump(turns, f, indent=2)
    print("Wrote BASELINE-REUSE-2026-08-30-amber-cracks.json")


if __name__ == "__main__":
    main()
