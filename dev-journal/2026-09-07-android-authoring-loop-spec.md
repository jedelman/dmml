# Android agentic authoring — local worktree, spec only

The (b) half of Jason's browsing/authoring split (see
`dev-journal/2026-09-07-android-jgit-sync-spec.md` for (a)). Proceeding
with this now per Jason's own instruction: spec the authoring loop
whether or not the canonical world-repo structure has landed, since it
hadn't as of this writing. Like the JGit spec, this is NOT wired to real
code yet.

## Don't reinvent the loop — it's already settled

`written-world/dev-journal/2026-09-04-android-authoring-agent-design.md`
already answered the core question: reuse SPEC.md §13's client-language
loop (agent reads resolved DMML, emits a `View` — a `Panel` tagged union
of `Narration`/`List`/`Actions`/`Map` — the client renders it, the player
interacts, a `ClientEvent` goes back to the agent, the agent authors a
DMML commit) unchanged. That loop is platform-agnostic and was proven
end to end against real LLM calls before the local-first pivot — nothing
about it is Android-specific, and this entry doesn't touch it. What that
entry left concrete-but-undesigned is exactly what this entry now specs:
the ON-DEVICE ISOLATION mechanism (a "local worktree") the loop drafts
into, and how Android's own secret storage resolves the monetization
tension that same entry explicitly left open.

## "Local worktree" = a second JGit clone, not the `git worktree` command

Same reasoning as the browsing spec: JGit, no native code. JGit doesn't
implement the `git worktree` CLI feature (multiple working directories
sharing one `.git` object store) — but the property Jason actually wants
from "local worktree" (the authoring agent drafts in isolation, browsing
is never disturbed mid-draft) doesn't require that specific mechanism.
A second, fully independent JGit-managed clone, on its own branch,
gets the same isolation:

```
context.filesDir/world-repo/           <- (a)'s browsing clone, read-only, main branch
context.filesDir/authoring-worktree/   <- (b)'s clone, branch authoring/<device-id>
```

**Real, disclosed cost of this choice over a true `git worktree`**: two
full clones on disk instead of one repo + a lightweight second working
tree (JGit has no shared-object-store equivalent) — real storage
duplication, acceptable for a single small world repo, a real
scaling concern if the canonical repo turns out large. Worth
re-litigating once the actual repo's size is known, not before.

## The authoring loop, concretely

1. Agent reads the CURRENT resolved DMML from `authoring-worktree/`
   (starts as a fresh branch off whatever `world-repo/` has at the
   moment authoring begins — a real fork point, not a live merge of
   ongoing upstream changes mid-session).
2. Agent emits a `View` (per SPEC.md §13 — unchanged).
3. `GameScreen.kt`-equivalent authoring UI renders it, player interacts,
   emits a `ClientEvent`.
4. Agent authors a DMML commit in response.
5. **The commit is validated by firing it through the REAL gate before
   it's treated as real** — `DMML.Fire`'s consistency check
   (`gateConsistentTree`), reached via the exact same
   `dmml_fire_history` JNI path `dev-journal/2026-09-06-android-cross-compile-verified-on-device.md`
   already proved works on-device. Not a new validation mechanism — the
   same one every other real commit in this project goes through
   (`validate-commit`/`check-declared`/`retro-gate`'s discipline,
   reused, not reimplemented for Android).
6. On gate failure: the agent's draft is rejected — regenerate or
   refuse, never silently apply an inconsistent commit. On success: the
   commit is written into `authoring-worktree/`'s branch as a real git
   commit (not just held in memory), so a crashed/backgrounded app
   doesn't lose drafted-and-validated work.
7. Loop back to 1 with the updated resolved state.

## Local vs. network split — already decided, restated for this loop specifically

From the same prior entry, verbatim in spirit:

- **Local, instant, never touches network**: steps 1, 5, 6 above
  (materialize + fire, via JNI) — the player's own turn never blocks on
  connectivity.
- **Network-dependent, must degrade**: step 2's narration generation (one
  LLM call per turn). If it fails or times out: render the raw resolved
  facts with no narration rather than block the turn — same rule the
  reference client already follows, not a new one invented for Android.

## Secrets — the actual unlock, made concrete

Two credentials this loop needs, both stored via Android Keystore-backed
`EncryptedSharedPreferences` (Jetpack Security `security-crypto`
artifact — hardware-backed keys where the device supports it, software
fallback otherwise, never plaintext `SharedPreferences`):

- **The authoring LLM's API key** (BYOK) — this is the concrete
  resolution to the "no server proxy" tension
  `written-world/dev-journal/2026-09-04-android-authoring-agent-design.md`
  explicitly left open for the free tier: a client that can hold a
  secret safely no longer needs a server to broker it. Real UX surface
  not designed here: how the player actually enters/rotates this key
  (a settings screen, presumably, not built in this entry).
- **A git credential for pushing** `authoring-worktree/`'s branch
  upstream (see Publish below) — only needed once publishing is wired,
  not for local drafting.

**Explicitly still unresolved, not solved here either**: the METERED
tier (Gemini-Flash/GLM) still needs a real server-side proxy — Android
inherits this tension unchanged from the reference client, exactly as
the prior entry already said. Nothing about Keystore storage touches
this; Keystore solves "the client can hold a secret," not "the client
can afford a proxy-free metered call."

## Publish — merge and (optionally) push, not designed in detail here

Once the player is satisfied with a drafted session (or hits an explicit
"publish" action, not yet designed), `authoring-worktree/`'s branch gets
merged into `world-repo/`'s browsing branch locally, and optionally
pushed to the canonical remote if a push credential is configured. Real,
disclosed gaps, deliberately not solved in this spec:

- **Push permission model**: does every device get real push access to
  the canonical repo, or does publishing go through a review/PR-style
  gate? This is a real content-governance question (who's allowed to
  author canon), not a technical one — needs Jason's call, not a default
  assumed here.
- **Multi-device conflicts**: two devices authoring concurrently and
  both trying to publish is a real DAG-merge question this project's own
  content-addressed commit model (`DMML.LocalIdentity`,
  `DMML.Retroconsistency`) is well-suited to eventually answer, but
  answering it is out of scope for this entry — v1 is implicitly
  single-device, single-author.

## What this spec deliberately does NOT include

No Kotlin/JGit code, no UI mockup, no `View`/`Panel` wire-format
specifics (already real and settled in SPEC.md §13, not restated here).
This is the isolation/validation/secrets architecture ONLY — the next
real step is wiring `DMML.JniBridge`'s multi-machine extension (see the
JGit spec's own "one real blocker" section — it blocks (b) exactly as
much as it blocks (a), since firing against a real multi-machine world
needs it too) and then building `WorldRepo`'s authoring-clone
counterpart against whichever canonical structure Jason lands on.
