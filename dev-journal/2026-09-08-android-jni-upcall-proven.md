# The Android JNI-upcall path, proven end to end without Android hardware

Jason: "can we abstract out these incompatibilities into a common
environment interface, then build the android side? ... write a prompt
for the laptop agent to do the pieces that need an Android virtual
device." This is the account of what got built and verified here, and
what's left for a real device.

## The abstraction: `DMML.Jni.JvmEnvironment`/`withJvm`

Every module built on `JvmHandle` this cycle (`DMML.Jgit`, `DMML.Http`,
`DMML.Llm`) only ever needed a valid `JNIEnvPtr` — none of them cared
how it was obtained. The one real platform incompatibility was
entirely inside `withEmbeddedJvm`'s lifecycle: desktop creates and owns
a JVM (`JNI_CreateJavaVM`/`DestroyJavaVM`); Android upcalls into one
that already exists and must never create or destroy it.

```haskell
data JvmEnvironment
  = EmbeddedJvm FilePath   -- desktop: classpath to embed with
  | UpcallJvm JNIEnvPtr    -- android: the JNIEnv* the upcall received

withJvm :: JvmEnvironment -> (JvmHandle -> IO a) -> IO a
withJvm (EmbeddedJvm classpath) action = withEmbeddedJvm classpath action
withJvm (UpcallJvm envPtr) action = action (JvmHandle envPtr)
```

Every existing call site (`atproto-resolve`/`-publish`/`-pull`/
`-delete`, `jgit-smoke-test`, `jgit-checkpoint-demo`, and
`written-world`'s `fire`/`Broker.hs`/`Author.hs`/`BrokerSmokeTest.hs`)
was migrated from calling `withEmbeddedJvm` directly to
`withJvm (EmbeddedJvm classpath)` — a pure interface-shape change,
verified behavior-preserving by recompiling all of them and re-running
`written-world-author` live against a real BYOK key.

**Verified the `UpcallJvm` branch actually works, not just compiles**:
embedded a JVM to obtain a real `JNIEnv*` (standing in for "Android
handed this to an upcall"), fed that raw pointer through
`UpcallJvm`/`withJvm` (no create/destroy performed), and ran a real
JGit init/add/commit through it — correct commit, clean
`git fsck --full`.

## `DMML.AndroidBridge`: the real entry-point surface

New module, six `foreign export ccall` functions covering everything
built this cycle that needs a live `JNIEnv*` (as opposed to
`DMML.JniBridge`'s original v1 surface, which is pure Haskell logic
with no real JVM object calls at all):

- `android_jgit_commit` — write a file, `git add`+commit via `DMML.Jgit`
- `android_atproto_resolve` — handle/DID → PDS endpoint
- `android_atproto_pull` — `DMML.Atproto.pullNewRecords`
- `android_atproto_create_session` / `android_atproto_create_record`
- `android_llm_chat_complete` — `DMML.Llm.chatComplete`

Every function takes the upcall's real `JNIEnv*` as its own first
parameter (ordinary JNI convention) and does
`withJvm (UpcallJvm envPtr) $ \jvm -> ...` internally — same
exception-safety discipline as `DMML.JniBridge` (`try @SomeException`,
`\"ERROR: ...\"`-prefixed string on any failure, nothing escapes the
FFI boundary raw).

**Not yet covered**: `written-world`'s own `Broker.hs` `incorporate`
orchestration (validate + commit + divergence + checkpoint as one
unit) lives in `written-world`, not `dmml-hs`, and needs its own
parallel Android entry point once an app exists to call these
primitives together. This module proves the PRIMITIVES individually.

## `cbits/android_onload.c`: JNI_OnLoad + RegisterNatives

Binds the six exported C symbols to a documented (but overridable)
default Kotlin class, `org/jasonedelman/writtenworld/NativeBridge`, via
`RegisterNatives` inside `JNI_OnLoad` — not `Java_pkg_Class_method`
name-mangling, since no real Android app package existed when this was
written. Real, previously-unconsidered detail caught before it bit
anyone: a `.so` loaded INTO an already-running JVM (Android's own, or
this verification's plain JDK process) needs `hs_init()` called
explicitly inside `JNI_OnLoad` — nothing else starts the Haskell RTS
for it, unlike a desktop executable where GHC's own generated `main()`
does this automatically. Fixed before ever linking a real `.so`, not
discovered by a crash.

## Verified for real, on this host, without any Android tooling at all

This sandbox has no Android NDK, no `adb`, no AVD — checked directly,
not assumed. So the verification that WAS possible: GHC here already
targets `x86_64-linux`, and the JNI mechanism itself (`JNI_OnLoad`,
`RegisterNatives`, the calling convention, RTS lifecycle inside a
JVM-hosted library) is identical between a desktop JDK and ART — only
the final NDK cross-compile to `arm64-v8a` is untested here. So:

1. Built a real shared library (`ghc --make -shared -fPIC -dynamic`)
   linking `DMML.AndroidBridge`, `cbits/jni_prims.c`, and
   `cbits/android_onload.c` together — a real `.so`, not a plan for one.
2. Wrote a plain Java class (`org.jasonedelman.writtenworld.NativeBridge`)
   declaring the six `native` methods matching the `RegisterNatives`
   table exactly, and `System.load()`ed the `.so` for real.
3. Ran it. `JNI_OnLoad` fired, `hs_init` started the RTS, all six
   natives bound successfully, and every one of `jgitCommit`/
   `atprotoResolve`/`atprotoPull`/`llmChatComplete` executed correctly
   through a REAL JNI upcall (Java calling INTO Haskell, the exact
   opposite direction from every desktop entry point) — real git commit
   (`git fsck --full` clean), real network calls to `jason-edelman.org`'s
   real PDS, a real BYOK OpenRouter completion.

This is the actual mechanism Android needs, proven correct end to end
on a plain JVM. What's genuinely NOT proven: the NDK cross-compile
itself, and whether ART's own JNI implementation and bundled
`java.net.http.HttpClient`/JGit compatibility behave identically to
HotSpot's — real, disclosed, different risks from "does the upcall
mechanism work," which is what this entry proves.

## What's left, and needs a real device — handed to the laptop agent

1. Cross-compile `dmml-hs` (specifically `DMML.AndroidBridge` +
   `cbits/jni_prims.c` + `cbits/android_onload.c`) to a real
   `arm64-v8a` `.so` via the Android NDK's GHC cross toolchain
   (`hatter`'s demonstrated shape is the named precedent, never
   exercised against this codebase specifically).
2. A minimal Kotlin app skeleton with a `NativeBridge` object declaring
   the six `external fun`s exactly matching `cbits/android_onload.c`'s
   `RegisterNatives` table (or override the class name there and
   rebuild — see that file's own `BRIDGE_CLASS` macro).
3. Real, on-device confirmation that JGit and `java.net.http.HttpClient`
   actually behave correctly under ART — two real, disclosed, unverified
   assumptions this whole design has carried since 2026-09-07, still
   open.
4. `written-world`'s `Broker.hs` orchestration ported to a parallel
   Android entry point once the above works, so the whole sync loop
   (not just its individual primitives) runs on-device.

See the handoff prompt delivered directly to the laptop agent's session
for the concrete task breakdown.
