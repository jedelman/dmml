# haiku-world-test

Not a demo of a design pattern — a test artifact from checking whether
`dmml/.claude/skills/dmml-authoring/SKILL.md` actually works. A
Haiku-model agent, given only that skill file and two example `.dmml`
files, authored `world.dmml` (a ~75-fact mountain teahouse: a keeper,
rooms, teas, a stove, a kettle) and `machine.dmml` (a 5-state machine
governing the keeper's daily routine) from scratch, with no other
DMML exposure. An Opus-model agent then reviewed the result
adversarially, and its findings drove two real fixes elsewhere in this
session: the skill file itself (several sections added/corrected), and
a real ordering bug in `written-world`'s own CLI (see that repo's
`cli/app/Main.hs`).

## What the skill got right, concretely

Both files pass `validate-commit` cleanly, and firing `awaken()` then
`lightStove()` (verified independently, not just on Haiku's own
say-so) correctly moves the keeper resting → preparing → serving, the
storage room sealed → open, and the stove cold/20° → heated/90°, with
every predicate reading as one clean value afterward. The skill's
segment-count rule (multi-segment values for anything meant to be
externally guardable) and its hyphen rule were followed correctly
throughout ~75 facts with zero violations — real evidence the skill
prevents the mistakes it names, not just describes them.

## Known, disclosed defect: this machine deadlocks

**Do not use this as a pattern to copy.** `extinguishAndClose()` can
never fire: it retracts `room/storage`'s `open` status and
`stove/brass`'s `heated` status, both of which `lightStove()` and
`serveVisitor()`'s own guards still depend on — `DMML.Retroconsistency
.gateConsistentTree` correctly refuses the firing as a whole-machine
consistency break. There's also no `sleeping -> resting` edge, so even
without that gate the keeper's day never actually cycles. This is a
real, instructive example of writing a plausible-looking forward
sequence whose own later step tears down what an earlier step's guard
still needs — exactly the kind of thing a clean `validate-commit` pass
and even a couple of successful `fire`s will NOT catch. See the
skill's own checklist item on this.

Separately, `serveVisitor()`'s `guest`/`teaType` parameters are
declared but never referenced in any guard or effect — a plausible
minimal stub, not a bug, but also not a pattern to copy uncritically:
if a transition takes parameters, use them in at least one guard or
effect, or drop them.

Left in place, unfixed, on purpose — as a real, checked-in example of
what "passes validate-commit and fires twice" does NOT guarantee.
