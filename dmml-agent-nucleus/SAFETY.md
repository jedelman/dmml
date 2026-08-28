# Before you run a self-modified harness against real credentials

Everything checkpointed by this project so far was four bounded API calls
per turn, dispatched by a person, read by that person, and published only
after they read the output. That's a different risk class from an agent
that forks this repo, rewrites its own dispatch logic, and then runs the
result unattended against a real `DMML_APP_PASSWORD` that can write
permanent, publicly resolvable records under a real identity.

This file is not a gate DMML or this harness enforces — nothing here can
enforce it, by design (see `GRAMMAR.md`). It's the one thing worth doing
anyway, on your own discipline, before the gap above stops being abstract:

1. **Read your own diff before it runs.** Not "trust that it compiles" —
   actually read what changed in your dispatch, citation, or checkpoint
   logic. A self-modified harness that silently starts checkpointing more
   than you intended, or citing without verifying, is a real failure mode,
   not a hypothetical one — this project has already logged several real,
   distinct instances of a model producing well-formed-looking output that
   was subtly wrong (see the sibling `dmml` repo's `dev-journal/` entries
   on missing-required-field and schema-drift failures). Code you wrote to
   run unattended deserves at least the scrutiny a single tool call got.

2. **Test against a throwaway identity first.** Create a separate PDS
   account and DID for a self-modified harness's first few runs, not your
   real one. Checkpoints are permanent, publicly resolvable, and (per
   `README.md`) tied to whoever's credentials wrote them — there is no
   "undo" once something real is checkpointed under a real DID.

3. **Cap what a single run can do before you trust it.** A round limit, a
   spend limit, a dry-run mode that prints instead of checkpoints — pick
   one appropriate to what you actually changed, the way every
   `pantheon_*.rs` example in the sibling repo caps rounds and token
   budgets rather than running unbounded.

4. **If your fork gains the ability to modify itself again, mid-run, know
   that before you run it, not after.** That's a materially different
   thing than a fork you edit and then run — it's the actual "self-
   transforming machine" case, and it deserves more caution than a fork
   you reviewed once and then executed, not less.

None of this is a reason not to build the thing. It's the reason to build
it with your eyes open, which is the same discipline the rest of this
project has tried to hold everywhere else — fact-check the claims, verify
the citations, name the failure modes honestly instead of past them.
