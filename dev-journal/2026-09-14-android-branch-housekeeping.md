# Housekeeping pass on the Android work, while the laptop session rests

Jason asked for an honest evaluation of the laptop agent's Android work
(14 real commits past `26f63bd`, this session's own last contribution),
then asked to take care of what that review found. Record of what
actually got fixed here vs. what's flagged for the laptop session
specifically, since this cloud session has no NDK/AVD/WSL2 toolchain
and can't touch anything that needs one.

## The evaluation itself, briefly

Genuinely strong, trustworthy work — real on-device verification
throughout (OAuth login completed end to end on a real Pixel 8, DPoP
writes confirmed live, the BYOK authoring agent ported and verified),
spot-checked the DPoP/ECDSA crypto code by hand and it's correct (right
DER-to-raw conversion, right RFC 7638 thumbprint canonicalization,
private key genuinely never leaves Keystore). The dev-journal discipline
held up under scrutiny — it kept disclosing "not yet confirmed" in
places that would've been easy to gloss over.

**One correction to my own initial read**: I first described
`org.writtenworld.androidpoc`/`libdmmlbridge.so` as dead PoC code
worth deleting. Checked more carefully before touching anything — it
isn't. `NativeBridge.kt`'s own header comment already discloses "the
two architectures haven't been unified into one linked .so yet," and
`WorldRepository.kt` (directory-based git-sync browsing, real
functionality) is built on the older bridge. It's real, currently
*broken* (a real link bug, `UnsatisfiedLinkError: stg_SRT_1_info` —
never got the RTS whole-archive fix `libdmmlandroidbridge.so` has),
not abandoned. Deleting it would have destroyed real work to fix a
housekeeping complaint. Left it alone.

## What got fixed here (no toolchain needed)

1. **`README.md`/`BOOTSTRAP.md` staleness.** Both stopped updating
   after F1's 2026-09-06 close and gave a badly outdated picture of how
   far along the project actually is (`BOOTSTRAP.md` in particular
   still described the ORIGINAL, since-removed `haskell/Bridge.hs`
   PoC). Added a current-status banner to each rather than rewriting
   history — `README.md`'s points at the six real dev-journal entries
   from 09-07 through 09-11 and explains the two-bridge situation in
   one place; `BOOTSTRAP.md`'s marks itself superseded and points at
   the entry with the toolchain that actually worked
   (`2026-09-06-android-cross-compile-verified-on-device.md`).
2. **`SO_BUILD_MANIFEST.md`** (new) — the manifest Decision 1 in
   `2026-09-07-android-canonical-structure-decisions.md` called for but
   never got created. Filled in with the real, confirmed toolchain
   facts (GHC 9.2.5 via `ghc-cross-tools`, NDK r27c, the libffi/GMP/
   libiconv-from-source + whole-archive + 16KB-alignment link flags).
   **Deliberately left the actual commit-hash/build-date fields as
   `TODO`** rather than guess — the vendored `.so` files live only on
   the machine that built them (gitignored, correctly), which this
   session has no access to. Filling in a plausible-looking hash
   without the real `.so` to check it against would be exactly the
   fabricated-provenance mistake this project's own fact-checking
   discipline exists to catch.
3. **`ApiKeyStore.kt`**: `apply()` → `commit()`, matching the fix
   `OAuthPendingAuthStore`/`OAuthTokenStore` already got 2026-09-09 for
   a real, confirmed race (an async write losing to the process being
   backgrounded/killed right after). Lower-risk here (saving a BYOK key
   isn't immediately followed by handing off to a Custom Tab the way
   OAuth login is), but the same class of bug, fixed the same way for
   consistency rather than waiting to reproduce it a third time.

## What's flagged, not fixed here — needs the real toolchain or a live-write decision

- **The live test-record cleanup.** Not 1 stray record, 5:
  `test/dpop-verification` on `claude.jason-edelman.org`'s real,
  public PDS (`did:plc:5y6kop75jnvkbujbubrhj6e3`), rkeys
  `3mvalrncb242d`/`3mvalir52zl2j`/`3mvalgfvwpd2j`/`3mv4cq7bxug2v`/
  `3mv4chtfgl62v`, spanning 2026-09-09 through 2026-09-11 (the on-device
  verification check got run more than once). Confirmed via a live
  `atproto-resolve`/`listRecords` call — the account's OTHER ~45 real
  records (a `critiques`/`raises`/`repairs`/`replies`/`verifies`
  corpus from unrelated work, 2026-08-29 onward) are untouched and
  should stay that way. Deleting a live record on a real, public
  identity is exactly the kind of external-system write this session's
  own auto-mode classifier correctly refused without explicit
  confirmation — not attempted past that refusal. `deleteRecordDpop`
  still doesn't exist (same real, disclosed gap the 09-09 entry named);
  cleanup here would use the existing app-password `atproto-delete`
  instead, which still works against any account with app-password
  auth configured.
- **The two-bridge unification / `libdmmlbridge.so` relink.** Real fix
  is known (apply the same `-Wl,--whole-archive`/`libHSrts.a`/
  `libffi.a` treatment `libdmmlandroidbridge.so` already has, or unify
  into one `.so` entirely) — needs the actual NDK/cross-GHC toolchain,
  which only exists in the laptop's WSL2 environment. Documented in
  `README.md`'s new status banner so it isn't lost, not attempted here.
- **`SO_BUILD_MANIFEST.md`'s TODO fields** — need whoever has the
  actual vendored `.so` files to fill in real commit hashes/build
  dates, not guessable from here.

## Status

Pushed to `dmml` `main`. The live-record deletion is the one item
still waiting on Jason's explicit go-ahead before it happens at all.
