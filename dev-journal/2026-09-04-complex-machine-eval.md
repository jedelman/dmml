# Updating the authoring prompt for the generalized Effect, and a real eval against openrouter/free

Jason: "update the prompt for any agents using this then eval some of
the cheap models to see what they come up with. give them a bit of a
world to work with" -> "use openrouter/free and see what we get."

## What "any agents using this" actually meant

Grepped for every real authoring-agent prompt in the project. Two
candidates: `deprose_agent.py` (de-prose extraction) and `run.py` (the
4-agent free-authoring endurance harness). `deprose_agent.py` only
ever authors commits, never machines — its own rules never mention
machine grammar at all, so the generalized `Effect` doesn't touch it.
`run.py` DOES author both commits and machines, and its
`SYSTEM_PROMPT_TEMPLATE` interpolates `dmml-hs/SURFACE.md` directly as
the agent's grammar reference — which was itself stale. `SURFACE.md`'s
"Grammar (informal, machines)" section still documented ONLY the old
bare `assert <ident>`/`retract <ident>` sugar; the general form added
2026-09-03 (`dev-journal/2026-09-03-phase-2-3-effect-generalization-
and-firing.md`) was never added to the doc that's actually fed to
authoring agents as ground truth.

## What changed

**`dmml-hs/SURFACE.md`**: added the general assert/retract form to the
machine grammar block and bullet list, explained node-minting via a
transition parameter with the real `key/rusty42` example already on
record, and added `examples/complex-demo/master.dmml` as a third
worked example (the one that found the produces/consumes ordering bug
a few hours ago) alongside `door/12` and `shrine.dmml`.

**`compliance-endurance/run.py`**'s `SYSTEM_PROMPT_TEMPLATE`: added an
explicit paragraph inviting the general form and node-minting, and
stating directly that a multi-fact, multi-transition machine is a
stronger contribution than a minimal one-state-change machine — not
just passively available via the grammar reference, actively
requested.

## The eval: `eval_complex_machine.py`

New, narrower than `run.py`'s full 4-agent divergence/thrash harness:
N independent single-shot `openrouter/free` completions (real per-call
model rotation, whatever's currently free — not a fixed roster), each
asked once to author ONE complex machine against a real world —
`dmml-hs/examples/endurance/seed-genesis.dmml` plus its own real
11-machine set, reused rather than invented, per this project's own
DMML-first/reuse-real-evidence discipline. Every candidate goes through
the real `validate-commit`/`check-declared` pipeline, not just
eyeballed.

**Real run, n=6, `results/complex-machine-eval/`:**

5 different free models rotated in across 6 calls (`google/gemma-4-
31b-it:free`, `dots-studio/dots-3-note-preview:free` x3, `minimax/
minimax-m3:free`, `liquid/lfm-2.5-2.6b:free`). 2/6 parsed and fully
self-declared; both used the general assert form for real (one mints a
new node, `$eventId`, by naming it in `logEvent`'s effects); 0/6 used
the general retract form successfully.

**A real, single-root-cause bug, not model incompetence, in 3 of the 4
failures**: `minimax-m3`, `dots-3-note-preview` (twice) all
independently wrote a general retract WITH a trailing value —
`` retract $target `wardedBy` self `` — even though `SURFACE.md`
explicitly documents retract's general form as
`` retract <term> `<ident>` `` with no value. `DMML.Surface`'s
`pRetractGeneral` correctly parses up through the predicate backtick
and stops there; the trailing `self` becomes an unparsed token the
enclosing `indentBlock` then tries to read as the start of a NEW
transition-body line, failing with a real but confusing "incorrect
indentation" error rather than anything naming the actual problem
(trailing content after a valid retract). Confirmed by parsing each
failure directly with `parseMachineSurface` rather than trusting
`validate-commit`'s own error text, which (a separate, smaller, real
gap) always prints the COMMIT-parse error on a dual-parse failure, even
when the machine parse is what actually needs debugging.

The 4th failure (`liquid/lfm-2.5-2.6b:free`) is unrelated: the model
leaked a broken fine-tuned tool-calling template
(`<|tool_call_start|>[edit(...)]`) instead of the requested fenced
DMML — a real model malfunction on a small free model, not a grammar
or prompt issue.

## The real, informative finding

**Three independent free models, given no reason to expect it, all
guessed the SAME wrong shape for general retract** — a trailing value
symmetric to assert's. That's not noise; it's the natural generalization
from what the grammar looks like everywhere else (`guard`, `assert`,
and every plain fact all take a value after the backtick predicate).
The grammar's own asymmetry (retract genuinely has no value slot,
because it removes whatever's currently there regardless of what that
is) is the surprising part, not the models' guess.

**Not fixed here — a real design choice, not a bug fix, flagged for
Jason rather than made unilaterally**: either (a) tighten the prompt/
docs further to make the no-value rule impossible to miss, or (b)
extend the grammar to actually accept an optional trailing value on
general retract (parsed and either validated against the live fact or
simply ignored) so the shape real models organically reach for just
works. Option (b) is the more interesting one — three independent
models converging on the same "wrong" guess is real evidence about
what shape is actually natural to author, which is exactly the kind of
signal this project treats as informative rather than something to
train around.

**Also worth fixing regardless of that decision**: `validate-commit`'s
error reporting always shows the commit-side parse error on a
dual-parse failure, never the machine-side one — genuinely misleading
for exactly this kind of debugging (a real machine-shaped file whose
actual problem is deep in the machine grammar, not "this isn't a
commit").
