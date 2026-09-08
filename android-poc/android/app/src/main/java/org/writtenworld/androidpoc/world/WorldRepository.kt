package org.writtenworld.androidpoc.world

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.eclipse.jgit.api.Git
import org.eclipse.jgit.api.MergeCommand
import org.eclipse.jgit.api.errors.GitAPIException
import java.io.File

/**
 * Owns the on-device clone of the canonical world repo -- browsing's
 * git-sync half (dmml/dev-journal/2026-09-07-android-jgit-sync-spec.md,
 * refined by 2026-09-07-android-canonical-structure-decisions.md).
 * JGit, not a native git binary: pure-JVM, no NDK/cross-compile
 * involvement, unlike everything [DmmlBridge] itself needed.
 *
 * Deliberately READ-ONLY: this clone is never locally committed to.
 * Firing a transition (see [DmmlBridge.fireDir]) returns a commit's
 * text as a PREVIEW only -- nothing here writes it back, matching
 * `written-world`'s own CLI split (`look` never writes, only `fire`
 * does). Real persistence belongs to the separate, isolated authoring
 * worktree (dmml/dev-journal/2026-09-07-android-authoring-loop-spec.md)
 * -- not built yet, and deliberately not conflated with this class.
 *
 * Fast-forward-only sync: a non-fast-forward result means something
 * unexpected happened (this clone was somehow advanced outside this
 * sync path) and surfaces as a real [SyncResult.Error], never silently
 * resolved.
 */
sealed class SyncResult {
    data object UpToDate : SyncResult()
    data object Updated : SyncResult()
    data class Error(val reason: String) : SyncResult()
}

class WorldRepository(
    context: Context,
    private val remoteUrl: String,
    private val branch: String = "main",
) {
    private val repoDir = File(context.filesDir, "world-repo")

    /** Where [DmmlBridge]'s `dmml_*_dir` functions should read from --
     * per the canonical structure proposal, `commits/` inside the
     * checkout, not the checkout root itself. */
    val commitsDir: File
        get() = File(repoDir, "commits")

    fun isCloned(): Boolean = File(repoDir, ".git").exists()

    /** Clones on first use; a no-op ([SyncResult.UpToDate]) if already
     * cloned -- call [sync] to actually pull updates thereafter. */
    suspend fun ensureCloned(): SyncResult = withContext(Dispatchers.IO) {
        if (isCloned()) return@withContext SyncResult.UpToDate
        try {
            Git.cloneRepository()
                .setURI(remoteUrl)
                .setDirectory(repoDir)
                .setBranch(branch)
                .call()
                .close()
            SyncResult.Updated
        } catch (e: GitAPIException) {
            SyncResult.Error(e.message ?: "clone failed")
        } catch (e: Exception) {
            // JGit throws plain java.io/java.lang exceptions too (e.g.
            // an unreachable host) -- never let one of those escape as
            // an unhandled crash from a suspend function a Compose
            // screen is calling.
            SyncResult.Error(e.message ?: e.javaClass.simpleName)
        }
    }

    /** Fetch + fast-forward-only merge. Calls [ensureCloned] first if
     * this is the very first sync. */
    suspend fun sync(): SyncResult = withContext(Dispatchers.IO) {
        if (!isCloned()) return@withContext ensureCloned()
        try {
            Git.open(repoDir).use { git ->
                git.fetch().call()
                val repo = git.repository
                val target = repo.resolve("origin/$branch")
                    ?: return@withContext SyncResult.Error("origin/$branch not found after fetch")
                val before = repo.resolve("HEAD")
                val result = git.merge()
                    .include(target)
                    .setFastForward(MergeCommand.FastForwardMode.FF_ONLY)
                    .call()
                when {
                    !result.mergeStatus.isSuccessful ->
                        SyncResult.Error("non-fast-forward: ${result.mergeStatus}")
                    before == repo.resolve("HEAD") -> SyncResult.UpToDate
                    else -> SyncResult.Updated
                }
            }
        } catch (e: GitAPIException) {
            SyncResult.Error(e.message ?: "sync failed")
        } catch (e: Exception) {
            SyncResult.Error(e.message ?: e.javaClass.simpleName)
        }
    }
}
