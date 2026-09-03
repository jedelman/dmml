#!/usr/bin/env python3
"""De-prose, agentic version: same refinement task as deprose.py (prose ->
valid, whole-tree-consistent DMML), but the model drives its own
draft/check/revise/commit loop through real tool calls instead of a
scripted Python retry loop deciding for it.

Jason, 2026-09-03, after reviewing deprose.py's results: "we're not
running agents yet, these are just completions... let's figure out how
to upgrade these lil buddies to proper agents." Concretely: "all of
these should be tools: check, commit."

What's actually different from deprose.py, precisely, since it's easy to
oversell this: deprose.py's driver notices a check failure and re-prompts
with the error -- the MODEL never sees that it failed, it just gets asked
again. Here the model calls `check` itself, reads real structured
feedback, and *decides* what to revise -- and decides for itself when to
call `commit` at all, including committing zero, one, or several times
for one prose passage. The driver's only remaining job is: expose the
two tools, execute them faithfully against the real binaries (never trust
the model's own claim that something is valid), and cap the round budget
so a stuck loop terminates instead of running forever.

`check` and `commit` share one implementation (`run_checks`) against the
real parser/checker/gate -- `commit` is not a rubber stamp on the model's
say-so, it re-runs the identical checks as a final gate and refuses to
write anything that fails them, same as `check` would report.

Usage:
    OPENROUTER_API_KEY=... python3 deprose_agent.py --source <prose.txt> \
        --world-dir <dir-of-existing-.dmml-files-or-empty> \
        --out-dir <dir-to-write-accepted-commits-into> \
        [--model moonshotai/kimi-k2.5] [--max-rounds 10]
"""
import argparse
import json
import os
import sys
import time
import urllib.request
from pathlib import Path

import run as base  # reuse dispatch/parsing helpers
import deprose as dp  # reuse check_self_declared/gate_candidate/world_files_in/builders

HERE = Path(__file__).resolve().parent
SURFACE_PATH = base.SURFACE_PATH

DEFAULT_MODEL = "moonshotai/kimi-k2.5"

AGENT_SYSTEM_PROMPT = """You are a "de-prose" operator: you refine plain prose into valid DMML \
(Desiring-Machine Markup Language) commits -- the same operation a smelter performs on ore. Treat \
prose as malformed DMML: your job is to extract what's already structurally there, not to invent \
content the text doesn't support.

--- SURFACE.md (commit grammar) ---
{surface}
--- end SURFACE.md ---

{world_context}

You have two tools, `check` and `commit`. Work iteratively:
1. Draft a candidate DMML commit from the prose.
2. Call `check` on it. Read the real result -- it tells you exactly what's wrong (parse error, \
undeclared predicate, or a whole-tree consistency conflict), not a guess.
3. Revise and check again as many times as you need. There's no penalty for checking a draft \
repeatedly.
4. Once a candidate is clean, call `commit` to deposit it. `commit` re-runs the same checks as a \
final gate -- if it still fails, fix it and try again, exactly like a failed `check`.
5. You may commit more than once for one prose passage, but only for genuinely separate content -- \
never as alternate drafts of the same facts. Most short passages need exactly one commit.
6. When you're done (everything extractable has been committed, or nothing in the prose is \
extractable at all), respond with a short plain-text summary and DO NOT call another tool. That \
ends the session.

Rules for what to extract, in order of importance:
1. Reuse an EXISTING node reference or predicate EXACTLY as given whenever the prose is plainly \
talking about something already in the world above -- do not mint a new node for something that \
already has one under a different name. Two different names for the same real thing must become \
ONE node reference, not two. This also applies to what YOU commit earlier in this same session.
2. Only declare a new predicate, or mint a new node, for something genuinely absent from the world \
above.
3. Extract only what the prose actually asserts -- do not infer facts the text doesn't support, and \
do not pad structure the text gives no basis for.
4. Identifier rule: a commit verb, predicate name, and each node-reference segment are single \
identifiers -- letters, digits, underscore only, no hyphens, no spaces.
5. Some prose doesn't literally assert anything but still plainly implies, suggests, or evokes \
something -- figurative, indirect, or deliberately ambiguous text (poetry, metaphor, allusion). \
Don't decline this as if it had no content, and don't flatten it into a bare assertion it doesn't \
make either. Extract it using a hedge predicate whose name carries the epistemic status honestly \
-- `seemsTo`, `couldImply`, `evokes`, or similar -- as a RELATION between the thing doing the \
evoking and a minted concept node for what's evoked (e.g. `wildGeese \\`evokes\\` belonging`, with \
`belonging` declared as its own node), not an attribute string. A concept node named for what it \
means, not what page it came from, is what lets a later passage's independent interpretation \
converge on the same node under Rule 1 instead of minting a near-duplicate.

--- PROSE TO REFINE ---
{prose}
--- end PROSE ---"""

TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "check",
            "description": (
                "Validate a candidate DMML commit against the real parser, the self-declaration "
                "checker, and the whole-tree consistency gate -- WITHOUT depositing it. Use this "
                "to test a draft before committing. Returns exactly what's wrong, if anything."
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "candidate": {
                        "type": "string",
                        "description": "The full DMML commit text to check, e.g. 'commit mints\\n  declare relation worksAt\\n\\n  mara :: a Person\\n  ...'",
                    }
                },
                "required": ["candidate"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "commit",
            "description": (
                "Deposit a candidate DMML commit into the world. Re-runs the identical checks as "
                "`check` first -- if it fails, nothing is written and you get the same kind of "
                "error back, so fix it and try again. On success this commit becomes part of the "
                "world for any further check/commit calls in this same session."
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "candidate": {
                        "type": "string",
                        "description": "The full DMML commit text to deposit.",
                    }
                },
                "required": ["candidate"],
            },
        },
    },
]


def call_openrouter_tools(api_key, model, reasoning_none, messages, tools):
    """Like run.call_openrouter, but passes tools/tool_choice and returns the
    raw assistant message (content + tool_calls), not just content -- the
    driver needs to see tool_calls to execute them and to know when the
    model has stopped calling tools at all."""
    payload = {
        "model": model,
        "max_tokens": base.MAX_TOKENS,
        "messages": messages,
        "tools": tools,
        "tool_choice": "auto",
    }
    if reasoning_none:
        payload["reasoning"] = {"effort": "none"}
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        method="POST",
    )
    start = time.monotonic()
    with urllib.request.urlopen(req, timeout=200) as resp:
        body = json.loads(resp.read().decode("utf-8"))
    elapsed = time.monotonic() - start
    if "choices" not in body:
        raise RuntimeError(f"unexpected response: {body}")
    msg = body["choices"][0]["message"]
    usage = body.get("usage") or {}
    return msg, usage, elapsed


def run_checks(candidate_text, validate_bin, check_declared_bin, retro_gate_bin, world_files):
    """The one real implementation shared by both tools: parse, self-declaration,
    whole-tree gate, in that order, stopping at the first failure (later checks
    are meaningless against text that doesn't even parse). Returns
    (ok: bool, report: dict) -- report always has 'stage' and 'detail' on
    failure, or {'stage': 'ok'} on success, so `commit`'s refusal reads exactly
    like `check`'s report -- same tool, same shape, no special-casing."""
    tmp = HERE / "results" / "_deprose_agent_candidate.dmml"
    tmp.parent.mkdir(parents=True, exist_ok=True)
    tmp.write_text(candidate_text)

    ok, output = base.validate_file(validate_bin, tmp)
    if not ok:
        return False, {"stage": "parse", "detail": output}

    kind = base.classify_file(validate_bin, tmp)
    if kind != "commit":
        return False, {"stage": "parse", "detail": "this parsed as a machine declaration, not a commit -- de-prose only extracts facts, never governance structure"}

    decl_ok, decl_detail = dp.check_self_declared(check_declared_bin, tmp, world_files)
    if not decl_ok:
        return False, {"stage": "self_declaration", "detail": decl_detail}

    gate_ok, gate_detail = dp.gate_candidate(retro_gate_bin, tmp, world_files)
    if not gate_ok:
        return False, {"stage": "whole_tree_gate", "detail": gate_detail}

    return True, {"stage": "ok"}


def deprose_agentic(api_key, model, prose_text, world_dir, out_dir, max_rounds, log, reasoning_none=True):
    validate_bin = base.DMML_HS / "validate-commit"
    render_bin = base.DMML_HS / "render-snapshot"
    retro_gate_bin = dp.build_retro_gate()
    check_declared_bin = dp.build_check_declared()

    world_files = dp.world_files_in(world_dir)
    if world_files:
        world_context = "--- CURRENT WORLD (reuse what's already here) ---\n" + base.render_snapshot(render_bin, world_files) + "--- end CURRENT WORLD ---"
    else:
        world_context = "The world is currently EMPTY -- there is nothing to reuse yet; every node and predicate you use will be new."

    surface_text = SURFACE_PATH.read_text()
    system_prompt = AGENT_SYSTEM_PROMPT.format(surface=surface_text, world_context=world_context, prose=prose_text)
    messages = [{"role": "user", "content": system_prompt}]

    out_dir.mkdir(parents=True, exist_ok=True)
    existing_indices = [
        int(p.stem.split("-", 1)[0])
        for p in out_dir.glob("*-agent.dmml")
        if p.stem.split("-", 1)[0].isdigit()
    ]
    next_index = max(existing_indices, default=0) + 1

    running_world_files = list(world_files)
    committed, check_calls = [], 0
    total_prompt_tokens = total_completion_tokens = total_reasoning_tokens = 0
    api_elapsed = 0.0
    wall_start = time.monotonic()

    for round_num in range(1, max_rounds + 1):
        log(f"[round {round_num}/{max_rounds}] dispatching...")
        msg, usage, elapsed = call_openrouter_tools(api_key, model, reasoning_none, messages, TOOLS)
        api_elapsed += elapsed
        total_prompt_tokens += usage.get("prompt_tokens", 0)
        total_completion_tokens += usage.get("completion_tokens", 0)
        # Only present when reasoning is actually on -- OpenRouter nests it
        # under completion_tokens_details, and reasoning tokens are already
        # counted inside completion_tokens, not additional to it.
        reasoning_tokens = (usage.get("completion_tokens_details") or {}).get("reasoning_tokens", 0)
        total_reasoning_tokens += reasoning_tokens
        reasoning_note = f" ({reasoning_tokens} reasoning)" if reasoning_tokens else ""
        log(f"  {elapsed:.1f}s, {usage.get('prompt_tokens', 0)}+{usage.get('completion_tokens', 0)} tokens{reasoning_note}")
        tool_calls = msg.get("tool_calls") or []

        if not tool_calls:
            final_text = (msg.get("content") or "").strip()
            log(f"[round {round_num}] no tool call -- model ended the session: {final_text[:200]!r}")
            break

        # Real OpenAI-style tool-calling shape: the assistant message carrying
        # tool_calls goes into history verbatim, then one 'tool' message per
        # call, addressed by tool_call_id -- the model needs both to keep the
        # conversation coherent on the next round.
        messages.append(msg)

        for tc in tool_calls:
            fn_name = tc["function"]["name"]
            try:
                args = json.loads(tc["function"]["arguments"])
            except json.JSONDecodeError as e:
                result = {"stage": "tool_call_malformed", "detail": f"arguments weren't valid JSON: {e}"}
                messages.append({"role": "tool", "tool_call_id": tc["id"], "content": json.dumps(result)})
                continue

            candidate = args.get("candidate", "")
            check_calls += 1
            ok, report = run_checks(candidate, validate_bin, check_declared_bin, retro_gate_bin, running_world_files)

            if fn_name == "check":
                log(f"  [check] {'OK' if ok else 'FAILED: ' + report['stage']}")
                messages.append({"role": "tool", "tool_call_id": tc["id"], "content": json.dumps({"valid": ok, **report})})

            elif fn_name == "commit":
                if not ok:
                    log(f"  [commit] REFUSED: {report['stage']} -- {str(report['detail'])[:120]}")
                    messages.append({"role": "tool", "tool_call_id": tc["id"], "content": json.dumps({"deposited": False, **report})})
                else:
                    path = out_dir / f"{next_index:03d}-agent.dmml"
                    path.write_text(candidate)
                    running_world_files.append(path)
                    committed.append(str(path))
                    log(f"  [commit] deposited -> {path}")
                    messages.append({"role": "tool", "tool_call_id": tc["id"], "content": json.dumps({"deposited": True, "path": str(path)})})
                    next_index += 1

            else:
                messages.append({"role": "tool", "tool_call_id": tc["id"], "content": json.dumps({"error": f"unknown tool {fn_name!r}"})})
    else:
        log(f"[deprose-agent] hit max_rounds ({max_rounds}) without the model ending the session on its own")

    wall_elapsed = time.monotonic() - wall_start
    return {
        "committed": committed,
        "check_calls": check_calls,
        "rounds": round_num,
        "prompt_tokens": total_prompt_tokens,
        "completion_tokens": total_completion_tokens,
        "reasoning_tokens": total_reasoning_tokens,
        "total_tokens": total_prompt_tokens + total_completion_tokens,
        "api_elapsed": api_elapsed,
        "wall_elapsed": wall_elapsed,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", required=True, type=Path, help="plain text file to de-prose")
    ap.add_argument("--world-dir", required=True, type=Path, help="existing .dmml world (may not exist -- empty world)")
    ap.add_argument("--out-dir", required=True, type=Path, help="where committed commits are written")
    ap.add_argument("--model", default=DEFAULT_MODEL)
    ap.add_argument("--max-rounds", type=int, default=10)
    ap.add_argument(
        "--reasoning",
        action="store_true",
        help="Let the model reason (default: reasoning off, same as deprose.py's dispatch convention). "
        "Only meaningful for a model that supports disabling reasoning in the first place.",
    )
    args = ap.parse_args()

    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        print("OPENROUTER_API_KEY not set", file=sys.stderr)
        return 1

    print("[deprose-agent] building validate-commit, render-snapshot...", flush=True)
    base.build_binaries()

    prose_text = args.source.read_text()

    def log(msg):
        print(msg, flush=True)

    result = deprose_agentic(
        api_key, args.model, prose_text, args.world_dir, args.out_dir, args.max_rounds, log,
        reasoning_none=not args.reasoning,
    )
    print()
    print(
        f"[deprose-agent] done: {len(result['committed'])} committed, {result['check_calls']} check/commit calls, "
        f"{result['rounds']} rounds"
    )
    reasoning_note = f" ({result['reasoning_tokens']} reasoning)" if result["reasoning_tokens"] else ""
    print(
        f"[deprose-agent] stats: {result['prompt_tokens']}+{result['completion_tokens']}="
        f"{result['total_tokens']} tokens{reasoning_note}, {result['api_elapsed']:.1f}s API time, "
        f"{result['wall_elapsed']:.1f}s wall time ({result['wall_elapsed'] - result['api_elapsed']:.1f}s local "
        f"check/build/gate time)"
    )
    for p in result["committed"]:
        print(f"  committed: {p}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
