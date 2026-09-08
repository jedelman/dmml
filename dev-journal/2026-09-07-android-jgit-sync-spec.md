# Android browsing via JGit sync — spec, not yet wired

Follow-through on Jason's call after the real cross-compile/on-device
verification (`dev-journal/2026-09-06-android-cross-compile-verified-on-device.md`):
split the Android client into (a) browsing, synced from git, and (b)
agentic authoring, in an isolated local worktree (see
`dev-journal/2026-09-07-android-authoring-loop-spec.md` for that half).
This entry specs (a). Deliberately NOT wired to `GameScreen.kt` yet —
Jason is producing the canonical world-repo structure separately (a
cloud-agent session); this spec is written to be dropped onto that
structure once it exists, not to invent one.

## Why JGit, not a cross-compiled native `git`

This project just spent a full session cross-compiling GHC for Android
(`dev-journal/2026-09-06-android-cross-compile-verified-on-device.md`) —
real, necessary work, because there was no alternative to actually
running Haskell on-device. Git has no such constraint: JGit
(`org.eclipse.jgit`) is a mature, pure-JVM implementation with a long
history of working on Android (Gerrit's own tooling, MGit, and others
use it), needing zero native code and zero NDK involvement. Reaching for
a native cross-compiled `git` here would be repeating the hardest part
of last session's work for a problem that doesn't require it.

**Real, disclosed unverified item**: JGit's Android compatibility is
well-established by other projects' use of it, not independently
confirmed against THIS app's exact toolchain (AGP 9.4.0 / Kotlin 2.4.10
/ minSdk 28) in this repo. First real build against it should treat this
the same way `android-poc/README.md` always has treated its own
unverified steps — expect something to need a fix, don't assume clean.

## Dependency

```kotlin
// app/build.gradle.kts
implementation("org.eclipse.jgit:org.eclipse.jgit:6.10.0.202406032230-r")
// only if the canonical repo needs SSH (vs. HTTPS+PAT) transport:
// implementation("org.eclipse.jgit:org.eclipse.jgit.ssh.apache:6.10.0.202406032230-r")
```

No GPG/signature-verification artifact — browsing is read-only sync, not
provenance verification of authorship (DMML's own content-addressing,
already real via `DMML.LocalIdentity`, is the actual integrity mechanism
here, not git commit signatures).

## Storage

`context.filesDir/world-repo/` — app-private storage, no runtime
permission needed, cleaned up automatically on uninstall. Never
`/sdcard`-visible storage; this is synced content, not a user-facing
file the player is expected to browse outside the app.

## Sync model: clone once, fetch + fast-forward-only thereafter

```kotlin
// First run:
Git.cloneRepository()
    .setURI(worldRepoUrl)
    .setDirectory(File(context.filesDir, "world-repo"))
    .call()

// Every subsequent sync (pull-to-refresh, or a periodic trigger later):
val git = Git.open(File(context.filesDir, "world-repo"))
git.fetch().call()
val result = git.merge()
    .include(repo.resolve("origin/main"))
    .setFastForward(MergeCommand.FastForwardMode.FF_ONLY)
    .call()
```

Fast-forward-only, not a real merge or rebase — this clone is never
locally committed to (browsing is read-only), so a non-fast-forward
result means something unexpected happened (the local ref was somehow
advanced outside this sync path) and should surface as a real error, not
be silently resolved. This mirrors the discipline this whole project
already applies to itself — see `git-sync`'s own real usage across this
session: every pull this session was a fast-forward or an explicit,
visible merge, never a force.

## Auth

Assumed public/read-only for v1 (no credentials needed to clone/fetch).
If the canonical repo turns out private, JGit takes a
`CredentialsProvider` — the natural integration point is a PAT pulled
from Android Keystore-backed `EncryptedSharedPreferences` (Jetpack
Security), the same storage mechanism the authoring loop spec uses for
its LLM API key. Not built now since the repo's visibility isn't decided
yet — a real, disclosed unknown, not an oversight.

## Integration point (once the canonical structure exists)

A new `WorldRepo` class wrapping the JGit operations above, exposing
something like:

```kotlin
class WorldRepo(private val context: Context, private val remoteUrl: String) {
    suspend fun ensureCloned(): Unit
    suspend fun sync(): SyncResult  // Success | UpToDate | Error(reason)
    fun worldCommitSources(): List<String>   // read from the real layout, once known
    fun machineSources(): List<String>       // likewise
}
```

`GameScreen.kt` currently hardcodes `WORLD_SRC`/`MACHINE_SRC` as Kotlin
string constants (the same fixture `dmml-hs/app/JniBridgeSmokeTest.hs`
and `examples/interactive-browser-demo` already verified end-to-end).
Once `WorldRepo` exists, those constants get replaced by real reads
through it, feeding the exact same `DmmlBridge.renderHistory`/
`.actionsHistory`/`.fireHistory` calls already proven working on-device.

## The one real blocker this surfaces: `DMML.JniBridge` is single-machine

`dmml-hs/src/DMML/JniBridge.hs`'s own header comment already discloses
this: every exported function takes exactly ONE machine, not a set — an
honest v1 scoping, not a design ceiling, but a REAL limit the moment
browsing reads a canonical repo with more than one machine file (which
any real world almost certainly has — `examples/opus-world-test/` alone
ships seven). Extending `dmml_render_history`/`dmml_actions_history`/
`dmml_fire_history` to accept a machine-source array (same JSON-array
approach already used for world-commit history) is real, bounded,
follow-on work — not attempted here, since it's JNI-bridge work
independent of the git-sync question this entry specs.

## Open questions, genuinely unresolved

- **Update trigger**: pull-to-refresh (a Compose `SwipeRefresh`-style
  gesture) is the obvious v1 -- explicit, player-initiated, no background
  work. A periodic background sync (WorkManager) is a real, deferred
  enhancement, not v1 -- SPEC.md §13's own design already treats network
  activity as best-effort/non-blocking, and periodic background fetch
  adds battery/data-usage tradeoffs worth deciding deliberately, not by
  default.
- **Stale-content UX**: if `sync()` fails (offline, repo unreachable),
  browsing should keep working against whatever was last successfully
  cloned/fetched -- consistent with this whole project's "feels tight
  despite a bad link" goal (the same motivation behind
  `jedelman/dmml#7`'s checkpoint/atproto hardening). Needs a real "last
  synced at ___" indicator in the UI, not built here.
- **Repo layout**: entirely dependent on Jason's canonical structure,
  not yet available. This spec is deliberately silent on directory
  shape beyond "some set of world-commit files and some set of machine
  files" -- filling that in is the very next step once the structure
  lands.
