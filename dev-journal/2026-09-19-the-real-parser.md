# 2026-09-19 — The real parser

Jason: *"YES FIX IT!"* — the regex guard reader.

The conflict pruner needed to know what each candidate's guards read. It found out by
running a regex over the machine file: a second, weaker parser for a language that already
has one, sitting on the far side of a process boundary from the real one. `scan-candidates`
has the parsed `TransitionDecl` in hand. It should just say.

## What the regex could not have known

`scan-candidates` now reports a `reads` array per candidate — every `(subject, predicate)`
its guards look up, `null` subject meaning *any*. `guardReads` mirrors
`DMML.Guard.resolveTerm` case for case rather than approximating it, and two things fall out
that no text reader could have got:

**The implicit state guard.** `resolveTransition` prepends `(self, state, from)` for a
`from -> to` transition. It appears in no guard line, so a reader of the file cannot see it.
`quarry-delve`'s `rest()` has *zero* written guards and a real read set of exactly that one
slot. It was covered incidentally, because a transition that changes state also writes it —
but incidentally is a convention, not a guarantee, and a `from -> to` transition with no
explicit state effect would have been read as guarding on nothing at all.

**Params resolve.** `resolveTerm` lets a `?binder` fall back to the caller's own params, so
a candidate carrying `rock=rock/1` reads `(rock/1, in)`, not the wildcard `(*, in)` the
regex produced. Narrower, and narrower for the right reason. On `quarry-delve` that alone
took the round's probes from 90 to **37**, with 343 pairs pruned instead of 290.

## The thing I found on the way

`scan_binary()` read `$SCAN_CANDIDATES` and returned `None` if it was unset —
while `fire_binary()` has always fallen back to the plain name on `$PATH`. So **every run
that did not explicitly set that variable silently took the per-candidate path**, including
every demo run in this session. The batching from `00aa865` was not being exercised by the
scenarios at all.

Fixed by looking on `$PATH` too. The four scenarios, same seeds, same output:

| | fallback path | batch path |
|---|---|---|
| `quarry-delve` | 13.0 s | **1.4 s** |
| `cannon-grow` | 38.3 s | **4.3 s** |
| `cannon-fanout` | 46.6 s | **3.0 s** |
| `kiln-delve` | 5.1 s | **1.1 s** |

Transcripts identical between the two paths, so this is the same run, nine to fifteen times
faster, and it had been available since yesterday.

**The cost, stated plainly:** the fallback path no longer prunes at all. Read sets are
something only the real parser can report, and a checkout without `scan-candidates` does not
have one; `cand.reads is None` therefore means *unknown*, which `depends_on` reads as
"reads everything". Correct, and slower than it was this morning. The mitigation is that the
fallback is now taken only when the binary genuinely is absent.

## A check that had quietly stopped checking

The pruning was verified by running the driver with `may_conflict` monkey-patched to always
return `True` and diffing the transcripts. That was a real check when it was written. It
became a **no-op the moment the index landed** — `may_conflict` stayed correct, stayed
asserted against, and stopped being on the path the pair loop takes. Patching it disabled
nothing.

Caught by an implausible number, again: the harness reported the unpruned run probing *15*
pairs, the same as the pruned one. So the AST read sets were, for one round of checking,
verified against nothing.

Now committed as `conflict-pruning-selftest.py`, wired into CI, and it patches `ReadIndex`,
which is what the loop actually consults. It requires two things, not one:

- the transcripts match, line for line, over a whole deterministic run;
- **the unpruned run probed strictly more pairs.** A harness that stops disabling the thing
  under test is how a check goes vacuous, and this project has now shipped that twice.

Verified on all four scenarios:

```
quarry-delve    probed  37, pruned  343; exhaustive probed  380
cannon-grow     probed  35, pruned  188; exhaustive probed  223
kiln-delve      probed  15, pruned   92; exhaustive probed  107
cannon-fanout   probed  15, pruned 1372; exhaustive probed 1387
```

## Regression

14 Haskell selftests/demos green; `check-prose-coverage` 8/8; `minting-params-selftest.py`;
`index-scaling-bench.py`; `DMML_INDEX_SELFCHECK=1` across all four scenarios with no
disagreement; `conflict-pruning-selftest.py` on all four.

## Still open

- Checked the neighbours, since the same bug twice would be worse than once: `render-prose`,
  `list-candidates` and `cannon` all default to the plain name on `$PATH` already.
  `scan_binary` was the only asymmetric one. They are still one subprocess per round each,
  which is a separate and much smaller problem.
- The pruner still learns what a firing *writes* by regex over the commit output
  (`touches`). That is a different parser problem from the one just fixed, and
  `scan-candidates` could equally report the resolved effects.
