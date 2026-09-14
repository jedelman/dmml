# Vendored `.so` build manifest

Called for by `dev-journal/2026-09-07-android-canonical-structure-
decisions.md`'s Decision 1 ("vendor the built `.so` per ABI, with a
real manifest — not just the filename — recording: the exact `dmml`
commit it was built from, the date, and the toolchain versions used")
but never actually created — this file is that manifest, created
2026-09-14 while reviewing the branch. **It starts empty of any real
build record**, since the `.so` files themselves live only on the
machine that built them (`android/app/src/main/jniLibs/` is
gitignored, correctly — see its own `.gitignore` comment) and this
session has no access to that machine or those files. Filling in a
commit hash here without having the actual `.so` to check it against
would be exactly the kind of fabricated-provenance record this
project's own discipline exists to prevent — so the entries below are
templates with the known-real toolchain facts filled in, and an
explicit `TODO` for whoever next builds or rebuilds a `.so`, not
invented values.

## Why this matters

Two different Android bridges get built from this repo
(`libdmmlbridge.so` / `org.writtenworld.androidpoc.DmmlBridge`,
`libdmmlandroidbridge.so` / `org.jasonedelman.writtenworld
.NativeBridge`), and neither is rebuilt automatically — a stale `.so`
silently running against a newer `dmml-hs` source tree would drift
without anyone noticing, since Gradle only packages whatever's already
sitting in `jniLibs/`, it doesn't check freshness. This file is the
paper trail that closes that gap: before trusting an on-device test
result, check this file to confirm the vendored `.so` was actually
built from the commit whose behavior you think you're testing.

## Toolchain (confirmed real, 2026-09-06 — see that entry for the full
build log and every blocker/fix)

- Cross-GHC: 9.2.5, via `MrAdityaAlok/ghc-cross-tools`, targeting
  `aarch64-unknown-linux-android` (and `x86_64-linux-android` for
  emulator testing on hosts that can't run arm64 system images).
- NDK: r27c.
- Built from source against the NDK's cross clang (none of these three
  are bundled by the NDK itself): libffi 3.4.6, GMP 6.3.0, libiconv 1.17.
- Link flags required beyond a plain `ghc -shared`: `-Wl,--whole-archive
  <path-to-libHSrts.a> <path-to-libffi.a> -Wl,--no-whole-archive` (GMP
  deliberately NOT whole-archived — see 2026-09-06 entry for why), plus
  `-Wl,-z,max-page-size=16384` (16KB page-size alignment, added
  2026-09-09 — see that entry).
- `libc++_shared.so` from the NDK sysroot must be copied into
  `jniLibs/<abi>/` alongside the built `.so` — not resolved
  automatically by Android's dynamic linker.

## Build records

Fill in one entry per `.so` rebuild, both ABIs. Do not guess a value
you can't confirm — leave the field as `TODO` rather than write a
plausible-looking placeholder; a wrong manifest is worse than an
incomplete one, since it actively misleads whoever reads it next.

### `libdmmlandroidbridge.so` (DMML.AndroidBridge / NativeBridge.kt)

| Field | Value |
|---|---|
| `dmml` commit built from | TODO — fill in from the machine that has the actual `.so` |
| Build date | TODO |
| ABIs built | TODO (arm64-v8a, x86_64, or both) |
| Toolchain versions | As above, unless changed — note here if so |
| Built by / on | TODO (which machine/session) |

### `libdmmlbridge.so` (DMML.JniBridge / DmmlBridge.kt)

| Field | Value |
|---|---|
| `dmml` commit built from | TODO |
| Build date | TODO |
| ABIs built | TODO |
| Toolchain versions | As above, unless changed — note here if so |
| Built by / on | TODO |
| Known issue | Not relinked with the RTS whole-archive fix — real `UnsatisfiedLinkError: stg_SRT_1_info` on load, see `README.md`'s "Two architectures, not yet unified" |
