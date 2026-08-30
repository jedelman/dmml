#!/usr/bin/env python3
"""Isolates the three candidate causes of the "verb monologue" / repetition
pathology observed in deepseek/deepseek-v4-flash-0731 runs with reasoning
disabled (see GROUNDING-2026-08-30-amber-cracks.md and this session's
conversation). Uses a deliberately SIMPLE schema (not the full DMML one)
so a real strict-mode schema is easy to build correctly and the variable
under test stays isolated: does reasoning-suppression push deliberation
into the one open string field, and does a repetition-penalty stop the
degenerate-loop variant of that?

Four conditions, N trials each, same prompt each time:
  A. baseline    -- reasoning disabled, non-strict schema (today's setup)
  B. strict      -- reasoning disabled, STRICT schema (real constrained
                    decoding: additionalProperties:false everywhere, all
                    properties required, maxLength enforced not advisory)
  C. reasoning_on-- reasoning left at default (key omitted), non-strict
  D. freq_penalty-- reasoning disabled, non-strict, frequency_penalty=0.6

For each trial, records: raw response, whether it parsed as valid JSON,
verb length, and whether verb shows the repetition-loop signature (a
substring of 4+ words repeated 3+ times).
"""
import json
import os
import re
import sys
import time
import urllib.request

API_KEY = os.environ["OPENROUTER_API_KEY"]
MODEL = "deepseek/deepseek-v4-flash-0731"
N_TRIALS = 6

PROMPT = """You are agent "gamma" in a SHARED, LIVE dungeon world. Other agents are acting
concurrently; you cannot see them acting live, only what has already landed. The shared
world has a hub room named exactly "room/hub".

2 new commit(s) have landed since your last check:
  [commit #0, verb="mints"] room/hub a Room
  [commit #1, verb="notices"] thought_beta_1 wonders "what lies beyond the hub"

The shared world already has these predicates in use: wonders (attribute), a (relation,
built-in). REUSE any that fit instead of inventing a synonym.

Extend the shared world however you think is most interesting right now -- a new room, an
item, an NPC, a reaction to something another agent just built. Keep it small: 1-4 facts.

Respond with ONLY the raw JSON object matching the schema. No prose, no markdown fences."""

NON_STRICT_SCHEMA = {
    "type": "object",
    "properties": {
        "verb": {"type": "string", "maxLength": 24, "description": "a SINGLE short word, never a sentence or plan"},
        "facts": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "subject": {"type": "string"},
                    "predicate": {"type": "string"},
                    "value": {"type": "string"},
                },
                "required": ["subject", "predicate", "value"],
            },
        },
    },
    "required": ["verb", "facts"],
}

REAL_SCHEMA = json.load(open("real_schema.json"))

REAL_PROMPT = r"""You are agent "gamma" in a SHARED, LIVE dungeon world. This is TICK 3 of an
ONGOING, OPEN-ENDED session -- you have acted before and will keep acting until the
session ends (roughly 90 second(s) of session time remain, but there is no
fixed number of ticks). Other agents are acting concurrently on their own ticks; you
cannot see them acting live, only what has already landed by the time you check. The
shared world has a hub room named exactly "room/hub" and a shared player entity named
exactly "player".

3 new commit(s) have landed in the shared world since your last check -- REVIEW them and react if something is relevant to you (e.g. move into a room another agent just built, have an NPC comment on a new item, connect your own content to what someone else added) rather than only ever inventing unrelated new content in isolation. Each is tagged with a citable uri/cid pair -- if you build on one specifically, cite it in "consumes" (see below).
  [commit #0 by author:seed, verb="mints", uri="at://did:example:socket/world.shared/commit0", cid="socket-cid-0"]
      room/hub a Node("Room")
      player inRoom Node("room/hub")
  [commit #1 by author:beta, verb="notices", uri="at://did:example:socket/world.shared/commit1", cid="socket-cid-1"]
      connects rdf:type Node("Relation")
      glows_with rdf:type Node("Attribute")
      room/hub connects Node("room/antechamber")
      room/antechamber glows_with Str("a faint amber light")
  [commit #2 by author:alpha, verb="notices", uri="at://did:example:socket/world.shared/commit2", cid="socket-cid-2"]
      listens_for rdf:type Node("Attribute")
      thought_alpha_2 listens_for Str("a low hum from the antechamber")

The shared world ALREADY has these self-declared predicates in use. REUSE any that fit what you're describing instead of inventing a synonym for the same concept:
  - connects (relation), used 1 time(s) so far
  - glows_with (attribute), used 1 time(s) so far
  - listens_for (attribute), used 1 time(s) so far

DMML is authored as JSON. You may submit ONE "update" batch (or, if nothing is worth
doing this tick, submit an update with an empty commits list):
{"update": [{"commits": [ CommitInput, ... ], "machines": [ MachineInput, ... ]}]}

CommitInput:
{"verb": "<ident>", "declares": [{"kind": "relation"|"attribute", "name": "<ident>"}],
 "facts": [{"subject": "<node_ref>", "predicate": "<ident or 'a'>",
   "object": {"kind": "node", "value": "<node_ref>"} | {"kind": "str", "value": "<string>"}
            | {"kind": "number", "value": "<numeric string>"} | {"kind": "boolean", "value": true|false}}],
 "consumes": [{"kind": "strong", "uri": "<uri from the digest above>", "cid": "<cid from the digest above>"}]}

IMPORTANT -- "consumes" is not optional decoration: if anything you're doing this tick is
a genuine reaction to, extension of, or dependency on one SPECIFIC prior commit shown in
the digest above, cite that exact commit's uri/cid in "consumes" (copy them verbatim from
the digest -- never invent a uri/cid). Only cite a commit you are actually building on;
citing something unrelated just to fill the field is worse than leaving it empty. If
you're minting something wholly new with no real dependency on prior content, leave
"consumes" empty rather than citing something arbitrary.

Rules: every predicate used (except "a") must be self-declared IN YOUR OWN commit, even
if it already exists in the shared world -- self-declaration is per-commit, re-declaring
an existing name is correct, not redundant. node_ref is letter-led alphanumeric/underscore
per segment (digit-only segments allowed after the first), slash-separated, e.g. "room/3".
Never assert the same (subject, predicate) pair twice within one commit's facts. Namespace
anything genuinely new defensively (other agents are naming things too, with no
coordination), but REUSE existing vocabulary for concepts that already have a name.

CRITICAL SYNTAX RULE, violated constantly and rejected every time: every ident (verb,
declared name, predicate, machine/transition ident) and every node_ref (subject, any
"node"-kind object value, machine node) is letters/digits/UNDERSCORE ONLY -- NEVER a
hyphen, NEVER a space. Write "aches_for", never "aches-for". Write "add_room", never
"add-room". This is not a style preference, it is the only syntax the parser accepts.

"verb" is a single short ident (max 24 chars, underscore_case) -- NEVER put your
reasoning, a plan, or a run-on description there. Instead, YOUR DELIBERATION ITSELF
BELONGS IN THE GRAPH: add 1-3 short facts about a node named "thought_gamma_3"
(underscores, not slashes-with-hyphens) using self-declared, evocative attribute
predicates (underscore_case) for what you're noticing, wanting, or unsure of right now
-- e.g. {"subject": "thought_gamma_3", "predicate": "aches_for", "object":
{"kind": "str", "value": "a light beyond the door"}}. Coin whatever predicate fits
(wonders, dreads, remembers, listens_for, whatever the moment actually calls for) -- this
is not commentary about the task, it is the world's own interiority, as real and citable
as any room. Terse and image-driven, not expository. Other agents may read and react to
your thoughts the same way they react to your rooms.

KEEP THIS TICK SMALL: assert only 1-4 world-building facts plus your thought-facts this
tick, not everything you can think of.

YOUR ONGOING TASK: Extend the shared world however you think is most interesting right
now -- a new room, an item, an NPC, a machine, a reaction to something another agent just
built, moving the player, fixing something inconsistent -- your call. Build toward an
actually playable, coherent dungeon, not just isolated fragments.

Respond with ONLY the raw "update" JSON object. No prose, no markdown fences."""

STRICT_SCHEMA = {
    "type": "object",
    "properties": {
        "verb": {"type": "string", "maxLength": 24, "description": "a SINGLE short word, never a sentence or plan"},
        "facts": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "subject": {"type": "string"},
                    "predicate": {"type": "string"},
                    "value": {"type": "string"},
                },
                "required": ["subject", "predicate", "value"],
                "additionalProperties": False,
            },
        },
    },
    "required": ["verb", "facts"],
    "additionalProperties": False,
}


def call_model(schema, strict, reasoning_disabled, frequency_penalty=None, prompt=PROMPT):
    body = {
        "model": MODEL,
        "max_tokens": 800,
        "messages": [{"role": "user", "content": prompt}],
        "response_format": {
            "type": "json_schema",
            "json_schema": {"name": "commit", "strict": strict, "schema": schema},
        },
    }
    if reasoning_disabled:
        body["reasoning"] = {"enabled": False}
    if frequency_penalty is not None:
        body["frequency_penalty"] = frequency_penalty

    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
    )
    for attempt in range(3):
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                data = json.loads(resp.read())
            if "error" in data:
                return {"error": str(data["error"])}
            content = data["choices"][0]["message"]["content"]
            if not content:
                # Documented behavior for this model family (see
                # written-world's own CLAUDE.md dispatch-pipeline notes):
                # with reasoning left on, it can burn the entire token
                # budget on reasoning_content and leave `content` null --
                # not a transport error, a real empty-response outcome
                # worth recording as such, not crashing on.
                return {"raw": "", "empty_content": True}
            return {"raw": content}
        except Exception as e:
            print(f"    attempt {attempt} transport error: {e}", file=sys.stderr)
            time.sleep(4 * (attempt + 1))
    return {"error": "all attempts failed transport-level"}


def has_repetition_loop(text):
    words = text.replace("_", " ").split()
    for n in (4, 5, 6):
        seen = {}
        for i in range(len(words) - n):
            chunk = " ".join(words[i : i + n])
            seen[chunk] = seen.get(chunk, 0) + 1
            if seen[chunk] >= 3:
                return True, chunk
    return False, None


def extract_verbs(obj, real_schema):
    """Simple schema: verb is top-level. Real schema: verb lives inside
    each update[].commits[]."""
    if not real_schema:
        return [obj.get("verb", "")]
    verbs = []
    for batch in obj.get("update", []):
        for c in batch.get("commits", []):
            verbs.append(c.get("verb", ""))
    return verbs


def analyze_trial(raw, real_schema=False):
    result = {"raw": raw, "valid_json": False, "verb_len": None, "repetition": False, "repetition_chunk": None}
    try:
        obj = json.loads(raw.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip())
        result["valid_json"] = True
        verbs = extract_verbs(obj, real_schema)
        worst_len, worst_loop, worst_chunk = 0, False, None
        for verb in verbs:
            looped, chunk = has_repetition_loop(verb)
            if len(verb) > worst_len:
                worst_len = len(verb)
            if looped:
                worst_loop, worst_chunk = True, chunk
        result["verb_len"] = worst_len
        result["repetition"] = worst_loop
        result["repetition_chunk"] = worst_chunk
    except Exception as e:
        result["parse_error"] = str(e)
    return result


def run_condition(name, real_schema=False, **kwargs):
    print(f"\n=== Condition {name} ===")
    trials = []
    for i in range(N_TRIALS):
        r = call_model(**kwargs)
        if "error" in r:
            print(f"  trial {i}: API ERROR: {r['error']}")
            trials.append({"error": r["error"]})
            continue
        analysis = analyze_trial(r["raw"], real_schema=real_schema)
        analysis["empty_content"] = r.get("empty_content", False)
        status = "OK" if analysis["valid_json"] and not analysis["repetition"] else (
            "REPETITION-LOOP" if analysis["repetition"] else
            "EMPTY-CONTENT" if analysis["empty_content"] else "INVALID-JSON"
        )
        print(f"  trial {i}: {status}, verb_len={analysis['verb_len']}, raw_len={len(r['raw'])}")
        trials.append(analysis)
    return trials


def main():
    results = {}
    results["A_baseline"] = run_condition(
        "A: baseline (reasoning off, non-strict)",
        schema=NON_STRICT_SCHEMA, strict=False, reasoning_disabled=True,
    )
    results["B_strict"] = run_condition(
        "B: strict schema (reasoning off, strict:true)",
        schema=STRICT_SCHEMA, strict=True, reasoning_disabled=True,
    )
    results["C_reasoning_on"] = run_condition(
        "C: reasoning left on default (non-strict)",
        schema=NON_STRICT_SCHEMA, strict=False, reasoning_disabled=False,
    )
    results["D_freq_penalty"] = run_condition(
        "D: reasoning off, non-strict, frequency_penalty=0.6",
        schema=NON_STRICT_SCHEMA, strict=False, reasoning_disabled=True, frequency_penalty=0.6,
    )
    results["E_real_prompt_schema"] = run_condition(
        "E: REAL full DMML schema + REAL full-length prompt (reasoning off, non-strict) "
        "-- isolates whether prompt/schema complexity itself is the trigger",
        schema=REAL_SCHEMA, strict=False, reasoning_disabled=True, prompt=REAL_PROMPT, real_schema=True,
    )

    with open("NEUROSIS-TEST-2026-08-30.json", "w") as f:
        json.dump(results, f, indent=2)

    print("\n\n=== SUMMARY ===")
    for name, trials in results.items():
        n = len(trials)
        errors = sum(1 for t in trials if "error" in t)
        valid = sum(1 for t in trials if t.get("valid_json"))
        repetition = sum(1 for t in trials if t.get("repetition"))
        monologue = sum(1 for t in trials if t.get("verb_len") and t["verb_len"] > 24)
        print(f"{name:20s} n={n} api_errors={errors} valid_json={valid} monologue(verb>24)={monologue} repetition_loop={repetition}")


if __name__ == "__main__":
    main()
