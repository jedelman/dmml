#!/usr/bin/env python3
"""Stage 2: one batched Jev call, one `noul` per Stage 1 candidate.

This is the nose, not the decision engine: it asks, independently and
in parallel, "does this specific comparison look worth a reviewer's
attention" -- never "should this block the PR," never a `choice`
between candidates. Output is a ranked list for a human (or a required-
status-free CI annotation) to look at.

The request/response shape and the ranking math are carried over
VERIFIED, not reconstructed from memory, from this repo's own real Jev
integration: dmml-hs/examples/jev-driver-demo/driver.py on branch
claude/recombinant-cannon-opr929 (`call_jev_batch`'s `noul` question
type, `interest_probability`'s z-score+sigmoid ranking, `read_interest`'s
answer parsing). That code is real and has been run live against Jev
dozens of times; this script's request-building and response-parsing
follow it line for line so Stage 2 doesn't reinvent a schema on a guess.

One thing deliberately NOT carried over: that branch's prior (mean
0.412, sd 0.086, weight 12) is a measured fact about ITS domain -- 122
live noul scores over a fantasy-narrative interest question. It says
nothing about what a code-review noul score distribution looks like.
This script's prior defaults to weight 0 (no prior at all) until
code-nose has minted its own real baseline the same way that project
measured its own -- see baseline.py.

Usage:
    python3 stage2_score.py --candidates candidates.ndjson --dry-run
    python3 stage2_score.py --candidates candidates.ndjson \
        --api-key "$TYPESAFE_API_KEY" --baseline-path nose-baseline.json

Output (stdout): ranked ndjson, one candidate per line, each carrying
its `noul` score and (if a baseline exists) a `probability` -- and
a bare `{"score": ...}` companion stream isn't emitted here on purpose:
pipe stdout through `--scores-only` to feed baseline.py --mint directly.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

JEV_ENDPOINT = "https://api.typesafe.ai/v1/systemone"

NOSE_INSTRUCTIONS = (
    "A mechanical scan found two or more pieces of code that share a "
    "name, a job, or a template shape, and flagged a specific place "
    "where they disagree -- or a comment making a claim a nearby line "
    "of code doesn't match. You are not being asked whether this is a "
    "bug. You are being asked whether it looks like something a real "
    "reviewer would actually want to look at, as opposed to expected, "
    "deliberate variation, or noise from the scan itself."
)

NOUL_CRITERIA = {
    "true": "Yes -- this looks like something worth a reviewer's attention.",
    "false": "No -- expected, deliberate, or not worth a look.",
}


def candidate_id(candidate: dict) -> str:
    """Stable id from content, not position -- so the same candidate
    gets the same id across re-runs (needed if a future batch wants to
    avoid re-asking about something already scored recently)."""
    blob = json.dumps(candidate, sort_keys=True).encode()
    return "nose_" + hashlib.sha1(blob).hexdigest()[:12]


def build_state_summary(candidates: list[dict], repo_label: str) -> str:
    from collections import Counter
    kinds = Counter(c["cluster"] for c in candidates)
    breakdown = ", ".join(f"{n} cluster-{k}" for k, n in sorted(kinds.items()))
    return (
        f"Reviewing {repo_label}. A mechanical Stage 1 scan produced "
        f"{len(candidates)} candidate(s) this batch: {breakdown}. Each "
        f"question below names one specific, already-identified "
        f"disagreement -- not a request to re-scan the codebase."
    )


def call_jev_noul_batch(api_key: str, model: str, state_summary: str, questions: dict) -> dict:
    body = {"state": state_summary, "model": model, "questions": questions}
    t0 = time.monotonic()
    req = urllib.request.Request(
        JEV_ENDPOINT,
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            out = json.loads(resp.read())
    except urllib.error.HTTPError as e:
        print(f"fatal: Jev call failed: {e.code} {e.read().decode()}", file=sys.stderr)
        sys.exit(3)
    elapsed = time.monotonic() - t0
    answers = out.get("answers") or {}
    nouls = [a["noul"] for a in answers.values()
             if isinstance(a, dict) and isinstance(a.get("noul"), (int, float))]
    print(
        f"# stage2: {len(questions)} question(s) in {round(elapsed, 3)}s, "
        f"{(out.get('usage') or {}).get('input_tokens')}->"
        f"{(out.get('usage') or {}).get('output_tokens')} tokens, "
        f"noul decisiveness "
        f"{round(sum(abs(n - 0.5) * 2 for n in nouls) / len(nouls), 3) if nouls else None}",
        file=sys.stderr,
    )
    return out


def read_noul(answers: dict, key: str) -> float | None:
    a = answers.get(key)
    if not isinstance(a, dict):
        return None
    v = a.get("noul")
    return float(v) if isinstance(v, (int, float)) else None


def dry_run_noul(key: str) -> float:
    """Deterministic stand-in for a live noul score, so the pipeline can
    be rehearsed with no API key. NOT a prediction of what Jev will say
    -- same honesty rule the DMML dry-run chooser follows for its own
    stand-in. A stable hash of the key, not randomness, so a rehearsal
    is reproducible."""
    h = hashlib.sha256(key.encode()).digest()
    return int.from_bytes(h[:4], "big") / 0xFFFFFFFF


def load_baseline(path: Path) -> tuple[int, float, float]:
    if not path.exists():
        return 0, 0.0, 0.0
    data = json.loads(path.read_text())
    return data.get("n", 0), data.get("mean", 0.0), data.get("sd", 0.0)


def probability(
    score: float, n: int, mean: float, sd: float,
    prior_mean: float, prior_sd: float, prior_weight: float, temperature: float,
) -> float | None:
    """z-score against the baseline blended with a stated prior, then a
    sigmoid -- identical mechanism to interest_probability in driver.py.
    Returns None (unranked) only when there is neither a real baseline
    nor a configured prior to standardize against; that's the honest
    cold-start state, not a value to paper over with a guess.
    """
    total_weight = n + prior_weight
    if total_weight <= 0:
        return None
    blended_mean = (n * mean + prior_weight * prior_mean) / total_weight
    blended_sd = math.sqrt(
        (n * sd**2 + prior_weight * prior_sd**2) / total_weight
    ) if total_weight > 0 else 0.0
    z = (score - blended_mean) / max(blended_sd, 1e-6)
    return 1.0 / (1.0 + math.exp(-z / max(temperature, 1e-6)))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--candidates", type=Path, default=None, help="Stage 1 ndjson (default: stdin)")
    ap.add_argument("--repo-label", default="this repository")
    ap.add_argument("--model", default="jev-1.13.0")
    ap.add_argument("--api-key", default=os.environ.get("TYPESAFE_API_KEY"))
    ap.add_argument("--max-questions", type=int, default=40)
    ap.add_argument("--baseline-path", type=Path, default=Path(__file__).parent / "nose-baseline.json")
    ap.add_argument("--prior-mean", type=float, default=0.5)
    ap.add_argument("--prior-sd", type=float, default=0.25)
    ap.add_argument("--prior-weight", type=float, default=0.0,
                     help="0 = no prior; ranking is unranked (raw noul order) until a real baseline exists")
    ap.add_argument("--temperature", type=float, default=1.0)
    ap.add_argument("--dry-run", action="store_true", help="skip the live Jev call; use a deterministic stand-in score")
    ap.add_argument("--scores-only", action="store_true",
                     help="emit {\"score\": ...} per line instead of full candidate rows -- feeds baseline.py directly")
    args = ap.parse_args()

    text = args.candidates.read_text() if args.candidates else sys.stdin.read()
    candidates = [json.loads(l) for l in text.splitlines() if l.strip()]
    if not candidates:
        print("# stage2: no candidates -- nothing to score", file=sys.stderr)
        return

    truncated = candidates[: args.max_questions]
    overflow = len(candidates) - len(truncated)
    if overflow > 0:
        print(
            f"# stage2: {overflow} candidate(s) not asked about this batch "
            f"(max-questions={args.max_questions}) -- pick them up on a "
            f"future run rather than pretending they were scored",
            file=sys.stderr,
        )

    ids = [candidate_id(c) for c in truncated]
    state_summary = build_state_summary(truncated, args.repo_label)
    questions = {
        cid: {
            "type": "noul",
            "instructions": NOSE_INSTRUCTIONS + "\n\n" + c["claim"],
            "criteria": NOUL_CRITERIA,
        }
        for cid, c in zip(ids, truncated)
    }

    if args.dry_run:
        answers = {cid: {"noul": dry_run_noul(cid)} for cid in ids}
    else:
        if not args.api_key:
            print("fatal: no Jev API key. Set TYPESAFE_API_KEY or pass --api-key, or use --dry-run.", file=sys.stderr)
            sys.exit(2)
        out = call_jev_noul_batch(args.api_key, args.model, state_summary, questions)
        answers = out.get("answers") or {}

    n, mean, sd = load_baseline(args.baseline_path)
    if n == 0 and args.prior_weight == 0:
        print("# stage2: no baseline yet and no prior configured -- ranking by raw noul score", file=sys.stderr)

    scored = []
    for cid, c in zip(ids, truncated):
        score = read_noul(answers, cid)
        if score is None:
            print(f"# stage2: no noul answer for {cid} -- skipping", file=sys.stderr)
            continue
        p = probability(score, n, mean, sd, args.prior_mean, args.prior_sd, args.prior_weight, args.temperature)
        scored.append((score, p, cid, c))

    scored.sort(key=lambda t: t[1] if t[1] is not None else t[0], reverse=True)

    for rank, (score, p, cid, c) in enumerate(scored, start=1):
        if args.scores_only:
            print(json.dumps({"score": score}))
            continue
        row = dict(c)
        row["id"] = cid
        row["score"] = score
        row["probability"] = p
        row["rank"] = rank
        print(json.dumps(row))


if __name__ == "__main__":
    main()
