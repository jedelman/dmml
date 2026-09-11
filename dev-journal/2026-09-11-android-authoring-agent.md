# The real BYOK authoring agent, ported to Android and verified live

Follow-up to 2026-09-09's DPoP work. Rather than design a new
authoring loop, ported `written-world`'s own real one
(`cli/app/Author.hs`, same branch/commit,
`claude/written-world-dmml-enrichment-257mkv`) onto the upcall
architecture -- matching the established convention every other
`DMML.AndroidBridge` function already follows.

## What the desktop original does, kept behavior-equivalent

A genuinely free-form authoring turn ("write me a room," "invent an
object here"), not a template match: loads the real current world
snapshot for grounding, builds a system prompt (real grammar rules +
one worked example, verified against `DMML.Surface`'s actual parse
behavior, not re-derived), calls `DMML.Llm.chatComplete`, strips
markdown fences models routinely add despite being told not to,
validates the response via `DMML.Surface.parseCommitSurface`/
`parseMachineSurface` -- never trusting raw model output -- and on
failure retries up to 3 times total, feeding the real parse error back
to the model each time. On success: writes the `.dmml` file and
commits it via JGit.

## The port

`DMML.AndroidBridge.authorBridge` (new) -- reuses
`DMML.Loader.worldSnapshotFromDirectory` for grounding instead of
duplicating the desktop original's own mtime-sorted loader (that
loader already exists here; the desktop original predates it and
still duplicates its own copy, a real, disclosed asymmetry between the
two, not fixed on either side). One real, necessary difference: no
`exitFailure` anywhere (a JNI-loaded library must never call it), and
the result is structured JSON instead of stdout lines -- same pattern
`brokerIncorporateBridge` already established for the same reason.

New: `android_author` foreign export, `cbits/android_onload.c`'s
matching `RegisterNatives` entry, `NativeBridge.author` in Kotlin, and
a 6th `VerifyActivity` check (a free-form text field + button) using
the saved BYOK API key.

Both `.so`s recompiled and relinked (16KB alignment kept, no new
toolchain blockers).

## Verified, for real, on the emulator, first real attempt, no retries needed

Request: "describe a small, dusty workshop with one interesting object
in it." Real model output parsed as valid DMML on the FIRST attempt --
no parse-error-retry loop needed to observe, though the code path for
it is the same as the desktop original's and unexercised by this one
success:

```
commit describeWorkshop
  declare relation contains
  declare attribute description

  room/workshop `description` "a small, dusty workshop cluttered with forgotten tools and scattered papers"
  room/workshop `contains` device/astrolabe
  device/astrolabe `description` "a gleaming brass astrolabe, its intricate engravings oddly untouched by dust"
```

Real commit SHA `b932444c07d45c2e850e99cb531e5c5fe53eed81`, written and
committed to the same on-device JGit repo the other checks use.

## What's still open

- The retry-on-parse-failure path is real code, ported verbatim, but
  not yet observed firing for real on Android -- the one real success
  above didn't need it.
- Not yet run on the phone specifically (same build works on the
  emulator; the phone should behave identically but hasn't been
  separately confirmed for this specific check).
- Authored content stays local (written + committed via JGit only) --
  publishing an authored commit to atproto (via `createRecordDpop`)
  would be the natural next step to actually get it out of the
  device, not built here.
- `DMML.Loader`'s duplication with `Author.hs`'s own loader (noted
  above) is a real, disclosed asymmetry between the desktop and
  written-world's own CLI -- not something this port fixes on either
  side.
