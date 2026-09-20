#!/usr/bin/env python3
"""Stage 1 of the code-nose pipeline: find candidate inconsistencies
mechanically, with no LLM call, so Jev (Stage 2) only ever sees a
short list of already-framed comparisons instead of raw source.

Three cluster types, each grounded in a real pattern already present
in this repo (see dev-journal / claude-memory conversation this was
designed from):

  A. sibling-family drift  -- files with the same basename living in
     different top-level directories (compliance/dispatch.py vs.
     compliance-parallel/dispatch.py, etc.): same job, deliberately
     forked, at risk of a fix landing in one copy and not the others.
  B. structural-template outliers -- files that belong to an obvious
     naming family (app/Check*.hs) and should share a shape (doc
     comment, getArgs, an empty-args usage branch, exitFailure on the
     failing path); flags line-count outliers and missing elements.
  C. comment-claim vs. code-value pairs -- a comment stating a number
     next to an assignment of a (possibly different) number. Doesn't
     decide staleness -- just surfaces the pair for Jev or a human to
     judge, which is exactly the "nose sniffs, doesn't decide" split.

Output: one JSON object per line (ndjson) to stdout, each already
phrased as a comparison ("claim") rather than a raw diff or raw file
dump -- see claude-memory's dmml projects.md entry on why the
*description* handed to Jev is the part that determines signal
quality, not the model call itself.

Usage: python3 stage1_cluster.py [--root PATH] > candidates.ndjson
"""

from __future__ import annotations

import argparse
import difflib
import json
import re
import statistics
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

SKIP_DIRS = {
    ".git", "dist-newstyle", ".stack-work", "__pycache__",
    "node_modules", ".venv", "venv", ".claude",
}

CODE_EXTS = {".py", ".hs"}


def iter_source_files(root: Path):
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        if path.suffix not in CODE_EXTS:
            continue
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        yield path


def git_changed_files(root: Path, since_ref: str) -> set[str] | None:
    """Files touched between the merge-base of `since_ref` and HEAD,
    as paths relative to `root` (matching what candidates put in
    "files"). Returns None if git or the ref isn't available, so the
    caller can fall back to an unscoped run rather than silently
    reporting zero candidates -- a nose that goes quiet because its
    git call failed is worse than one that's briefly unscoped.
    """
    def run(args: list[str]) -> str | None:
        try:
            return subprocess.run(
                args, cwd=root, capture_output=True, text=True, check=True
            ).stdout
        except (subprocess.CalledProcessError, FileNotFoundError, OSError):
            return None

    toplevel = run(["git", "rev-parse", "--show-toplevel"])
    merge_base = run(["git", "merge-base", since_ref, "HEAD"])
    if toplevel is None or merge_base is None:
        return None
    diff_out = run(["git", "diff", "--name-only", merge_base.strip(), "HEAD"])
    if diff_out is None:
        return None
    git_root = Path(toplevel.strip())
    changed: set[str] = set()
    for line in diff_out.splitlines():
        if not line.strip():
            continue
        abs_path = git_root / line
        try:
            changed.add(str(abs_path.relative_to(root)))
        except ValueError:
            continue  # touched a file outside --root; not this scan's business
    return changed


def scope_to_changed(candidates, changed_files: set[str]):
    for candidate in candidates:
        if any(f in changed_files for f in candidate["files"]):
            yield candidate


# ---------------------------------------------------------------------
# Cluster A: sibling-family drift
# ---------------------------------------------------------------------

def is_comment_or_blank(line: str, suffix: str) -> bool:
    s = line.strip()
    if not s:
        return True
    if suffix == ".py":
        return s.startswith("#") or s.startswith('"""') or s.startswith("'''")
    if suffix == ".hs":
        return s.startswith("--") or s.startswith("{-") or s.startswith("-}")
    return False


LOOKS_LIKE_CODE_VALUE = re.compile(
    r"""
    ^\s*[A-Za-z_][A-Za-z0-9_.\[\]]*\s*=\s*\S      # NAME = value  (py assignment)
    | ^\s*[A-Za-z_][A-Za-z0-9_']*\s*::            # Haskell type sig / binding cue
    """,
    re.VERBOSE,
)


def find_sibling_families(root: Path):
    by_basename: dict[str, list[Path]] = {}
    for path in iter_source_files(root):
        by_basename.setdefault(path.name, []).append(path)
    return {name: paths for name, paths in by_basename.items() if len(paths) >= 2}


def cluster_a_candidates(root: Path):
    # Star topology, not all-pairs: comparing every one of k siblings
    # against every other is C(k,2) hunks-worth of Jev questions for a
    # single naming family. On this repo's own compliance*/{dispatch,
    # score}.py families (5-6 dirs each) that alone produced 129
    # candidates -- already past the ~30-60 question ceiling measured
    # for a single live Jev call elsewhere in this project. One file per
    # family is picked as reference and every sibling is compared only
    # against it: k-1 comparisons instead of k*(k-1)/2, and it still
    # surfaces every real divergence (if A and B both differ from the
    # reference in the same way, that's the reference being the odd one
    # out, which is exactly as worth a look).
    families = find_sibling_families(root)
    for basename, paths in sorted(families.items()):
        paths = sorted(paths)
        reference, others = paths[0], paths[1:]
        for other in others:
            fa, fb = reference, other
            try:
                lines_a = fa.read_text(errors="replace").splitlines()
                lines_b = fb.read_text(errors="replace").splitlines()
            except OSError:
                continue
            sm = difflib.SequenceMatcher(a=lines_a, b=lines_b, autojunk=False)
            for tag, a1, a2, b1, b2 in sm.get_opcodes():
                if tag != "replace":
                    continue
                block_a = lines_a[a1:a2]
                block_b = lines_b[b1:b2]
                suffix = fa.suffix
                code_a = [l for l in block_a if not is_comment_or_blank(l, suffix)]
                code_b = [l for l in block_b if not is_comment_or_blank(l, suffix)]
                if not code_a and not code_b:
                    continue  # comment-only divergence -- expected, not a candidate
                value_like = any(
                    LOOKS_LIKE_CODE_VALUE.search(l) for l in code_a + code_b
                )
                if not value_like:
                    continue
                # shared context: how much of the two files agrees, as a
                # cheap proxy for "same family, not just same filename"
                ratio = sm.ratio()
                if ratio < 0.35:
                    continue  # too different to be the same lineage at all
                yield {
                    "cluster": "A",
                    "basename": basename,
                    "files": [str(fa.relative_to(root)), str(fb.relative_to(root))],
                    "anchor_lines": [[a1 + 1, a2], [b1 + 1, b2]],
                    "claim": (
                        f"{fa.relative_to(root)} and {fb.relative_to(root)} share "
                        f"{ratio:.0%} of their lines (same job, forked implementation), "
                        f"but disagree here:\n"
                        f"  {fa.name} L{a1+1}-{a2}: {' / '.join(code_a) or '(removed)'}\n"
                        f"  {fb.name} L{b1+1}-{b2}: {' / '.join(code_b) or '(removed)'}"
                    ),
                    "context_identical": f"{ratio:.0%} line-level similarity outside this hunk",
                }


# ---------------------------------------------------------------------
# Cluster B: structural-template outliers
# ---------------------------------------------------------------------

DOC_COMMENT_RE = re.compile(r"^\s*--\s*\|")
GETARGS_RE = re.compile(r"\bgetArgs\b")
EMPTY_ARGS_BRANCH_RE = re.compile(r"\[\]\s*->")
EXIT_FAILURE_RE = re.compile(r"\bexitFailure\b")


@dataclass
class ShapeFeatures:
    path: Path
    line_count: int
    has_doc_comment: bool
    has_getargs: bool
    has_empty_args_branch: bool
    has_exit_failure: bool


def extract_shape(path: Path) -> ShapeFeatures:
    text = path.read_text(errors="replace")
    lines = text.splitlines()
    return ShapeFeatures(
        path=path,
        line_count=len(lines),
        has_doc_comment=any(DOC_COMMENT_RE.search(l) for l in lines[:10]),
        has_getargs=bool(GETARGS_RE.search(text)),
        has_empty_args_branch=bool(EMPTY_ARGS_BRANCH_RE.search(text)),
        has_exit_failure=bool(EXIT_FAILURE_RE.search(text)),
    )


def naming_family(path: Path) -> str | None:
    """Group app/*.hs files by a shared name prefix (e.g. 'Check').
    Returns None for files that don't look like they belong to a
    same-purpose family (fewer than 2 siblings sharing the prefix)."""
    stem = path.stem
    m = re.match(r"([A-Z][a-z0-9]+)", stem)
    return m.group(1) if m else None


def cluster_b_candidates(root: Path):
    app_files = [p for p in iter_source_files(root) if p.suffix == ".hs" and "/app/" in str(p)]
    by_prefix: dict[str, list[Path]] = {}
    for p in app_files:
        prefix = naming_family(p)
        if prefix:
            by_prefix.setdefault(prefix, []).append(p)
    for prefix, paths in sorted(by_prefix.items()):
        if len(paths) < 3:
            continue  # need at least 3 to call a shared shape "the family norm"
        shapes = [extract_shape(p) for p in sorted(paths)]
        counts = statistics.median([s.line_count for s in shapes])
        for s in shapes:
            missing = []
            if not s.has_doc_comment:
                missing.append("no leading `-- |` doc comment")
            if not s.has_getargs:
                missing.append("no getArgs call")
            if not s.has_empty_args_branch:
                missing.append("no `[] ->` usage/empty-args branch")
            if not s.has_exit_failure:
                missing.append("no exitFailure on any path")
            size_ratio = s.line_count / counts if counts else 1.0
            is_size_outlier = size_ratio >= 2.0 or size_ratio <= 0.5
            if not missing and not is_size_outlier:
                continue
            rel = s.path.relative_to(root)
            siblings = ", ".join(str(p.relative_to(root).name) for p in sorted(paths) if p != s.path)
            claim_bits = []
            if missing:
                claim_bits.append("missing: " + "; ".join(missing))
            if is_size_outlier:
                claim_bits.append(
                    f"{s.line_count} lines vs. sibling median {counts:.0f} "
                    f"({size_ratio:.1f}x)"
                )
            yield {
                "cluster": "B",
                "family": prefix,
                "files": [str(rel)],
                "anchor_lines": [[1, s.line_count]],
                "claim": (
                    f"{rel} belongs to the '{prefix}*' family ({siblings}), "
                    f"which shares a CLI shape (doc comment, getArgs, "
                    f"empty-args usage branch, exitFailure on failure). "
                    f"This one: {'; '.join(claim_bits)}."
                ),
                "context_identical": f"{len(paths)} files share the '{prefix}' naming convention",
            }


# ---------------------------------------------------------------------
# Cluster C: comment-claim vs. code-value pairs
# ---------------------------------------------------------------------

NUMBER_IN_COMMENT_RE = re.compile(r"(?:#|--)\s*.*?\b(\d{2,})\b")
ASSIGNMENT_RE = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(\d+)\b")
CLAIM_VERB_RE = re.compile(
    r"\b(now|currently|sized|set to|capped at)\b",
    re.IGNORECASE,
)
# "bumped/reduced/increased FROM 8000 [to 12000]" names the OLD value, not
# a claim about what the code currently says -- caught by hand on this
# repo's own compliance-surface/dispatch.py (comment: "Bumped from 8000",
# code: MAX_TOKENS = 12000, both correct, and the naive rule flagged it as
# a mismatch). A number immediately preceded by "from" is history, not a
# claim, and must not be compared against the current assignment.
BACKWARD_REFERENCE_RE = re.compile(r"\bfrom\s+\d{2,}\b", re.IGNORECASE)

LOOKAHEAD_WINDOW = 6


def cluster_c_candidates(root: Path):
    for path in iter_source_files(root):
        try:
            lines = path.read_text(errors="replace").splitlines()
        except OSError:
            continue
        for idx, line in enumerate(lines):
            if not CLAIM_VERB_RE.search(line):
                continue
            if BACKWARD_REFERENCE_RE.search(line):
                continue  # "bumped/reduced/increased FROM N" -- N is history
            m = NUMBER_IN_COMMENT_RE.search(line)
            if not m:
                continue
            claimed_number = m.group(1)
            window = lines[idx : idx + LOOKAHEAD_WINDOW]
            assign_match = None
            assign_offset = None
            for off, wline in enumerate(window):
                am = ASSIGNMENT_RE.match(wline)
                if am:
                    assign_match = am
                    assign_offset = off
                    break
            if not assign_match:
                continue
            code_value = assign_match.group(2)
            var_name = assign_match.group(1)
            if code_value == claimed_number:
                continue  # they agree -- not a candidate, keep the nose quiet
            rel = path.relative_to(root)
            yield {
                "cluster": "C",
                "files": [str(rel)],
                "anchor_lines": [[idx + 1, idx + 1 + assign_offset]],
                "claim": (
                    f"{rel} L{idx+1}: comment mentions {claimed_number}, but "
                    f"L{idx+1+assign_offset} sets `{var_name} = {code_value}` "
                    f"-- comment text: \"{line.strip()}\""
                ),
                "context_identical": "comment and assignment are within "
                f"{assign_offset} line(s) of each other",
            }


# ---------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------

def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--root", type=Path, default=Path("."))
    ap.add_argument(
        "--cluster",
        choices=["A", "B", "C"],
        action="append",
        help="restrict to one or more cluster types (default: all)",
    )
    ap.add_argument(
        "--changed-since",
        metavar="REF",
        help=(
            "only emit candidates touching a file changed since REF "
            "(merge-base REF..HEAD) -- e.g. --changed-since origin/main "
            "for a PR run. A pre-existing sibling-family disagreement "
            "nobody's PR touched is noise, not signal; without this "
            "flag every run is unscoped (whole-repo, as before)."
        ),
    )
    args = ap.parse_args()
    root = args.root.resolve()
    wanted = set(args.cluster) if args.cluster else {"A", "B", "C"}

    generators = []
    if "A" in wanted:
        generators.append(cluster_a_candidates(root))
    if "B" in wanted:
        generators.append(cluster_b_candidates(root))
    if "C" in wanted:
        generators.append(cluster_c_candidates(root))

    all_candidates = (c for gen in generators for c in gen)

    if args.changed_since:
        changed = git_changed_files(root, args.changed_since)
        if changed is None:
            print(
                f"# stage1_cluster: could not resolve --changed-since "
                f"{args.changed_since!r} (git or ref unavailable) -- "
                f"falling back to an UNSCOPED run",
                file=sys.stderr,
            )
        else:
            print(
                f"# stage1_cluster: scoped to {len(changed)} changed file(s) "
                f"since {args.changed_since}",
                file=sys.stderr,
            )
            all_candidates = scope_to_changed(all_candidates, changed)

    count = 0
    for candidate in all_candidates:
        print(json.dumps(candidate))
        count += 1
    print(f"# stage1_cluster: {count} candidates", file=sys.stderr)


if __name__ == "__main__":
    main()
