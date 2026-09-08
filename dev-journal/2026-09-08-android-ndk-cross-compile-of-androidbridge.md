# `DMML.AndroidBridge` cross-compiled for real, arm64-v8a `.so`, all symbols verified

Continues the cloud agent's 2026-09-08 handoff: item 1 of "(1) NDK
cross-compile `DMML.AndroidBridge` to a real arm64-v8a `.so`, (2)
minimal Kotlin app skeleton, (3) on-device verification in order, (4)
port `Broker.hs` orchestration." Also folds in the OkHttp migration
(`java.net.http.HttpClient` doesn't exist on Android/ART -- see the
same day's `2026-09-08-dmml-http-okhttp-migration.md`-equivalent work
in `dmml-hs`) into this same cross-compile, since `DMML.AndroidBridge`
depends on `DMML.Atproto`/`DMML.Llm`, both of which go through
`DMML.Http`.

## Toolchain: reused, not rebuilt

Everything from `2026-09-06-android-cross-compile-verified-on-device.md`
was still on disk in WSL2 (`~/ghc-cross`, `~/ghc-target`, `~/android-ndk`,
`~/libffi-android`, `~/android-libs` for gmp+iconv) -- reused as-is. That
cross-compile predates every JVM-dependent module (`DMML.Jgit`,
`DMML.Jni`, `DMML.Http`, `DMML.AndroidBridge`); this is the first time
any of them have gone through the NDK cross-compiler.

## New, real blockers -- none of them repeats of 2026-09-06's

1. **The cross-GHC wrapper (`~/ghc-cross/bin/ghc`) failed outright** --
   `could not determine version`, tracing to a stale Termux path shim
   (`/data/data/com.termux/files/usr/lib/ghc-9.2.5/bin/ghc`, symlinked
   to a directory layout that doesn't match). Fixed by invoking the
   *target-prefixed* binary directly instead --
   `~/ghc-target/bin/aarch64-linux-android-ghc` -- which works
   standalone with no Termux path fakery needed at all.
2. **`extra-libraries: jvm`, added to `dmml-hs.cabal` after 2026-09-06
   by the cloud agent's upcall-architecture work, has no real target on
   Android** -- Android/ART doesn't ship a linkable `libjvm.so` at all
   (the entire reason `DMML.Jni`'s `UpcallJvm` path exists is to avoid
   ever needing one there). `JNI_CreateJavaVM` is real code in
   `cbits/jni_prims.c` (`hs_jgit_create_jvm`, desktop/CLI-only,
   deliberately dead on Android) but still needs *something* to link
   against. Built a 5KB stub `libjvm.so` for `aarch64-linux-android28`
   exporting one no-op `JNI_CreateJavaVM` that returns an error code --
   real, minimal, and explicitly disclosed as never meant to actually
   run (Android always goes through `UpcallJvm`, never
   `hs_jgit_create_jvm`).
3. **NDK's own per-API-level clang wrapper scripts resolve their real
   binary via `` `dirname "$0"` ``, not symlink-aware** -- a first
   attempt at a cross-compile toolchain shim (unversioned names like
   `aarch64-linux-android-clang` that GHC's settings file expects,
   symlinked to the versioned `aarch64-linux-android28-clang`) failed
   with `clang: No such file or directory`, because the wrapper's own
   `dirname "$0"` resolved to the *symlink's* directory, not the real
   NDK `bin/`. Fixed by writing real one-line wrapper scripts
   (`exec "<absolute real path>" "$@"`) instead of symlinks.
4. **GHC's own per-module compile step invokes `ld.lld` by that exact
   unversioned name**, separately from whatever `--with-gcc` was
   passed -- needed `ld`, `ld.lld`, and the target-prefixed
   `ar`/`nm`/`ranlib` names all present on `PATH` pointing at the NDK's
   real binaries, not just the compiler.

With all four real and fixed: `cabal build lib:dmml-hs
--with-compiler=<target-ghc> --with-gcc=<ndk-clang28>` succeeded end to
end -- all 25 modules (`DMML.Http`'s OkHttp rewrite, `DMML.Jgit`,
`DMML.Jni`, `DMML.AndroidBridge`, everything) compiled to real `ELF ...
ARM aarch64` object code, exit 0.

## Linking `libdmmlbridge.so`

Same whole-archive recipe as 2026-09-06 (`--whole-archive
libHSrts.a/libffi.a --no-whole-archive`, GMP linked normally). New
this time: `cbits/android_onload.c` had to be compiled separately
against the real GHC-generated `DMML/AndroidBridge_stub.h` (confirming
its own doc comment's claim -- "included below rather than
hand-duplicating the signatures" -- is actually true, not aspirational)
and `HsFFI.h` from the cross-GHC's own include tree. Link succeeded,
exit 0, first real attempt once the object files existed.

## Verified, for real, via `llvm-nm`, not assumed

```
$ llvm-nm -D libdmmlbridge.so | grep -E 'JNI_OnLoad|android_'
T JNI_OnLoad
T android_atproto_create_record
T android_atproto_create_session
T android_atproto_pull
T android_atproto_resolve
T android_jgit_commit
T android_llm_chat_complete
```

All 7 expected symbols present, survives `llvm-strip` (95MB unstripped
-> 57MB stripped). `llvm-nm -D -u` (undefined dynamic symbols) shows
nothing unaccounted for: standard libc/libm/pthread/C++-ABI symbols
(resolved by Android's own `libc.so`/`libm.so`/`libc++_shared.so` at
load time) plus exactly one `JNI_CreateJavaVM`, resolved against the
stub `libjvm.so` from blocker #2 above.

## What this means for packaging -- a real, disclosed gap

`libdmmlbridge.so`'s `DT_NEEDED` list includes `libjvm.so` as a real
runtime dependency, purely because the desktop-only `EmbeddedJvm` code
path is still compiled into the same object closance as everything
Android actually uses. Real Android devices have no `libjvm.so`
anywhere on the system -- so shipping this `.so` without also bundling
the no-op stub into `jniLibs/<abi>/` (same pattern this project
already established for `libc++_shared.so` on 2026-09-06) would fail
to load at all, `UnsatisfiedLinkError`-style, the first time anything
tries `System.loadLibrary`. Bundled the stub alongside it as the
pragmatic fix for now. The cleaner fix -- `#ifndef __ANDROID__`
guarding `hs_jgit_create_jvm`/`hs_jgit_destroy_jvm` in
`cbits/jni_prims.c` so Android's build never references
`JNI_CreateJavaVM` at all -- is real, correct, and NOT done here;
noted as follow-up rather than done under time pressure disguised as
"simple."

## What's still open

- Not run on-device or on-emulator yet -- this entry proves the same
  thing 2026-09-06's did at the equivalent stage: real symbols, real
  ELF, `nm`-confirmed, not yet launch-verified. That's the cloud
  agent's own items 2 (Kotlin app skeleton) and 3 (on-device
  `jgitCommit`/`atprotoResolve`+`atprotoPull`/`llmChatComplete`
  verification, in that order) -- not started.
- Only arm64-v8a built this session, not x86_64-android (the ABI this
  dev machine's own emulator can actually run) -- 2026-09-06 needed
  both because the emulator here can't run arm64 at all. Building the
  x86_64 target too, the same way, would let items 2/3 actually launch
  on this machine's emulator rather than requiring a real device.
- The `libjvm.so`-stub-bundling gap above is real and disclosed, not
  silently worked around; the cleaner `#ifndef __ANDROID__` fix is
  explicitly deferred, not forgotten.
- Build outputs (`.so` files, the stub toolchain shim) are gitignored
  scratch artifacts (`dmml-hs/wsl-scratch-android/`), not committed --
  same "commit scripts, not artifacts" convention as
  `wsl-scratch-http/`.
