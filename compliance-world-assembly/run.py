#!/usr/bin/env python3
"""Multi-agent sequential world assembly: three different models, one
after another, each handed the REAL materialized snapshot of everything
authored so far (genesis + every prior accepted step) -- not a static
context, an actually-growing one. Each step's task depends on reading
the PREVIOUS step's real output out of the snapshot (a node name none
of the task text reveals), so this tests real cross-agent referential
coherence, not just "does it parse" or "does it reuse a name it was
handed outright."

Usage:
    OPENROUTER_API_KEY=... python3 run.py

Writes, for each step: results/stepN-<id>.dmml (the extracted candidate,
if any), results/snapshot-before-stepN.txt, and one combined
results/transcript.json + results/report.md at the end.
"""
import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
DMML_HS = HERE.parent / "dmml-hs"
SURFACE_PATH = DMML_HS / "SURFACE.md"
GENESIS_PATH = DMML_HS / "examples" / "hearth-genesis.dmml"
STEPS_PATH = HERE / "steps.json"
RESULTS_DIR = HERE / "results"

NEEDS_REASONING_NONE = {"moonshotai/kimi-k2.5"}
MAX_TOKENS = 12000

SYSTEM_PROMPT_TEMPLATE = """You are one of several agents collaboratively authoring content for a DMML \
(Desiring-Machine Markup Language) world, using its text authoring syntax. Below is the complete, \
current syntax reference.

--- SURFACE.md ---
{surface}
--- end SURFACE.md ---

Below is the CURRENT STATE of the world, materialized from every commit authored so far by you and \
other agents. Read it carefully -- it may contain node names and existing predicates you need to \
reference or reuse correctly.

--- CURRENT WORLD STATE ---
{world_state}
--- end CURRENT WORLD STATE ---

Respond with exactly ONE fenced code block containing a single DMML commit in the syntax above. You \
may add brief prose outside the fence, but the fence must contain nothing except the DMML commit \
text -- no JSON, no other format."""


def build_binaries() -> tuple[Path, Path]:
    render = DMML_HS / "render-snapshot"
    validate = DMML_HS / "validate-commit"
    for src, out in [("app/RenderSnapshot.hs", render), ("app/ValidateCommit.hs", validate)]:
        subprocess.run(
            ["ghc", "-isrc", "-iapp", "-O0", src, "-o", str(out)],
            cwd=DMML_HS, check=True, capture_output=True, text=True,
        )
    return render, validate


def render_snapshot(render_bin: Path, files: list[Path]) -> str:
    result = subprocess.run([str(render_bin)] + [str(f) for f in files], capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"render-snapshot failed on {files}: {result.stdout}")
    return result.stdout


def validate_commit(validate_bin: Path, path: Path) -> tuple[bool, str]:
    result = subprocess.run([str(validate_bin), str(path)], capture_output=True, text=True)
    return result.returncode == 0, result.stdout


def extract_fence(text: str) -> str | None:
    m = re.search(r"```[^\n]*\n(.*?)```", text, re.DOTALL)
    if not m:
        return None
    body = m.group(1).strip()
    return body or None


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

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    render_bin, validate_bin = build_binaries()
    surface = SURFACE_PATH.read_text()
    steps = json.loads(STEPS_PATH.read_text())

    chain_files = [GENESIS_PATH]
    transcript = []

    for i, step in enumerate(steps, start=1):
        sid, model, task = step["id"], step["model"], step["task"]
        print(f"=== step {i}: {model} / {sid} ===", file=sys.stderr)

        snapshot = render_snapshot(render_bin, chain_files)
        (RESULTS_DIR / f"snapshot-before-step{i}.txt").write_text(snapshot)

        system_prompt = SYSTEM_PROMPT_TEMPLATE.format(surface=surface, world_state=snapshot)
        try:
            reply = call_openrouter(api_key, model, system_prompt, task)
        except Exception as e:  # noqa: BLE001
            reply = f"[dispatch error: {e}]"
            print(f"  ERROR: {e}", file=sys.stderr)

        candidate = extract_fence(reply)
        record = {"step": i, "id": sid, "model": model, "reply": reply, "candidate": candidate}

        if candidate is None:
            record["outcome"] = "no-fence"
            print("  no fenced content found", file=sys.stderr)
        else:
            step_path = RESULTS_DIR / f"step{i}-{sid}.dmml"
            step_path.write_text(candidate)
            ok, err = validate_commit(validate_bin, step_path)
            if ok:
                record["outcome"] = "accepted"
                chain_files.append(step_path)
                print("  accepted, appended to chain", file=sys.stderr)
            else:
                record["outcome"] = "rejected"
                record["error"] = err
                print(f"  rejected:\n{err}", file=sys.stderr)

        transcript.append(record)
        time.sleep(1)

    final_snapshot = render_snapshot(render_bin, chain_files)
    (RESULTS_DIR / "snapshot-final.txt").write_text(final_snapshot)
    (RESULTS_DIR / "transcript.json").write_text(json.dumps(transcript, indent=2))

    lines = ["# World-assembly transcript", ""]
    for r in transcript:
        lines.append(f"## Step {r['step']}: {r['model']} / {r['id']} -- {r['outcome']}")
        lines.append("")
        lines.append("```")
        lines.append(r["candidate"] or "(no fenced content)")
        lines.append("```")
        if r["outcome"] == "rejected":
            lines.append("")
            lines.append("Rejected:")
            lines.append("```")
            lines.append(r["error"])
            lines.append("```")
        lines.append("")
    lines.append("## Final materialized snapshot")
    lines.append("")
    lines.append("```")
    lines.append(final_snapshot)
    lines.append("```")
    (RESULTS_DIR / "report.md").write_text("\n".join(lines))

    print(f"wrote {RESULTS_DIR / 'report.md'}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
