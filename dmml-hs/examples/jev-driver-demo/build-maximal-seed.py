#!/usr/bin/env python3
"""Build a world that a chosen pool of machines can actually act in.

A pool picked by `select-pool.py` is structurally good and mechanically
inert: every machine needs its own lifecycle state asserted, and every
guard needs something to match. Without that a 22-machine pool fires
nothing and the run measures the seed rather than the cannon.

Three things get emitted, all read off the machines themselves:

  LIFECYCLE   `<node> `state` <first declared state>`, the same
              first-state-is-initial convention DMML.Recombine's state
              alignment and the driver's own minting already rely on.

  LITERAL GUARDS   a guard `X `p` Y` over literal nodes is satisfied by
              asserting exactly that fact. Generous on purpose: the
              point is a world where the pool CAN act, and what it then
              does is the measurement.

  SUBSTANCE   a guard `?u `p` Y` binds a unit, so it needs units --
              several per binder, since a run that spends its only rock
              in round one is measuring scarcity, not construction.

One commit per file and no repeated (subject, predicate) within it --
both are real grammar constraints, and the second silently drops facts
if ignored, so the emitter dedupes and reports what it dropped.

Usage: build-maximal-seed.py <pool.txt> <out.dmml> [--units N]
"""

import argparse
import pathlib
import re
import sys

TRANSITION = re.compile(r"^  transition ")
GUARD = re.compile(r"^\s*guard\s+(not\s+)?(\S+)\s+`([A-Za-z0-9_]+)`\s+(\S+)\s*$")
NODE = re.compile(r"^[A-Za-z0-9_.\-]+/[A-Za-z0-9_.\-]+$")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("pool", type=pathlib.Path)
    ap.add_argument("out", type=pathlib.Path)
    ap.add_argument("--units", type=int, default=6, help="units minted per bound substance")
    ap.add_argument("--candidates", type=pathlib.Path, default=None,
                    help="also write a candidates JSON array: every zero-param transition in the pool")
    args = ap.parse_args()

    facts: dict[tuple[str, str], str] = {}
    candidates: list[dict] = []
    preds: set[str] = set()
    dropped: list[str] = []
    machines = 0

    def put(subj: str, pred: str, obj: str) -> None:
        preds.add(pred)
        key = (subj, pred)
        if key in facts:
            if facts[key] != obj:
                dropped.append(f"{subj} `{pred}` {obj} (already {facts[key]})")
            return
        facts[key] = obj

    for line in args.pool.read_text().splitlines():
        path = pathlib.Path(line.strip())
        if not path.is_file():
            print(f"missing: {path}", file=sys.stderr)
            continue
        text = path.read_text()
        head = text.splitlines()[0]
        if not head.startswith("machine "):
            continue
        machines += 1
        node = head[len("machine "):].strip()

        # Every zero-param transition becomes a candidate. A pool handed
        # over with no candidates is a pool that cannot act: the driver
        # only registers candidates for machines IT mints, so the seed
        # machines would sit inert while growth built around them.
        for block in re.split(r"^  transition ", text, flags=re.M)[1:]:
            head = block.splitlines()[0]
            ident, _, rest = head.partition("(")
            params = [x for x in rest.rstrip(")").split(",") if x.strip()]
            if params:
                continue
            guards = [l.strip()[len("guard "):] for l in block.splitlines() if l.strip().startswith("guard ")]
            effects = [l.strip() for l in block.splitlines()
                       if l.strip().startswith(("assert ", "retract ", "spawn ", "graft "))
                       and "`state`" not in l]
            candidates.append({
                "id": f"{node.replace('/', '_')}-{ident.strip()}",
                "machine": "../../" + str(path)[len("examples/"):] if str(path).startswith("examples/") else "../../" + str(path),
                "transition": ident.strip(),
                "verb": "acts",
                "params": {},
                "description": (
                    f"{ident.strip()} on {node}. "
                    f"Requires: {'; '.join(guards) or 'nothing'}. "
                    f"Yields: {'; '.join(effects) or 'nothing beyond advancing its own state'}."
                ),
            })

        states, in_states = [], False
        for l in text.splitlines():
            if l.strip() == "states":
                in_states = True
                continue
            if TRANSITION.match(l):
                in_states = False
            if in_states and l.strip():
                states.append(l.strip())
        if states:
            put(node, "state", states[0])

        for l in text.splitlines():
            m = GUARD.match(l)
            if not m or m.group(1):          # a negated guard wants the fact ABSENT
                continue
            subj, pred, obj = m.group(2), m.group(3), m.group(4)
            if pred == "state":              # lifecycle, already seeded above
                continue
            if not NODE.match(obj):          # a bareword/binder object matches anything
                continue
            if subj == "self":
                put(node, pred, obj)
            elif subj.startswith(("?", "$")):
                kind = obj.split("/")[0]
                for i in range(args.units):  # a bound guard needs UNITS, and several
                    put(f"{kind}/u{i}", pred, obj)
            elif NODE.match(subj):
                put(subj, pred, obj)

    # A commit LABEL is an identifier -- no hyphens. Caught by the parser
    # on the first load ("unexpected \"-s\"").
    lines = ["commit maximalSeed"]
    lines += [f"  declare relation {p}" for p in sorted(preds)]
    lines.append("")
    for (subj, pred), obj in sorted(facts.items()):
        lines.append(f"  {subj} `{pred}` {obj}")
    args.out.write_text("\n".join(lines) + "\n")

    print(f"{machines} machines -> {len(facts)} facts over {len(preds)} predicates -> {args.out}")
    subjects = len({s for s, _ in facts})
    units = len({s for s, _ in facts if re.match(r".*/u\d+$", s)})
    print(f"  {subjects} subjects, of which {units} are raw substance units")
    if args.candidates:
        import json
        args.candidates.write_text(json.dumps(candidates, indent=2) + "\n")
        print(f"  {len(candidates)} zero-param transition(s) -> {args.candidates}")
    if dropped:
        print(f"  {len(dropped)} fact(s) dropped as duplicate (subject, predicate):")
        for d in dropped[:5]:
            print(f"    {d}")


if __name__ == "__main__":
    main()
