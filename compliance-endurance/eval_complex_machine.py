#!/usr/bin/env python3
"""Eval: ask `openrouter/free` (real per-call model rotation, whatever's
currently free) to author ONE complex DMML machine against a real seed
world, exercising the generalized `Effect` grammar SURFACE.md documents
as of 2026-09-04 (general-form assert/retract, node-minting via a
transition parameter -- see dev-journal/2026-09-03-phase-2-3-effect-
generalization-and-firing.md and dev-journal/2026-09-04-retract-
provenance-fix.md for what actually got built).

The world given is real, reused rather than invented: dmml-hs/examples/
endurance/seed-genesis.dmml plus its own real 11-machine set -- the
same seed run.py's endurance harness already uses, per this project's
own DMML-first/reuse-real-evidence discipline.

Not the full 4-agent divergence/thrash endurance harness (run.py) --
that tests peer-to-peer risk across many stacked rounds. This is
narrower and single-shot: N independent openrouter/free completions,
each asked once, each checked against the real parser/self-declaration
pipeline -- not eyeballed, not trusted on the model's own say-so.

Usage:
    OPENROUTER_API_KEY=... python3 eval_complex_machine.py [--n N]
"""
import argparse
import http.client
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
DMML_HS = HERE.parent / "dmml-hs"
SURFACE_PATH = DMML_HS / "SURFACE.md"
SEED_DIR = DMML_HS / "examples" / "endurance"
SEED_GENESIS = SEED_DIR / "seed-genesis.dmml"
MACHINE_FILES = sorted((SEED_DIR / "machines").glob("*.dmml"))
RESULTS_DIR = HERE / "results" / "complex-machine-eval"
CANDIDATES_DIR = RESULTS_DIR / "candidates"

MODEL = "openrouter/free"
MAX_TOKENS = 3000

SYSTEM_PROMPT = """You are an agent authoring content for a shared, append-only DMML \
(Desiring-Machine Markup Language) world, using its text authoring syntax.

--- SURFACE.md (commit and machine grammar) ---
{surface}
--- end SURFACE.md ---

--- THE WORLD (a real, already-established seed -- reuse its real node names, \
declared predicates, and existing machines exactly as given; don't invent a new \
predicate for something already declared) ---
{seed}

--- REAL MACHINES ALREADY GOVERNING PARTS OF THIS WORLD ---
{machines}
--- end THE WORLD ---

Your task: author ONE new, genuinely COMPLEX DMML machine for a node somewhere in \
this world (an existing node, or a new one you mint). "Complex" means: multiple \
states, multiple transitions, and effects that do more than change the machine's \
own bare state -- use the general effect form (`assert <term> \\`<predicate>\\` \
<value>` / `retract <term> \\`<predicate>\\``) to have a transition assert or \
retract facts about OTHER nodes, not just itself, and consider minting a brand-new \
node by naming a fresh transition parameter as an effect's subject. A machine that \
only flips its own state through two or three values is NOT what's being asked for \
here -- reach for the richer grammar.

Respond with exactly ONE fenced code block containing a single DMML machine. You \
may add brief prose outside the fence explaining your design."""

FENCE_RE = re.compile(r"```(?:\w+)?\n(.*?)```", re.DOTALL)
GENERAL_ASSERT_RE = re.compile(r"^\s*assert\s+\S.*`", re.MULTILINE)
GENERAL_RETRACT_RE = re.compile(r"^\s*retract\s+\S.*`", re.MULTILINE)
STATES_BLOCK_RE = re.compile(r"^\s*states\s*\n((?:\s*\S+\s*\n)*)", re.MULTILINE)
TRANSITION_RE = re.compile(r"^\s*transition\s+\w+\(", re.MULTILINE)


def sh(*args, **kwargs):
    import subprocess

    return subprocess.run(list(args), capture_output=True, text=True, **kwargs)


def build_binaries():
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    CANDIDATES_DIR.mkdir(parents=True, exist_ok=True)
    validate = DMML_HS / "validate-commit"
    check_declared = DMML_HS / "check-declared"
    for src, out in [
        ("app/ValidateCommit.hs", validate),
        ("app/CheckDeclared.hs", check_declared),
    ]:
        r = sh("ghc", "-isrc", "-iapp", "-O0", src, "-o", str(out), cwd=str(DMML_HS))
        if r.returncode != 0:
            print(r.stdout, r.stderr, file=sys.stderr)
            raise RuntimeError(f"build failed: {src}")
    return validate, check_declared


def _post_chat_completion(api_key, payload):
    req = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=200) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except (http.client.IncompleteRead, ConnectionError, TimeoutError, urllib.error.URLError) as e:
        if isinstance(e, urllib.error.HTTPError):
            raise
        time.sleep(2)
        with urllib.request.urlopen(req, timeout=200) as resp:
            return json.loads(resp.read().decode("utf-8"))


def call_openrouter_free(api_key, messages):
    """Real gap this project already found and fixed once (deprose_agent.py,
    2026-09-03): `openrouter/free` picks a DIFFERENT underlying model per
    call, so a fixed reasoning:none payload intermittently 400s ("Reasoning
    is mandatory for this endpoint and cannot be disabled") whenever the
    router lands on a mandatory-reasoning free model that call. Retries once
    without the reasoning param specifically for that error. Returns
    (content, model_used, usage, elapsed)."""
    payload = {"model": MODEL, "max_tokens": MAX_TOKENS, "messages": messages, "reasoning": {"effort": "none"}}

    def is_mandatory_reasoning_error(err_text):
        return "mandatory" in err_text.lower() and "reasoning" in err_text.lower()

    start = time.monotonic()
    try:
        body = _post_chat_completion(api_key, payload)
        if "error" in body and "reasoning" in payload and is_mandatory_reasoning_error(json.dumps(body["error"])):
            payload.pop("reasoning")
            body = _post_chat_completion(api_key, payload)
    except urllib.error.HTTPError as e:
        err_body = e.read().decode("utf-8", errors="replace")
        if e.code == 400 and "reasoning" in payload and is_mandatory_reasoning_error(err_body):
            payload.pop("reasoning")
            body = _post_chat_completion(api_key, payload)
        else:
            raise RuntimeError(f"HTTP {e.code}: {err_body}") from e
    elapsed = time.monotonic() - start

    if "choices" not in body:
        raise RuntimeError(f"unexpected response: {body}")
    content = body["choices"][0]["message"].get("content") or ""
    model_used = body.get("model", "unknown")
    usage = body.get("usage") or {}
    return content, model_used, usage, elapsed


def extract_fence(text):
    m = FENCE_RE.search(text)
    return m.group(1).strip() if m else text.strip()


def analyze(dmml_text):
    states_match = STATES_BLOCK_RE.search(dmml_text)
    n_states = len([ln for ln in states_match.group(1).splitlines() if ln.strip()]) if states_match else 0
    return {
        "n_states": n_states,
        "n_transitions": len(TRANSITION_RE.findall(dmml_text)),
        "uses_general_assert": bool(GENERAL_ASSERT_RE.search(dmml_text)),
        "uses_general_retract": bool(GENERAL_RETRACT_RE.search(dmml_text)),
    }


def run_trial(api_key, validate_bin, check_declared_bin, system_prompt, index):
    messages = [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": "Author ONE complex DMML machine now."},
    ]
    try:
        content, model_used, usage, elapsed = call_openrouter_free(api_key, messages)
    except Exception as e:  # real network/API failure, not a content problem
        return {"index": index, "ok": False, "stage": "api_call", "error": str(e)}

    dmml = extract_fence(content)
    candidate_path = CANDIDATES_DIR / f"trial-{index:02d}.dmml"
    candidate_path.write_text(dmml)

    r = sh(str(validate_bin), str(candidate_path))
    if r.returncode != 0:
        return {
            "index": index,
            "ok": False,
            "stage": "parse",
            "model_used": model_used,
            "usage": usage,
            "elapsed_s": round(elapsed, 1),
            "error": (r.stdout + r.stderr).strip(),
            "raw_response": content,
        }

    world_files = [SEED_GENESIS] + MACHINE_FILES + [candidate_path]
    r2 = sh(str(check_declared_bin), *[str(f) for f in world_files])
    declared_ok = r2.returncode == 0

    stats = analyze(dmml)
    return {
        "index": index,
        "ok": True,
        "model_used": model_used,
        "usage": usage,
        "elapsed_s": round(elapsed, 1),
        "declared_ok": declared_ok,
        "declared_output": (r2.stdout + r2.stderr).strip(),
        "dmml_path": str(candidate_path.relative_to(HERE)),
        **stats,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=8)
    args = ap.parse_args()

    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        print("OPENROUTER_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    validate_bin, check_declared_bin = build_binaries()

    surface = SURFACE_PATH.read_text()
    seed = SEED_GENESIS.read_text()
    machines_text = "\n\n".join(f.read_text() for f in MACHINE_FILES)
    system_prompt = SYSTEM_PROMPT.format(surface=surface, seed=seed, machines=machines_text)

    results = []
    for i in range(args.n):
        print(f"=== trial {i + 1}/{args.n} ===", file=sys.stderr)
        res = run_trial(api_key, validate_bin, check_declared_bin, system_prompt, i)
        results.append(res)
        tag = "OK" if res.get("ok") else "FAIL"
        model = res.get("model_used", "?")
        print(f"  [{tag}] model={model} stage={res.get('stage', 'checked')}", file=sys.stderr)

    n_ok = sum(1 for r in results if r["ok"])
    n_parsed = sum(1 for r in results if r["ok"])
    n_declared_ok = sum(1 for r in results if r.get("declared_ok"))
    n_general_assert = sum(1 for r in results if r.get("uses_general_assert"))
    n_general_retract = sum(1 for r in results if r.get("uses_general_retract"))
    models_seen = sorted({r.get("model_used") for r in results if r.get("model_used")})

    summary = {
        "n_trials": args.n,
        "n_parsed": n_parsed,
        "n_declared_ok": n_declared_ok,
        "n_using_general_assert": n_general_assert,
        "n_using_general_retract": n_general_retract,
        "models_seen": models_seen,
        "results": results,
    }
    (RESULTS_DIR / "report.json").write_text(json.dumps(summary, indent=2))
    print(f"\n{n_parsed}/{args.n} parsed, {n_declared_ok}/{args.n} fully self-declared, "
          f"{n_general_assert}/{args.n} used the general assert form, "
          f"{n_general_retract}/{args.n} used the general retract form", file=sys.stderr)
    print(f"models seen: {models_seen}", file=sys.stderr)


if __name__ == "__main__":
    main()
