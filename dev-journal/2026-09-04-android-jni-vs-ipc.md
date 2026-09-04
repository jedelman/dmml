# Android client: JNI in-process, not IPC-to-subprocess (F1 follow-up)

Answering a real question from the laptop session building out the
Android client's actual GHC-NDK cross-compile: JNI (keep the Haskell
interpreter in-process) vs. IPC (ship an ARM binary and shell out to it
like a desktop player runs `cli/`).

## The framing that matters: these are not "hard path vs. easy path"

The GHC→`aarch64-linux-android` cross-compilation cost is IDENTICAL
either way — IPC doesn't avoid it, it still needs the same NDK toolchain
`android-poc/build-android.sh` already targets. The two options only
differ in how the app talks to that ARM binary once it exists.

## The real, concrete problem with IPC specifically on Android

Modern Android (API 29+, W^X enforcement) generally blocks an app from
executing an arbitrary bundled file (assets/, app-private data dir) as a
subprocess on a production, non-rooted device — executable code has to
live in `lib/<abi>/` and load as a shared library. This is exactly why
`hatter`'s own documented shape is "cross-compile to a `.so`, load via
`System.loadLibrary`, JNI bridge calls in" rather than "bundle a binary,
`ProcessBuilder` it" — the JNI path is what the platform actually expects
and sanctions; the subprocess path would mean fighting this restriction
on top of, not instead of, the same cross-compile work (the Termux-style
workarounds for it exist but are fragile and have tightened release over
release).

## Recommendation: JNI, in-process, calling dmml-hs's own library functions directly

Not the `cli/` executable's argv interface wrapped a second time — the
JNI bridge should sit at the SAME layer `cli/`'s `look`/`fire`/`validate`
already sit at (`DMML.Materialize.renderSnapshot`, `DMML.Fire.
fireTransition`, the checkpoint functions), since those CLI commands are
themselves thin wrappers over exactly those library calls. Wrapping the
wrapper (JNI shelling out to the CLI binary) would be the worst of both:
still needs the subprocess-exec problem above, with no benefit over
calling the library directly.

## The real cost this choice has, and the concrete mitigation

No process isolation: an uncaught Haskell exception crossing the FFI
boundary is undefined behavior, not a clean subprocess exit code — unlike
a crash in a spawned CLI process, which would just be a nonzero exit the
app can handle. F1's `hsGreet` PoC has no real error path to catch, so
this wasn't exercised there. **Required for the real bridge**: every
`foreign export ccall` entry point wraps its body in `Control.Exception.
try` and marshals a failure back as an error-result string (or a tagged
result type), never lets an exception escape across the boundary raw.

## Status

Recommendation only — not yet validated against a real cross-compiled
build (that's the laptop session's own next step, same as F1's existing
handoff in `BOOTSTRAP.md`). Fold into `android-poc/README.md`/`jedelman/
dmml#1`'s F1 tracking once the laptop session confirms this shape
actually builds and runs on-device; not promoted to a firm, closed
decision here since it hasn't been proven end to end yet.
