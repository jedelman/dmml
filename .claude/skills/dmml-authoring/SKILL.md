---
name: dmml-authoring
description: Authoring real DMML content (Surface syntax .dmml files — commits and machines) correctly on the first try, without rediscovering the same grammar landmines by trial and error. Load before writing or generating any .dmml file, any DMML.Guard-facing agent code (a demiurge, an architect, any creation agent), or before reviewing someone else's generated DMML for correctness. Everything here is a real, verified finding from actually running the parser and the CLI against real content, not inference from the grammar spec alone.
---

# DMML authoring — hard-won, verified rules

Every rule below was learned the expensive way: a clean `cabal build`
passed, and the bug only showed up when the content was actually
*fired* through a real CLI (`written-world fire`, `validate-commit`).
**A clean build never proves DMML content is correct — only running it
does.** If you're generating `.dmml` files programmatically (a
demiurge, an architect, any creation agent), budget for this: write it,
build it, then actually fire every transition it defines against a
real `commits/` directory before trusting it.

## Node references: segment count changes meaning

A node reference is one or more `/`-separated segments (`door/1`,
`type/smith`, `state`). **How many segments it has changes what it
means, and the rule is different in different grammar positions**:

- **In a guard pattern's hop term** (`guard self `predicate` TERM`,
  or a later hop in a multi-hop chain): a **bare, single-segment**
  identifier (`open`, `hinge`, no `/`) is the **existential pattern
  variable** — it matches *anything*, the same as writing `?x` would in
  many other languages. Only a **multi-segment** reference (`state/
  open`, `type/smith`) parses as a literal match target. This is
  `DMML.Surface.pPatternTerm`'s real rule, confirmed by parsing it
  directly: `t <- pNodeRefText; if T.any (== '/') t then TermNode else
  TermVar`.
  - **Real consequence, hit for real**: `guard self `gates` door/1
    `equips` hinge `state` open` does **not** check "is the state
    literally open" — `open` here has no slash, so it's an unbound
    variable matching *any* state at all. The guard silently passed
    against a *closed* door. The fix wasn't a different guard — it was
    giving the door a *separate*, multi-segment, externally-checkable
    fact (`door/1 `status` status/passable`) to guard on instead of
    trying to literal-match a single-segment value from outside.
- **In a `states` block declaration**: only a **single-segment**
  identifier is legal — `pStateLine` uses `pIdent`, which has no `/`
  in its grammar at all. `states\n  state/open` is REJECTED outright
  (confirmed: `validate-commit` fails with "unexpected 'machin'... "
  from cascading parse failure). So a machine's own internal states
  can *never* be multi-segment, which is exactly why they can't be
  literal-matched from another machine's guard text (see above) — the
  fix has to be a separate, external, multi-segment fact, not renaming
  the states.
- **In an ordinary fact's value position** (`` subject `predicate`
  VALUE `` or `subject . predicate = VALUE`), single- and
  multi-segment references both parse as ordinary literal node values
  (`ValueNode`) — no ambiguity there. The ambiguity is specific to
  **guard pattern position**.

**Rule of thumb**: if you want a value to ever be *literal-matched by a
guard written somewhere else*, give it a real multi-segment name
(`state/open`, not `open`) and assert it as its own fact — never rely
on a machine's internal, necessarily-single-segment `state` being
externally guardable.

## There is no `?` sigil

An existential pattern variable is a **bare, unprefixed** identifier —
not `?foo`, not `_foo`, just `foo`. Writing `?hinge` is not "the
existential variable named hinge," it's a parse error (`validate-commit`
rejects it: "unexpected 'machin'..." from further down, since the whole
transition fails to parse). The **only** sigil-prefixed term in the
grammar is `$param` (`TermParam`, resolved from the transition's own
call-time parameters, e.g. `written-world fire machine/x transition
verb name=value`).

## Hyphens are invalid inside node-ref/identifier segments

`metal-object`, `early-days`, `content-gap-1` — all fail to parse, with
a misleading "incorrect indentation" error rather than an obvious
"invalid character" one. Use camelCase or a single run of letters/
digits instead: `metalobject`, `earlydays`, `contentgap1`.

## `from -> to` sugar adds a guard, never an effect

`` transitionName()\n  from -> to `` desugars to an **implicit guard**
(`self . state = from`) — nothing more. It does **not** also assert
`to`. If you want the transition to actually change the state, you
must write `assert to` yourself. Firing a transition with no
explicit effect at all produces a **fact-less, invalid commit** (DMML
rejects empty commits outright) — the fired commit fails to reparse.

**And `assert` alone isn't enough either.** Without a paired `retract
from`, the old value is never cleared — after firing, the predicate
reads as "N live alternatives" (both old and new value coexist)
instead of cleanly holding the new one. The full, correct shape is:

```
transition open()
  closed -> open
  retract closed
  assert open
```

`retract <bareIdent>` is real, separate sugar (always implicitly
`self . state`) — same idea as `assert`, not something you have to
reach for a general chained retract to get.

## Guard-only, no-effect transitions can be *listed*, but never *fired*

A transition with guards and no `assert`/`retract` at all (a pure
availability check, e.g. `world-browser-demo`'s own `actions.dmml`
pattern) works fine through `DMML.Guard.availableTransitions`/`mayFire`
— it's real, legal DMML. But **firing it through a real CLI (`written-
world fire`) produces an empty, invalid commit**, because DMML rejects
empty commits. If a transition needs to be *actually fireable*, not
just *listable*, give it a real effect — even a trivial one (a private
`used`/`unused` toggle on itself is enough).

## A freshly minted machine needs its own initial `state` fact

Minting a brand-new machine instance and immediately equipping it is
not enough — if its transitions use `from -> to` sugar, the implicit
guard (`self . state = from`) can never hold unless something asserted
an initial `state` fact for it first. A machine with no `state` fact at
all will refuse every `from -> to` transition unconditionally, even
when every other condition is satisfied. Assert the initial state in
the SAME commit that equips it.

## Effects CAN target other subjects, and CAN mint fresh nodes by firing

`DMML.Ast.Effect` generalized (2026-09-03) past `(self, "state", ident)`
— an effect's subject can be `self`, `$param`, or a literal node, and
its predicate/value can be anything. Concretely:

```
transition takeWard(token, ward)
  guard self `carries` $token
  assert $token `carries` $ward
```

**This also means a transition can mint a genuinely new node the
instant it fires**, no special mechanism needed: DMML is open-world (a
node exists the moment any fact mentions it, no separate registry) —
so `assert self `leadsTo` $room`, fired with `room=room/793` (a name
nobody has ever used before), brings `room/793` into existence right
then, checked by exactly the same self-declaration/guard/governance
machinery as any pre-existing node. **Do not assume "a machine can
never build/mint something by firing"** — it's a real, common, wrong
inference (made once in this project's own `written-world/cli/app/
Architect.hs`, corrected in place) — nothing about DMML's checks are
keyed on whether a node existed before this commit.

## `retract`/consumption needs a real git repository to actually apply

Confirmed by direct testing: firing a transition with a `retract`
effect against a `commits/` directory that is **not inside a real git
repository** can silently fail to actually clear the old fact — the
fired commit's own text still shows the right `consumes` citation, but
a subsequent `look` shows the predicate as multi-valued anyway (both
old and new). The identical sequence, run inside a real `git init`'d
directory, retracts cleanly. **Always test-fire generated `.dmml`
content inside a real, git-initialized `commits/` directory** — never
a bare scratch folder — matching `written-world/CLAUDE.md`'s own
"exercise the real CLI" rule, now with a concrete reason why.

## `written-world look`/`fire` do NOT apply governed arbitration; `render-snapshot` does

`written-world/cli/app/Main.hs`'s `worldSnapshot` calls
`DMML.Materialize.applyIdentifiedCommits` only — it never calls
`DMML.Governance.applyGovernance`. A genuinely governed, multi-valued
pair (one with a real `equips`/`trigger` machine claiming authority)
will still print as "N live alternatives" under `written-world look`,
even though a proper governed read would collapse it to one value.
`dmml-hs/app/RenderSnapshot.hs`'s `render-snapshot` binary DOES call
`applyGovernance` when machine files are among its inputs. If you need
a governance-aware read, use `render-snapshot`, not `written-world
look` — this is a real, current gap in `written-world`'s own CLI, not
something to work around by hand.

## Self-declaration is not automatically enforced

`declare relation X` / `declare attribute X` matters for documentation
and for `DMML.SelfDeclaration.undeclaredPredicates`, but **neither
`validate-commit` nor `written-world fire`/`look` actually calls that
check** — only the standalone `check-declared` binary does. Using an
undeclared predicate will NOT be caught by `validate-commit` (shape-
only) or by firing a transition. Don't assume "it built and fired
clean" means "every predicate was properly declared" — run
`check-declared` separately if that matters.

`declare relation` vs `declare attribute` is also just a label —
nothing currently checks that a "relation" only ever holds node values
or an "attribute" only ever holds literals (a real, disclosed,
unenforced distinction).

## Multi-hop guards are real, and chain forward through candidate sets

`guard self `p1` X `p2` Y` is one real guard clause with two hops, not
two guards — confirmed both in the real grammar (`pPattern = anchor;
hops <- some (...)`) and in real production content
(`dmml-hs/examples/chained-retract-demo/keeper.dmml`'s `guard self
`witnessedBy` self `at` $eruption`). Each hop narrows the CURRENT
candidate set forward — an unbound term at hop 2 fans out to every
match from wherever hop 1 landed, not from the original anchor. This
is what makes `guard self `gates` door/1 `status` status/passable`
correctly check "the door specifically," and what would make `guard
self `gates` door/1 `equips` hinge `state` open` (bare, single-segment
terms) wrongly match on ANY equipped machine's ANY state (see the
first section above) — the chaining itself was never the bug, the
single-segment literal-matching was.

## dot-syntax and backtick syntax are exactly equivalent

`subject . predicate = value` and `` subject `predicate` value `` are
parsed by the identical `pValue`, with the identical resulting
`FactStmt` — purely a style choice (`DMML.Surface.pDotFact`/
`pInfixFact`). Neither is more or less capable than the other; pick
whichever reads better in context.

## Practical checklist before trusting generated DMML content

1. Every guard-pattern term you want to literal-match: does it have a
   `/`? If not, it's an existential variable, not what you think it is.
2. Every `from -> to` transition: does it have BOTH an explicit
   `assert to` AND a `retract from`?
3. Every transition you expect to actually *fire* (not just list): does
   it have at least one real effect?
4. Every freshly minted machine: did you assert its initial `state` in
   the same commit that equips it?
5. Test by actually firing it (`written-world fire ...`), inside a real
   git-initialized `commits/` directory — never trust a clean
   `cabal build` alone.
6. If you need governance-collapsed reads, use `render-snapshot`
   (with machine files as input), not `written-world look`.
