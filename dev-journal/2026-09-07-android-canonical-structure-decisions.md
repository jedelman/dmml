# Responding to written-world's canonical repo structure proposal

`written-world` `f2859b9`/`fbafaf3` (on `claude/written-world-dmml-
enrichment-257mkv`, not yet merged to `written-world`'s `main`) proposes
the JGit + on-device authoring loop's canonical structure, and correctly
identifies its own two open questions as this session's calls to make,
since this is the session with the actual cross-compile toolchain. Both
decided here. Supersedes/refines
`dev-journal/2026-09-07-android-jgit-sync-spec.md` and `2026-09-07-
android-authoring-loop-spec.md`, written before this proposal existed —
those two entries' JGit/isolation reasoning stands, but should now be
read against the real directory shape this entry accepts.

## Decision 1: vendor the built `.so`, not rebuild-from-pin — for now

The proposal's Recommendation 1 correctly frames (b) rebuild-from-pin as
the correcter long-term answer (matches `cli/`'s own cabal-pin
discipline exactly). But `dev-journal/2026-09-06-android-cross-compile-
verified-on-device.md`'s own honest account of what that toolchain
actually is right now — a WSL2 environment stood up by hand this
session, a chain of ~20 one-off scripts under `android-poc/wsl-scratch/`,
several of which needed real interactive fixes mid-run (the Termux path
symlink trick, the `--whole-archive` relink after two separate
`UnsatisfiedLinkError`s found only by launching on a real device) — is
not a reproducible, unattended build today. Recommending "rebuild from
pin at every build" would be recommending something that doesn't
actually work hands-off yet; that's not a real option to pick, it's an
aspiration.

**Decision: (a), vendor the built `.so` per ABI**, with a real manifest
(not just the filename) recording: the exact `dmml` commit it was built
from, the date, and the toolchain versions used (GHC 9.2.5 via
`MrAdityaAlok/ghc-cross-tools`, NDK r27c, libffi 3.4.6/GMP 6.3.0/libiconv
1.17 built from source). This is the honest answer given the toolchain's
actual current maturity, not a rejection of (b) in principle.

**Real follow-up this decision creates, not done here**: consolidate
`android-poc/wsl-scratch/`'s ~20 individual scripts into one real,
idempotent, restartable build script before (b) becomes achievable —
worth doing specifically BECAUSE it's the path to (b), not as an end in
itself. Not attempted in this entry; a real, separately-scoped piece of
work.

## Decision 2: yes to directory-based JNI entry points — via a real library extraction, not a bolt-on

The proposal's own open question leaned toward "the interpreter should
own this" but stopped short of forcing it, correctly noting it hadn't
checked whether the logic already lives somewhere shareable. Checked
directly this entry: **it doesn't.** `cli/app/Main.hs`'s `loadAll`
(directory scan, real-mtime sort — not filename sort, the exact fix
`61f0b4f` made — then parse) is `cli`-only application code today, not
part of `dmml-hs`'s library at all. So the real choice isn't "expose
what the library already correctly owns" vs. "duplicate it in Kotlin" —
it's "the sort/parse logic currently has exactly ONE home (`cli`), and
adding an Android directory-reading path means either a SECOND
independent implementation (Kotlin) or extracting it into a real shared
home for the first time."

**Decision: extract, don't duplicate.** Concretely, in that order:

1. Move `cli/app/Main.hs`'s `loadAll` (mtime-sorted directory scan +
   parse) into `dmml-hs`'s library proper — real name TBD, likely
   `DMML.Materialize` or a new small module, matching how this project
   already names things by what they materialize/govern rather than by
   which tool first needed them.
2. Have `cli` itself call the library version — this closes a real,
   PRESENT duplication risk (not a hypothetical future one): right now
   `cli`'s own mtime-sort fix lives nowhere else, so any other consumer
   of a `commits/` directory either reimplements it correctly or gets it
   wrong the same way `61f0b4f` already had to fix once.
3. Add real directory-based entry points to `DMML.JniBridge`
   (`dmml_render_dir`/`dmml_actions_dir`/`dmml_fire_dir` — naming to
   match the existing `_history` suffix convention once picked) that
   call the same shared function `cli` now calls too.

The existing `_history`-suffixed functions (JSON array of commit
strings, added for F1) stay — a caller with an in-memory history and no
real directory (the host smoke test, e.g.) still has a legitimate use
for them. The directory-based ones are additive, not a replacement.

**Not done in this entry**: the actual extraction/JNI work. This entry
records the decision and its concrete first step (which file's logic
moves where); implementing it is real, separately-scoped `dmml-hs` work,
naturally sequenced before either `written-world/android/`'s
`WorldRepository.kt` (browsing) or `AuthoringAgent.kt` (authoring) can
be built against a real `commits/` directory instead of the F1-era
JSON-array shortcut.

## What this leaves genuinely still open

Everything the proposal itself left open and didn't ask this session to
decide: the metered-gateway-vs-BYOK split for the authoring LLM call,
the push-permission/content-governance model for publishing, and
multi-device authoring conflicts. Unchanged by this entry — see the
proposal's own "What's genuinely open" section and `2026-09-07-android-
authoring-loop-spec.md`'s equivalent list.
