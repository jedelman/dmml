"""A real, minimal Datalog engine used as a commit validator, run against
tonight's actual written-world corpus (311 real checkpointed commits, not
synthetic data). Stdlib only, in keeping with harness.py's own convention.

Scope, stated honestly: this validates the STRUCTURAL envelope every DMML
commit already carries (kind, consumes, responds_to) -- it does not parse
or reason over the prose in object_text. That's a real, separate problem
(extracting facts out of narrative text) this file makes no attempt at.

Two known simplifications, named rather than hidden:
  1. Evaluation is naive (recompute to fixpoint each round), fine at this
     scale (hundreds of facts), not designed to scale past it.
  2. Stratification is manual, not automatic: recursive positive rules
     (restsOn) are evaluated to a fixpoint first; only then do the
     non-recursive rules that negate them run. A rule that needed negation
     INSIDE a recursive cycle would break this and isn't attempted here.
"""

import json
import sys
from itertools import product


# ---------------------------------------------------------------------------
# A small general Datalog engine: ground facts, Horn-clause rules with
# variables (unification via simple binding), optional negation on body
# literals (checked only against relations already computed in an earlier
# stratum -- see module docstring).
# ---------------------------------------------------------------------------

class Var:
    __slots__ = ("name",)
    def __init__(self, name):
        self.name = name
    def __repr__(self):
        return f"?{self.name}"


def is_var(x):
    return isinstance(x, Var)


class Datalog:
    def __init__(self):
        self.facts = {}  # predicate -> set of tuples

    def add_fact(self, pred, *args):
        self.facts.setdefault(pred, set()).add(tuple(args))

    def query(self, pred, args, bindings):
        """Yield each binding extending `bindings` that satisfies pred(args)."""
        for tup in self.facts.get(pred, ()):
            b = dict(bindings)
            ok = True
            for a, v in zip(args, tup):
                if is_var(a):
                    if a.name in b and b[a.name] != v:
                        ok = False
                        break
                    b[a.name] = v
                elif a != v:
                    ok = False
                    break
            if ok:
                yield b

    def eval_body(self, body, bindings=None):
        bindings = bindings or {}
        results = [bindings]
        for lit in body:
            pred, args, negated = lit
            new_results = []
            for b in results:
                bound_args = [b.get(a.name, a) if is_var(a) else a for a in args]
                if negated:
                    if not any(True for _ in self.query(pred, bound_args, b)):
                        new_results.append(b)
                else:
                    new_results.extend(self.query(pred, bound_args, b))
            results = new_results
        return results

    def run_to_fixpoint(self, rules):
        """rules: list of (head_pred, head_args, body). Positive-only, may
        be recursive. Iterates until no new facts are derived."""
        changed = True
        while changed:
            changed = False
            for head_pred, head_args, body in rules:
                for b in self.eval_body(body):
                    tup = tuple(b.get(a.name, a) if is_var(a) else a for a in head_args)
                    before = len(self.facts.get(head_pred, ()))
                    self.add_fact(head_pred, *tup)
                    if len(self.facts[head_pred]) > before:
                        changed = True

    def run_once(self, head_pred, head_args, body):
        """Non-recursive rule, evaluated once (stratum after fixpoint)."""
        for b in self.eval_body(body):
            tup = tuple(b.get(a.name, a) if is_var(a) else a for a in head_args)
            self.add_fact(head_pred, *tup)


# ---------------------------------------------------------------------------
# Real fact extraction from the actual checkpointed corpus.
# ---------------------------------------------------------------------------

def load_corpus(path):
    return json.load(open(path))


def extract_facts(db: Datalog, log: list):
    n_parsed = n_raw = 0
    for e in log:
        if "kind" not in e:
            n_raw += 1
            continue
        n_parsed += 1
        uri = e["uri"]
        db.add_fact("commit", uri, e["kind"], e.get("predicate_iri", "?"),
                    e.get("subject_slug", "?"), e.get("respondent", "?"))
        for c in (e.get("consumes") or []):
            db.add_fact("consumes", uri, c["uri"])
        rt = e.get("responds_to")
        if rt:
            db.add_fact("respondsTo", uri, rt["uri"])
    return n_parsed, n_raw


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else \
        "/home/user/dmml/dev-journal/artifacts/2026-08-29-written-world-continuous6-children.json"
    log = load_corpus(path)
    db = Datalog()
    n_parsed, n_raw = extract_facts(db, log)
    print(f"loaded {n_parsed} parsed real commits ({n_raw} raw/unparsed, skipped) from {path}")

    X, Y, Z = Var("X"), Var("Y"), Var("Z")

    # Stratum 0: recursive, positive-only. restsOn(X,Z) = X's citation
    # lineage -- every commit X transitively rests on, via real consumes edges.
    recursive_rules = [
        ("restsOn", (X, Y), [("consumes", (X, Y), False)]),
        ("restsOn", (X, Z), [("consumes", (X, Y), False), ("restsOn", (Y, Z), False)]),
    ]
    db.run_to_fixpoint(recursive_rules)

    # Stratum 1: non-recursive, may negate stratum-0 (or ground) relations.
    K = Var("K")
    db.run_once("legalAccept", (X,), [
        ("commit", (X, "accepts", Var("_p1"), Var("_s1"), Var("_r1")), False),
        ("respondsTo", (X, Y), False),
        ("commit", (Y, "raises", Var("_p2"), Var("_s2"), Var("_r2")), False),
    ])
    db.run_once("illegalAccept", (X,), [
        ("commit", (X, "accepts", Var("_p1"), Var("_s1"), Var("_r1")), False),
        ("respondsTo", (X, Y), False),
        ("commit", (Y, "raises", Var("_p2"), Var("_s2"), Var("_r2")), True),  # negated
    ])
    db.run_once("rootCommit", (X,), [
        ("commit", (X, K, Var("_p"), Var("_s"), Var("_r")), False),
        ("consumes", (X, Var("_any")), True),  # negated: X cites nothing
    ])
    db.run_once("uncited", (X,), [
        ("commit", (X, K, Var("_p"), Var("_s"), Var("_r")), False),
        ("respondsTo", (Var("_any"), X), True),  # negated: nothing responds_to X
        ("consumes", (Var("_any2"), X), True),   # negated: nothing consumes X
    ])

    n_commits = len(db.facts.get("commit", ()))
    n_consumes = len(db.facts.get("consumes", ()))
    n_responds = len(db.facts.get("respondsTo", ()))
    n_rests_on = len(db.facts.get("restsOn", ()))
    roots = db.facts.get("rootCommit", set())
    uncited = db.facts.get("uncited", set())
    legal_accepts = db.facts.get("legalAccept", set())
    illegal_accepts = db.facts.get("illegalAccept", set())

    print(f"\nground facts:  commit={n_commits}  consumes={n_consumes}  respondsTo={n_responds}")
    print(f"derived:       restsOn (transitive citation edges) = {n_rests_on}")
    print(f"               root commits (cite nothing) = {len(roots)}")
    print(f"               uncited leaves (nothing cites or responds to them) = {len(uncited)}")
    print(f"               legalAccept = {len(legal_accepts)}  illegalAccept = {len(illegal_accepts)}")

    if n_commits:
        max_lineage = max(
            (sum(1 for t in db.facts["restsOn"] if t[0] == c[0]) for c in db.facts["commit"]),
            default=0,
        )
        print(f"               deepest single commit's total lineage (all ancestors, transitive) = {max_lineage}")


if __name__ == "__main__":
    main()
