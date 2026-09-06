package org.writtenworld.androidpoc

// The real interpreter boundary -- replaces F1's fake `greetFromHaskell`
// now that dmml/dev-journal/2026-09-04-android-jni-vs-ipc.md's JNI
// recommendation has a real module behind it (DMML.JniBridge, calling
// dmml-hs's own DMML.Materialize/DMML.Guard/DMML.Fire directly). Kept
// as its own object, not folded into any one Activity, so it can be
// called from whatever the real UI layer ends up being (SPEC.md §13's
// client-language loop -- see written-world/dev-journal/2026-09-04-
// android-authoring-agent-design.md) without depending on any one
// Activity's lifecycle.
//
// UPDATED 2026-09-06: added the `*History` functions, which is what
// GameScreen.kt actually uses. The original single-commit `render`/
// `actions`/`fire` cannot support more than one fire -- DMML.JniBridge's
// own doc comment discloses this as a deliberate v1 scoping, not a bug
// -- so a real multi-round Compose screen needs the history-aware
// variants, which take the whole growing list of fired-commit source
// texts as a JSON array (kotlin.org.json, part of the Android framework,
// no extra dependency) and decode it with the SAME Data.Aeson the
// Haskell side already depends on.
//
// Every function here returns either real output or a plain
// "ERROR: ..."-prefixed string -- DMML.JniBridge.hs's own contract,
// chosen specifically so nothing across this boundary is a Haskell
// exception (see that module's header comment and the JNI-vs-IPC
// dev-journal entry's "no process isolation" mitigation). Callers
// should check for that prefix before treating the result as real
// output.
object DmmlBridge {
    init {
        System.loadLibrary("dmmlbridge")
    }

    /** Single-commit variants -- exactly one world commit, no history.
     * Kept for a caller with nothing to accumulate; GameScreen.kt does
     * NOT use these (it needs multi-round history, see below). */
    external fun render(worldSrc: String, machineSrc: String): String
    external fun actions(worldSrc: String, machineSrc: String, selfNode: String): String
    external fun fire(worldSrc: String, machineSrc: String, selfNode: String, transitionIdent: String): String

    /** History-aware variants -- `worldHistoryJson` is a JSON array of
     * world-commit source strings, oldest (the original world) first,
     * each subsequent entry one previously-fired commit's own rendered
     * text (DMML.Fire.renderFiredCommit's output, unmodified) -- exactly
     * the accumulation dmml-hs/app/TouchBrowser.hs and
     * InteractiveBrowser.hs already do natively against
     * DMML.Materialize.applyIdentifiedCommits. `fire` returns just the
     * NEWLY fired commit; the caller appends it to its own list and
     * passes the extended list on the next call -- this function never
     * mutates or extends the history itself. */
    external fun renderHistory(worldHistoryJson: String, machineSrc: String): String
    external fun actionsHistory(worldHistoryJson: String, machineSrc: String, selfNode: String): String
    external fun fireHistory(worldHistoryJson: String, machineSrc: String, selfNode: String, transitionIdent: String): String

    /** True iff `result` (from any function above) is an error, not
     * real output. */
    fun isError(result: String): Boolean = result.startsWith("ERROR:")
}
