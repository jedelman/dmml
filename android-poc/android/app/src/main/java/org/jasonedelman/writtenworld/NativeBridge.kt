package org.jasonedelman.writtenworld

// The Kotlin half of the JNI-upcall architecture built 2026-09-07/08
// (dmml-hs's DMML.AndroidBridge + cbits/android_onload.c) -- the
// OPPOSITE direction from org.writtenworld.androidpoc.DmmlBridge
// (which is Kotlin calling INTO pure Haskell logic). Here, Haskell
// calls BACK OUT into real JVM objects -- JGit's `Git`, OkHttp's
// `OkHttpClient` -- via the live JNIEnv* this class's own native
// methods hand it, borrowed rather than a second embedded JVM (see
// DMML.Jni's `UpcallJvm`, and why one is needed at all: Android/ART
// exposes no linkable libjvm.so for a real JNI_CreateJavaVM embed).
//
// This class's exact package + name (`org.jasonedelman.writtenworld
// .NativeBridge`) and every method name/signature below are load-
// bearing: cbits/android_onload.c's JNI_OnLoad does
// `FindClass(env, "org/jasonedelman/writtenworld/NativeBridge")`
// then `RegisterNatives` against a hardcoded table of exactly these
// six (name, JNI signature) pairs. Move or rename anything here
// without updating that table and JNI_OnLoad's FindClass call fails
// at native-library-load time, silently disabling every native call
// below (not a crash -- `System.loadLibrary` still "succeeds"; only
// `UnsatisfiedLinkError` at first actual call reveals it, so this
// module's own on-device verification, not just a clean build, is
// what actually proves the binding is right).
//
// Loaded from a DIFFERENT shared library ("dmmlandroidbridge") than
// org.writtenworld.androidpoc.DmmlBridge's "dmmlbridge" -- the two
// architectures haven't been unified into one linked .so yet (see
// dev-journal/2026-09-08-android-ndk-cross-compile-of-androidbridge.md's
// "What's still open"), so both .so files are bundled side by side in
// jniLibs/ for now, each loaded independently, neither touching the
// other's native methods.
object NativeBridge {
    init {
        System.loadLibrary("dmmlandroidbridge")
    }

    /** Writes `content` to `repoDir/relPath` (parent dirs created as
     * needed), stages and commits it via JGit against the live
     * upcalled JVM. Returns "OK:<commit-sha>" on success, an
     * "ERROR: ..."-prefixed string on failure -- same contract as
     * every function below and as org.writtenworld.androidpoc
     * .DmmlBridge's own functions. `repoDir` must already be a real
     * git working tree (`git init`'d), same precondition DMML.Jgit's
     * `jgitOpen` has on desktop. */
    external fun jgitCommit(repoDir: String, relPath: String, content: String, message: String): String

    /** Resolves a handle or DID all the way to its real PDS endpoint.
     * Returns JSON `{"did":...,"pdsEndpoint":...}`. No credentials
     * needed -- pure public DID/handle resolution. */
    external fun atprotoResolve(identifier: String): String

    /** Pulls a peer's new commit records since `storedCursor` (pass
     * "" for "from the beginning"). Returns JSON
     * `{"nextCursor":...,"records":[{"rkey":...,"dmml":...}]}`. */
    external fun atprotoPull(peerIdentifier: String, collection: String, storedCursor: String): String

    /** Authenticates against a resolved PDS endpoint. Returns JSON
     * `{"did":...,"accessJwt":...,"pdsEndpoint":...}`. */
    external fun atprotoCreateSession(pdsEndpoint: String, identifier: String, password: String): String

    /** Publishes one DMML commit as a real atproto record.
     * `createdAt` is stamped on the Haskell side. Returns the
     * created record's at:// URI. */
    external fun atprotoCreateRecord(
        pdsEndpoint: String,
        did: String,
        accessJwt: String,
        collection: String,
        predicate: String,
        dmmlText: String,
    ): String

    /** Real atproto OAuth counterpart to [atprotoCreateRecord] -- same
     * job, but authenticated with a real DPoP-bound `accessToken`
     * from a completed OAuth login (see oauth/OAuthTokenStore.kt)
     * instead of an app-password session's plain-Bearer `accessJwt`.
     * DMML.Http computes the DPoP proof itself per-request (via an
     * upcall back into oauth/DpopKeyManager.kt) -- the caller here
     * doesn't build one. */
    external fun atprotoCreateRecordDpop(
        pdsEndpoint: String,
        did: String,
        accessToken: String,
        collection: String,
        predicate: String,
        dmmlText: String,
    ): String

    /** One BYOK chat completion via DMML.Llm.chatComplete. Returns
     * the raw assistant content, unvalidated as DMML -- the caller's
     * job on both platforms. */
    external fun llmChatComplete(apiKey: String, model: String, systemPrompt: String, userPrompt: String): String

    /** Port of written-world's Broker.hs `incorporate` (branch
     * claude/written-world-dmml-enrichment-257mkv, commit 317d179) --
     * pulls a peer's new commit records, validates the whole batch
     * (all-or-nothing), writes+commits them under `repoDir/commitsDir`,
     * computes real cross-player divergence, and folds the checkpoint
     * chain (only when `commitsDir` is literally "commits"). Returns
     * JSON -- see DMML.AndroidBridge.brokerIncorporateBridge's own doc
     * comment for the exact shape -- or an "ERROR: ..."-prefixed string
     * on a validation rejection or any other failure (never a partial
     * commit). `cursorFile`/`commitsDir` are paths relative to `repoDir`. */
    external fun brokerIncorporate(repoDir: String, peerIdentifier: String, cursorFile: String, commitsDir: String): String

    /** Port of written-world's own real BYOK authoring agent
     * (cli/app/Author.hs, same branch/commit as [brokerIncorporate]).
     * A genuinely free-form authoring turn -- "write me a room,"
     * "invent an object here" -- not a template match: grounds the
     * model in the real current world state under `repoDir/commits`,
     * validates every response via DMML.Surface before writing or
     * committing anything (up to 3 attempts, feeding the real parse
     * error back to the model on a retry), and on success writes +
     * commits the new .dmml file via JGit. Returns JSON
     * `{"path":...,"commitSha":...,"dmmlText":...}` on success, or an
     * "ERROR: ..."-prefixed string (an LLM call failure, or rejection
     * after 3 failed validation attempts) on failure. `repoDir` must
     * already be a real git working tree (`git init`'d), same
     * precondition [jgitCommit] has. */
    external fun author(repoDir: String, apiKey: String, model: String, request: String): String

    /** True iff `result` (from any function above) is an error, not
     * real output -- same "ERROR: ..." contract as DmmlBridge.isError. */
    fun isError(result: String): Boolean = result.startsWith("ERROR:")
}
