#!/usr/bin/env python3
"""De-prose: refines plain prose into valid, whole-tree-consistent DMML
commits, into an existing (possibly empty) world. Jason, 2026-09-03:
"a de-prose operator should take plain prose and a dmml world (which
may be empty) and extract dmml from it into the world... it may be
worth it to consider prose simply as malformed dmml."

Taken literally, not just as a metaphor: this is NOT a single LLM call
treated as a black box. It's the same refine-until-valid loop this
project already built for retroconsistency, with a different source of
impurity (unstructured text instead of a guard's implied gap):

  1. Ore extraction  -- one LLM call, given the world's current
     declared vocabulary (so it can REUSE existing nodes/predicates
     instead of minting duplicates -- the entity-resolution/dedup
     question folds into extraction itself, not a separate pass) plus
     the raw prose, producing candidate commit text.
  2. Smelting        -- a bounded parse-repair loop against the REAL
     parser (validate-commit), feeding the exact parser error back on
     failure -- same RETRY_PROMPT idiom run.py's own authoring loop
     already uses, reused here rather than reinvented.
  3. Assay            -- once a candidate parses, gate it against the
     WHOLE current world (retro-gate, built for retroconsistency) --
     a de-prosed fact is exactly as external to the deterministic core
     as a retro-implied one, and needs exactly the same whole-tree
     consistency check before being trusted.
  4. Deposit          -- only a candidate that clears both 2 and 3
     gets written into the world; a rejected candidate is reported,
     not silently dropped or forced in.

Usage:
    OPENROUTER_API_KEY=... python3 deprose.py --source <prose.txt> \
        --world-dir <dir-of-existing-.dmml-files-or-empty> \
        --out-dir <dir-to-write-accepted-commits-into> \
        [--model moonshotai/kimi-k2.5] [--max-repair 3]
"""
import argparse
import os
import sys
from pathlib import Path

import run as base  # reuse dispatch/parsing helpers

HERE = Path(__file__).resolve().parent
DMML_HS = base.DMML_HS
SURFACE_PATH = base.SURFACE_PATH

DEFAULT_MODEL = "moonshotai/kimi-k2.5"

DEPROSE_SYSTEM_PROMPT = """You are a "de-prose" operator: you refine plain prose into valid DMML \
(Desiring-Machine Markup Language) commits -- the same operation a smelter performs on ore. Treat \
prose as malformed DMML: your job is to extract what's already structurally there, not to invent \
content the text doesn't support.

--- SURFACE.md (commit grammar) ---
{surface}
--- end SURFACE.md ---

{world_context}

Rules, in order of importance:
1. Reuse an EXISTING node reference or predicate EXACTLY as given whenever the prose is plainly \
talking about something already in the world above -- do not mint a new node for something that \
already has one under a different name. Two different names for the same real thing must become \
ONE node reference, not two.
2. Only declare a new predicate, or mint a new node, for something genuinely absent from the world \
above.
3. Extract only what the prose actually asserts -- do not infer facts the text doesn't support, and \
do not pad structure the text gives no basis for.
4. Identifier rule: a commit verb, predicate name, and each node-reference segment are single \
identifiers -- letters, digits, underscore only, no hyphens, no spaces.

Respond with exactly ONE fenced code block containing a single DMML commit that captures \
everything the prose asserts (a commit can hold many facts -- do not split one coherent passage \
into several near-duplicate commits). Only use more than one fenced block if the prose plainly \
describes multiple separate, unrelated scenes or events that don't belong in the same commit -- \
never as alternate phrasings or drafts of the same content. Never a machine declaration -- this \
operator extracts facts, not governance structure; if the prose describes a real state machine, \
note that in plain prose outside the fence instead. If the prose supports no real DMML content at \
all, respond with the single word NONE and nothing else.

--- PROSE TO REFINE ---
{prose}
--- end PROSE ---"""

REPAIR_PROMPT = """That candidate did not parse as valid DMML. Real parser error:

{error}

Candidate that failed:
{candidate}

Respond with exactly ONE fenced code block containing a single, corrected DMML commit that fixes \
this specific error while still representing the same real content from the original prose."""

DECLARE_REPAIR_PROMPT = """That candidate parsed, but used predicates that were never declared -- \
DMML requires every predicate to be introduced with a `declare relation`/`declare attribute` line \
before use. Real checker output:

{error}

Candidate that failed:
{candidate}

Respond with exactly ONE fenced code block containing a single, corrected DMML commit that adds \
the missing `declare` line(s) for every predicate actually used, while still representing the \
same real content from the original prose. Do not remove any fact to avoid declaring it."""


def extract_fences(text):
    """All fenced blocks, not just the first -- unlike run.extract_fence,
    de-prosing one prose passage can legitimately yield several separate
    commits."""
    import re

    return [m.strip() for m in re.findall(r"```[^\n]*\n(.*?)```", text, re.DOTALL) if m.strip()]


def build_retro_gate():
    retro_gate = DMML_HS / "retro-gate"
    r = base.sh("ghc", "-isrc", "-iapp", "-O0", "app/RetroGate.hs", "-o", str(retro_gate), cwd=str(DMML_HS))
    if r.returncode != 0:
        print(r.stdout, r.stderr, file=sys.stderr)
        raise RuntimeError("build failed: app/RetroGate.hs")
    return retro_gate


def build_check_declared():
    check_declared = DMML_HS / "check-declared"
    r = base.sh("ghc", "-isrc", "-iapp", "-O0", "app/CheckDeclared.hs", "-o", str(check_declared), cwd=str(DMML_HS))
    if r.returncode != 0:
        print(r.stdout, r.stderr, file=sys.stderr)
        raise RuntimeError("build failed: app/CheckDeclared.hs")
    return check_declared


def check_self_declared(check_declared_bin, candidate_path, world_files):
    """True (ok), or (False, explanation). Runs even against an empty
    world -- unlike the whole-tree gate, self-declaration is a property
    of the candidate plus its OWN declare lines, not something that
    needs prior world content to check."""
    r = base.sh(str(check_declared_bin), str(candidate_path), *[str(f) for f in world_files])
    if r.returncode == 0:
        return True, None
    return False, r.stdout


def world_files_in(world_dir):
    if not world_dir.exists():
        return []
    return sorted(world_dir.glob("*.dmml"))


def gate_candidate(retro_gate_bin, candidate_path, world_files):
    """True (ok), or (False, explanation) -- trivially OK against an
    empty world, since there's nothing yet to break."""
    if not world_files:
        return True, None
    r = base.sh(str(retro_gate_bin), str(candidate_path), *[str(f) for f in world_files])
    if r.returncode == 0:
        return True, None
    return False, r.stdout


def repair_loop(api_key, model, validate_bin, candidate_text, max_repair, log):
    """Refines one candidate against the real parser, up to max_repair
    times. Returns (True, final_text) or (False, last_error)."""
    tmp = HERE / "results" / "_deprose_candidate.dmml"
    tmp.parent.mkdir(parents=True, exist_ok=True)
    text = candidate_text
    for attempt in range(max_repair + 1):
        tmp.write_text(text)
        ok, output = base.validate_file(validate_bin, tmp)
        if ok:
            return True, text
        if attempt == max_repair:
            return False, output
        log(f"    [smelting] parse error, repair attempt {attempt + 1}/{max_repair}: {output.strip()[:120]}")
        content, _usage, _elapsed = base.call_openrouter(
            api_key, model, True,
            [{"role": "user", "content": REPAIR_PROMPT.format(error=output, candidate=text)}],
        )
        fences = extract_fences(content)
        if not fences:
            return False, f"repair call produced no fenced block:\n{content}"
        text = fences[0]
    return False, "unreachable"


def declare_repair_loop(api_key, model, validate_bin, check_declared_bin, candidate_text, world_files, max_repair, log):
    """Mirrors repair_loop, but for self-declaration gaps rather than parse
    errors -- a missing `declare` line is a mechanical omission, not an
    epistemic conflict (unlike a gate failure), so it gets the same
    smelting-style retry budget parse errors get instead of an outright
    reject. Each repaired candidate is re-validated by the real parser too,
    since a declare-focused edit could itself introduce a syntax error.
    Returns (True, final_text) or (False, last_error)."""
    tmp = HERE / "results" / "_deprose_candidate.dmml"
    text = candidate_text
    for attempt in range(max_repair + 1):
        tmp.write_text(text)
        decl_ok, decl_detail = check_self_declared(check_declared_bin, tmp, world_files)
        if decl_ok:
            return True, text
        if attempt == max_repair:
            return False, decl_detail
        log(f"    [smelting: declare-repair] undeclared predicate(s), repair attempt {attempt + 1}/{max_repair}: {decl_detail.strip()[:120]}")
        content, _usage, _elapsed = base.call_openrouter(
            api_key, model, True,
            [{"role": "user", "content": DECLARE_REPAIR_PROMPT.format(error=decl_detail, candidate=text)}],
        )
        fences = extract_fences(content)
        if not fences:
            return False, f"declare-repair call produced no fenced block:\n{content}"
        candidate = fences[0]
        tmp.write_text(candidate)
        ok, output = base.validate_file(validate_bin, tmp)
        if ok:
            text = candidate
            continue
        # A declare-focused edit can itself introduce a syntax error (real
        # case hit here: the model moved `declare` lines outside the
        # `commit` block). Don't abort the whole loop over that -- spend
        # one parse-repair pass fixing it, same machinery already used for
        # genuine parse failures, and keep the remaining declare-repair
        # budget rather than giving up on the first collision.
        log(f"    [smelting: declare-repair] repair introduced a parse error, patching: {output.strip()[:120]}")
        parse_ok, parse_fixed_or_err = repair_loop(api_key, model, validate_bin, candidate, 1, log)
        if parse_ok:
            text = parse_fixed_or_err
        else:
            log("    [smelting: declare-repair] could not patch the parse error -- retrying declare-repair from last good candidate")
        # else: keep `text` as the prior (still parseable, still under-declared)
        # candidate and let the loop retry declare-repair on it next attempt.
    return False, "unreachable"


def deprose(api_key, model, prose_text, world_dir, out_dir, max_repair, log):
    validate_bin = DMML_HS / "validate-commit"
    render_bin = DMML_HS / "render-snapshot"
    retro_gate_bin = build_retro_gate()
    check_declared_bin = build_check_declared()

    world_files = world_files_in(world_dir)
    if world_files:
        world_context = "--- CURRENT WORLD (reuse what's already here) ---\n" + base.render_snapshot(render_bin, world_files) + "--- end CURRENT WORLD ---"
    else:
        world_context = "The world is currently EMPTY -- there is nothing to reuse yet; every node and predicate you use will be new."

    surface_text = SURFACE_PATH.read_text()
    prompt = DEPROSE_SYSTEM_PROMPT.format(surface=surface_text, world_context=world_context, prose=prose_text)
    log("[ore extraction] dispatching...")
    content, usage, elapsed = base.call_openrouter(api_key, model, True, [{"role": "user", "content": prompt}])
    log(f"[ore extraction] {elapsed:.1f}s, {usage.get('prompt_tokens', 0)}+{usage.get('completion_tokens', 0)} tokens")

    if content.strip().upper() == "NONE":
        log("[ore extraction] model reports no extractable content")
        return {"accepted": [], "rejected": [], "total_tokens": usage.get("total_tokens", 0)}

    candidates = extract_fences(content)
    log(f"[ore extraction] {len(candidates)} candidate commit(s) extracted")

    out_dir.mkdir(parents=True, exist_ok=True)
    rejected_dir = out_dir / "rejected"
    accepted, rejected = [], []
    running_world_files = list(world_files)  # grows as candidates are accepted, so later
    # candidates in the SAME de-prose run can be gated against, and can
    # reuse, what earlier ones in this same run just added.

    # Real bug found and fixed 2026-09-03: file numbering used to be a
    # pure within-run candidate index ("{i:03d}-deprosed.dmml"), so a
    # second de-prose run into the SAME out_dir -- the exact incremental
    # pattern this tool is for, feeding a prior run's real output back in
    # as --world-dir -- silently overwrote the first run's output. Next
    # index now accounts for whatever's already in out_dir.
    existing_indices = [
        int(p.stem.split("-", 1)[0])
        for p in out_dir.glob("*-deprosed.dmml")
        if p.stem.split("-", 1)[0].isdigit()
    ]
    next_index = max(existing_indices, default=0) + 1

    for i, cand in enumerate(candidates, start=1):
        file_idx = next_index + i - 1
        log(f"  candidate {i}/{len(candidates)}:")
        ok, final_text_or_err = repair_loop(api_key, model, validate_bin, cand, max_repair, log)
        if not ok:
            log(f"    [smelting] FAILED after {max_repair} repair attempts -- rejected, not forced in")
            rejected_dir.mkdir(parents=True, exist_ok=True)
            rej_path = rejected_dir / f"{file_idx:03d}-unparseable.dmml"
            rej_path.write_text(f"-- REJECTED (unparseable after {max_repair} repairs): {final_text_or_err}\n\n{cand}\n")
            rejected.append({"index": i, "reason": "unparseable", "detail": final_text_or_err})
            continue

        final_text = final_text_or_err
        tmp = HERE / "results" / "_deprose_candidate.dmml"
        tmp.write_text(final_text)
        kind = base.classify_file(validate_bin, tmp)
        if kind != "commit":
            log("    [smelting] model produced a machine despite instructions -- rejected")
            rejected_dir.mkdir(parents=True, exist_ok=True)
            rej_path = rejected_dir / f"{file_idx:03d}-was-machine.dmml"
            rej_path.write_text(final_text)
            rejected.append({"index": i, "reason": "was_machine"})
            continue

        decl_ok, decl_detail = check_self_declared(check_declared_bin, tmp, running_world_files)
        if not decl_ok:
            decl_ok, decl_detail = declare_repair_loop(
                api_key, model, validate_bin, check_declared_bin, final_text, running_world_files, max_repair, log
            )
            if decl_ok:
                log("    [smelting: declare-repair] fixed")
                final_text = decl_detail
            else:
                log(f"    [assay: self-declaration] REJECTED -- uses a predicate never declared, unfixed after {max_repair} repairs:\n      {decl_detail.strip() if isinstance(decl_detail, str) else decl_detail}")
                rejected_dir.mkdir(parents=True, exist_ok=True)
                rej_path = rejected_dir / f"{file_idx:03d}-undeclared.dmml"
                rej_path.write_text(f"-- REJECTED (undeclared predicate): {decl_detail}\n\n{final_text}\n")
                rejected.append({"index": i, "reason": "undeclared_predicate", "detail": decl_detail})
                continue

        gate_ok, gate_detail = gate_candidate(retro_gate_bin, tmp, running_world_files)
        if not gate_ok:
            log(f"    [assay] REJECTED -- would break the existing world:\n      {gate_detail.strip()}")
            rejected_dir.mkdir(parents=True, exist_ok=True)
            rej_path = rejected_dir / f"{file_idx:03d}-gate-broken.dmml"
            rej_path.write_text(f"-- REJECTED (gate): {gate_detail}\n\n{final_text}\n")
            rejected.append({"index": i, "reason": "gate_broken", "detail": gate_detail})
            continue

        log("    [deposit] accepted")
        accepted_path = out_dir / f"{file_idx:03d}-deprosed.dmml"
        accepted_path.write_text(final_text)
        accepted.append(str(accepted_path))
        running_world_files.append(accepted_path)

    return {
        "accepted": accepted,
        "rejected": rejected,
        "total_tokens": usage.get("total_tokens", 0),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", required=True, type=Path, help="plain text file to de-prose")
    ap.add_argument("--world-dir", required=True, type=Path, help="existing .dmml world (may not exist -- empty world)")
    ap.add_argument("--out-dir", required=True, type=Path, help="where accepted commits are written")
    ap.add_argument("--model", default=DEFAULT_MODEL)
    ap.add_argument("--max-repair", type=int, default=3)
    args = ap.parse_args()

    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        print("OPENROUTER_API_KEY not set", file=sys.stderr)
        return 1

    print("[deprose] building validate-commit, render-snapshot...", flush=True)
    base.build_binaries()  # builds the shared ones; retro-gate built separately inside deprose()

    prose_text = args.source.read_text()

    def log(msg):
        print(msg, flush=True)

    result = deprose(api_key, args.model, prose_text, args.world_dir, args.out_dir, args.max_repair, log)
    print()
    print(f"[deprose] done: {len(result['accepted'])} accepted, {len(result['rejected'])} rejected, {result['total_tokens']} tokens")
    for p in result["accepted"]:
        print(f"  accepted: {p}")
    for r in result["rejected"]:
        print(f"  rejected #{r['index']}: {r['reason']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
