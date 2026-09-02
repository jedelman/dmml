# F1 handoff: Android development moves to Jason's laptop

**Moved here from `written-world/dev-journal/` (same filename), 2026-09-02
— `android-poc/` itself moved from `written-world` to this repo the same
day. Content below is unchanged from the original; "this repo" in the
text below meant `written-world` at the time it was written, and
`CLAUDE.md`'s dev-journal convention it cites is `written-world`'s own
(this repo, `dmml`, has an equivalent `dev-journal/` of its own, just
not the same source of that specific convention-naming sentence).**

Jason: "I have an ori harness running against this repo on my laptop
with Thinking Machine's Inkling model harnessed in. check in a research
journal and bootstrap file with instructions for them to set up a local
dev environment. we'll do Android development on my laptop, so when I'm
away from the keyboard we'll just pause it."

This entry exists specifically so that incoming session (a different
harness, a different model, on different hardware) doesn't have to
reconstruct context from scratch — same reason this repo's own
`CLAUDE.md` names dev-journal entries as the cross-session handoff
mechanism, just crossing a harness/model boundary this time as well as
a session one, not only a session one.

## Why the handoff happens here, not further

`android-poc/`'s own `README.md` and `dev-journal/2026-09-02-f1-android-
jni-bridge-poc.md` already found and stated the real blocker: this
session's environment has no Android NDK, no Android SDK, no cross-
compiling GHC, and no device or emulator. What COULD be verified there
was — the actual FFI/JNI bridge mechanism itself, compiled and run for
real (not just written) on host GHC. What's left is specifically the
part that needs real hardware/toolchain this session never had access
to: cross-compiling `android-poc/haskell/Bridge.hs` for an Android ABI,
building the APK, and running it. Jason's laptop is a real machine with
(presumably, once `BOOTSTRAP.md` is followed) the Android SDK/NDK — the
natural, and now only real, place left to finish F1.

## What the incoming session should read first, in order

1. `android-poc/README.md` — the authoritative "what's proven, what
   isn't" split. Don't re-derive this; it's already been checked.
2. `dev-journal/2026-09-02-f1-android-jni-bridge-poc.md` — the
   narrative of how that PoC was built and what was actually run
   (a full `hs_init` → foreign-export call → string round-trip →
   `hs_exit` cycle, real output captured).
3. `android-poc/BOOTSTRAP.md` (new, this same commit) — the concrete,
   step-by-step local environment setup this journal entry exists to
   introduce.
4. `jedelman/dmml#1` (GitHub issue) — the single tracking issue for the
   whole `dmml-hs` spike this PoC is part of. F1 is the one item still
   open on it as of this handoff.

## Operating mode: human-paced, not autonomous background work

Jason's own framing — "when I'm away from the keyboard we'll just pause
it" — means this is supervised, interactive work on his machine, not a
scheduled or unattended loop. Nothing about this handoff asks the
incoming session to run unattended between sessions; picking the work
back up when Jason is at the keyboard is the expected, correct mode, not
a gap to fill with automation.

## What "done" looks like for F1

Per `android-poc/README.md`'s own "Next real step": run
`build-android.sh` for real, fix whatever it gets wrong (its own
comments already flag the exact toolchain flags as unconfirmed — expect
real fixes, not zero), `./gradlew assembleDebug`, and confirm on a real
device or emulator that the app launches and shows the Haskell-returned
string. Once that's true, `jedelman/dmml#1` can close F1 for real — see
that issue for exactly how the rest of the spike (A through E, F2) was
already closed, as a model for what "closed with real evidence" looks
like here too, not just a checked box.
