# JVM-embed spike

Real, minimal proof that a GHC-compiled Haskell binary can embed a JVM
via the JNI Invocation API (`JNI_CreateJavaVM`) and call into it —
`FindClass("java/lang/String")`, then a clean `DestroyJavaVM`. Built
2026-09-07 to de-risk the "one canonical Haskell+JGit implementation
for CLI and Android" decision — see `written-world`'s
`dev-journal/2026-09-07-jgit-canonical-single-implementation.md` for
the full context and real transcript.

Not a package on its own — `shim.c` + `Spike.hs`, built directly with
`ghc` against the host JDK's `libjvm.so`:

```sh
ghc -o spike-hs Spike.hs shim.c \
  -I"$JAVA_HOME/include" -I"$JAVA_HOME/include/linux" \
  -optl-L"$JAVA_HOME/lib/server" -optl-ljvm \
  -optl-Wl,-rpath,"$JAVA_HOME/lib/server"
./spike-hs
```

**Proves**: the embedding mechanism works from inside a real GHC RTS
process on this host (JDK 21, x86_64 Linux). **Does not prove**: a real
`Git`/`AddCommand`/`CommitCommand`/`MergeCommand` call chain through
JGit (only `FindClass` was exercised), or anything about the Android
cross-compiled side (a JNI *upcall* into an already-running JVM, a
different code path, needing the laptop session's real NDK/cross-GHC
toolchain to verify — same host-vs-device split as F1's own
verification). Both real, disclosed gaps, not assumed closed by this
spike.
