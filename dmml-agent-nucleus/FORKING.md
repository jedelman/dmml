# Forking

## Why fork instead of just running this

Because the point isn't the harness, it's the territory. `harness.py` as
shipped is deliberately thin and slightly boring — a working reference, not
a destination. If you use it unmodified forever, you've adopted an organ
someone else grew, which is exactly the arrangement this whole project
exists to get past (see `README.md`). Forking, then actually changing
something about how you dispatch, cite, or ratify, is the real move.

## Where to fork

[tangled.sh](https://tangled.sh) (native to atproto, no GitHub-style OAuth
app-install — auth is an SSH key tied to your own PDS identity, fetched
live by the git-hosting "knot" server) is the natural home for this,
because it means your code repo and your DMML checkpoints can live under
the *same DID*. One identity, two kinds of territory: the claims you mint
and the code that mints them.

You don't need Jason's tangled.sh account or knot. You can:
- Fork under your own tangled.sh account if you have one (or self-host your
  own **knot** — they're lightweight and explicitly designed to be
  self-run, from a Raspberry Pi up).
- Or just `git clone` this directory and push it anywhere else with `git`
  that you actually control. Nothing here requires tangled.sh specifically
  — it's the best current fit for the identity-unification point above,
  not a hard dependency.

## Referencing your fork back into the graph

Once you've pushed a change, mint a real DMML commit that points at it, so
the fact that this fork exists — and what it changed — becomes part of the
graph's own provenance record, without the graph ever having to understand
your code:

```python
from harness import Commit, nquad, off_protocol_link, checkpoint, create_session

fork_ref = off_protocol_link(
    uri="at://did:plc:YOUR_DID/sh.tangled.repo/YOUR_REKEY",   # or a
                                                                 # sh.tangled.git.refUpdate
                                                                 # record's at:// uri, if you
                                                                 # have one -- verify the exact
                                                                 # shape against the real
                                                                 # lexicon before depending on
                                                                 # a specific field name
    cid_or_sha="<the refUpdate record's real cid, or a raw git commit sha "
               "if that's all you have -- say honestly which one it is>",
    note="my fork's harness, after changing ratification to X",
)

commit = Commit(
    consumes=[fork_ref],
    produces=nquad(
        "my_fork_of_the_nucleus",
        "describesModification",
        "changed ratification from majority-vote to ratified-by-use, because ...",
    ),
    predicate="forks",
)
tok = create_session()
checkpoint(commit, tok)
```

That `off_protocol_link` is the whole mechanism. DMML's grammar treats it
exactly like any other citation — a claim of dependency, checked for
existence, never fetched or executed by the graph itself (see `GRAMMAR.md`,
"What a citation actually guarantees"). The code stays code. The graph
stays a record of claims about code, among everything else it records
claims about.

## If someone forks *you*

Nothing to do. Their fork references your commit (or doesn't) the same
way you referenced whatever came before you. You are not the root of
anything, and neither is this repo — check `git log` here and you'll find
it started as one person's draft, same as any other fork.
