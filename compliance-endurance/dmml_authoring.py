"""Syntax-level authoring scaffolding: assembles guaranteed-valid DMML
surface syntax from structured input, so an authoring agent's job is
deciding WHAT to assert, not generating correct grammar by hand.

Jason, 2026-09-03: "would it make sense to provide some scripts or tool
calls to help agents author incrementally, or quickly author common
units?" This is the syntax-level half of that answer -- pure text
assembly, no DMML semantics, no engine changes. It exists specifically
because this session's own real runs kept hitting the same class of
purely mechanical failures a deterministic assembler makes structurally
impossible: hyphenated commit verbs (a real 21% of REPORT.md's own
20-round endurance run's invalid attempts), duplicate facts within one
commit, wrong quoting/bareword confusion between node references and
string literals.

Deliberately NOT a semantic templating layer ("mint a standard NPC" as
one call) -- that's the deeper answer (generalizing DMML's own `machine`
Effect so a governed transition can assert a real fact cluster, tracked
separately), and building it here as a Python macro would be exactly
the kind of ungoverned, uncheckable intermediary this project's own
Section 10/11 argument (DMML is the evidence, not any agent's or any
tool's say-so) argues against. This module only ever emits raw DMML
surface text, which still goes through the same real parser/self-
declaration/gate checks as anything else -- it makes correct syntax
easy to produce, it doesn't skip verifying it.

Grammar constants below are the exact validators from
DMML.FromJson.isValidIdent/isValidNodeRef, kept in lockstep by hand
(no cross-language codegen here) since a mismatch would just mean this
assembler occasionally emits an ident the real parser then rejects --
caught immediately by check/commit either way, not a silent risk.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Union

IDENT_RE = re.compile(r"^[A-Za-z][A-Za-z0-9_]*$")


def is_valid_ident(s: str) -> bool:
    return bool(IDENT_RE.match(s))


def _is_valid_seg_piece(s: str) -> bool:
    return is_valid_ident(s) or (s != "" and s.isdigit())


def is_valid_node_ref(s: str) -> bool:
    if not s:
        return False
    return all(
        seg != "" and all(_is_valid_seg_piece(piece) for piece in seg.split("."))
        for seg in s.split("/")
    )


class AuthoringError(ValueError):
    """A structured-authoring request that can't become valid DMML at
    all -- caught here, before ever reaching the real parser, so the
    caller gets a precise reason instead of a generic parse error."""


@dataclass
class Value:
    """One tagged fact value -- the tag is explicit because DMML's own
    grammar disambiguates a bareword node reference from a string
    literal lexically (SURFACE.md's own "let lexical shape carry the
    tag" principle), and this assembler has no text to lex, only
    structured input, so the caller has to say which one is meant."""

    kind: str  # "node" | "string" | "number" | "bool"
    value: Union[str, float, bool]

    def render(self) -> str:
        if self.kind == "node":
            if not is_valid_node_ref(str(self.value)):
                raise AuthoringError(f"{self.value!r} is not a valid node reference")
            return str(self.value)
        if self.kind == "string":
            escaped = str(self.value).replace("\\", "\\\\").replace('"', '\\"')
            return f'"{escaped}"'
        if self.kind == "number":
            return repr(self.value)
        if self.kind == "bool":
            return "true" if self.value else "false"
        raise AuthoringError(f"unknown value kind {self.kind!r} -- must be node/string/number/bool")


@dataclass
class Mint:
    node: str
    type_ref: str  # the "a <type>" node reference, e.g. "Person"


@dataclass
class Fact:
    subject: str
    predicate: str
    value: Value
    form: str = "backtick"  # "backtick" | "dot" -- pure style, both lower identically


def build_commit(verb: str, declares: list[tuple[str, str]], mints: list[Mint], facts: list[Fact]) -> str:
    """Assembles one complete, syntactically-guaranteed-valid `commit`
    block. Real, structural checks applied BEFORE any text is emitted,
    not left for the parser to discover:
      - every ident (verb, declared idents, predicates) is checked
        against the real isValidIdent grammar
      - every node reference (mint targets, fact subjects, node-typed
        values) is checked against isValidNodeRef
      - declares are deduplicated by (kind, ident)
      - duplicate (subject, predicate) pairs within this one commit are
        rejected outright -- the exact real failure class REPORT.md's
        own endurance run hit ("duplicate fact within one commit," 10%
        of its invalid attempts), made structurally impossible here
        rather than corrected after the fact.
    Still just text: the result is exactly as un-trusted as anything
    else until it passes the real validate-commit/check-declared/
    retro-gate pipeline -- this function's only job is making that
    pipeline's job easier by removing purely mechanical failure modes.
    """
    if not is_valid_ident(verb):
        raise AuthoringError(f"commit verb {verb!r} is not a valid identifier (no hyphens, no spaces)")

    seen_declares: dict[tuple[str, str], None] = {}
    for kind, ident in declares:
        if kind not in ("relation", "attribute"):
            raise AuthoringError(f"declare kind must be 'relation' or 'attribute', got {kind!r}")
        if not is_valid_ident(ident):
            raise AuthoringError(f"declared ident {ident!r} is not a valid identifier")
        seen_declares[(kind, ident)] = None

    for m in mints:
        if not is_valid_node_ref(m.node):
            raise AuthoringError(f"mint target {m.node!r} is not a valid node reference")
        if not is_valid_node_ref(m.type_ref):
            raise AuthoringError(f"mint type {m.type_ref!r} is not a valid node reference")

    seen_facts: dict[tuple[str, str], None] = {}
    for f in facts:
        if not is_valid_node_ref(f.subject):
            raise AuthoringError(f"fact subject {f.subject!r} is not a valid node reference")
        if not is_valid_ident(f.predicate):
            raise AuthoringError(f"fact predicate {f.predicate!r} is not a valid identifier")
        key = (f.subject, f.predicate)
        if key in seen_facts:
            raise AuthoringError(
                f"duplicate fact within this commit: {f.subject} . {f.predicate} is asserted more than once "
                "-- the second occurrence would silently overwrite the first"
            )
        seen_facts[key] = None

    lines = [f"commit {verb}"]
    for kind, ident in seen_declares:
        lines.append(f"  declare {kind} {ident}")
    if seen_declares:
        lines.append("")

    for m in mints:
        lines.append(f"  {m.node} :: a {m.type_ref}")
    if mints:
        lines.append("")

    for f in facts:
        rendered = f.value.render()
        if f.form == "dot":
            lines.append(f"  {f.subject} . {f.predicate} = {rendered}")
        else:
            lines.append(f"  {f.subject} `{f.predicate}` {rendered}")

    return "\n".join(lines) + "\n"
