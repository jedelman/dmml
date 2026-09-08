package org.writtenworld.androidpoc

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.writtenworld.androidpoc.world.SyncResult
import org.writtenworld.androidpoc.world.WorldRepository

// Placeholder -- no canonical world repo exists yet
// (dmml/dev-journal/2026-09-07-android-canonical-structure-decisions.md
// is accepted, but Jason's own repo hasn't been created/pointed to).
// Deliberately obviously-fake rather than a real-looking URL, so this
// can't be mistaken for a working default if it's ever left unchanged.
private const val PLACEHOLDER_WORLD_REPO_URL = "https://example.invalid/replace-with-real-world-repo.git"

// Real self-node and firing identity are both still open per the
// canonical structure decision doc (git identity = the player's
// atproto DID, confirmed decided; the DMML *self* node used for guard
// evaluation is a related but distinct question, not yet resolved to a
// real per-player value). Placeholder until that's wired.
private const val SELF_NODE = "player/one"

/**
 * Real git-sync browsing, replacing the F1-era hardcoded
 * `WORLD_SRC`/`MACHINE_SRC` + in-memory `*History` functions with a
 * real [WorldRepository] clone and [DmmlBridge]'s directory-based
 * (`*Dir`) functions -- dmml/dev-journal/2026-09-07-android-jgit-sync-
 * spec.md, refined by 2026-09-07-android-canonical-structure-
 * decisions.md.
 *
 * Deliberately READ-ONLY, matching `written-world look`'s own scope:
 * tapping an action calls [DmmlBridge.fireDir] and shows the resulting
 * commit as a PREVIEW ("what firing this would produce") -- it is never
 * written back to the synced clone. Real persistence is the separate,
 * not-yet-built authoring worktree's job
 * (2026-09-07-android-authoring-loop-spec.md) -- keeping this screen
 * read-only is a deliberate scope boundary, not an oversight.
 */
@Composable
fun GameScreen() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val repo = remember { WorldRepository(context, PLACEHOLDER_WORLD_REPO_URL) }

    var syncStatus by remember { mutableStateOf("not synced yet") }
    var rendered by remember { mutableStateOf("") }
    // Each entry: Pair(machineNode, transitionIdent) -- actionsDir
    // renders "machineNode/transitionIdent" lines, and a machine node
    // can itself contain "/", so the split has to happen on the LAST
    // "/" only (a real transition ident is always a single, slash-free
    // identifier -- DMML.Surface's own grammar guarantees this).
    var actions by remember { mutableStateOf(listOf<Pair<String, String>>()) }
    var loadError by remember { mutableStateOf<String?>(null) }
    var previewText by remember { mutableStateOf<String?>(null) }

    fun reloadFromDisk() {
        val dir = repo.commitsDir.path
        val renderedResult = DmmlBridge.renderDir(dir)
        if (DmmlBridge.isError(renderedResult)) {
            loadError = renderedResult.removePrefix("ERROR: ")
            rendered = ""
            actions = emptyList()
            return
        }
        loadError = null
        rendered = renderedResult
        val actionsRaw = DmmlBridge.actionsDir(dir, SELF_NODE)
        actions = if (DmmlBridge.isError(actionsRaw)) {
            emptyList()
        } else {
            actionsRaw.lines().filter { it.isNotBlank() }.map { line ->
                val idx = line.lastIndexOf('/')
                Pair(line.substring(0, idx), line.substring(idx + 1))
            }
        }
    }

    fun doSync() {
        scope.launch {
            syncStatus = "syncing..."
            when (val result = repo.sync()) {
                is SyncResult.UpToDate -> syncStatus = "up to date"
                is SyncResult.Updated -> syncStatus = "synced"
                is SyncResult.Error -> syncStatus = "sync failed: ${result.reason}"
            }
            reloadFromDisk()
        }
    }

    LaunchedEffect(Unit) { doSync() }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Row {
            Text("dmml -- browsing", style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f, fill = true))
            OutlinedButton(onClick = { doSync() }) { Text("Sync") }
        }
        Text(syncStatus, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Spacer(Modifier.height(8.dp))

        loadError?.let { err ->
            Text("refused: $err", color = MaterialTheme.colorScheme.error)
            Spacer(Modifier.height(8.dp))
        }

        Text(
            text = rendered,
            fontFamily = FontFamily.Monospace,
            modifier = Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState())
        )

        previewText?.let { preview ->
            Spacer(Modifier.height(8.dp))
            Text("-- preview (not applied) --", style = MaterialTheme.typography.labelMedium)
            Text(preview, fontFamily = FontFamily.Monospace, modifier = Modifier.verticalScroll(rememberScrollState()))
        }

        Spacer(Modifier.height(16.dp))

        if (actions.isEmpty()) {
            Text("Nothing more you can do here.", color = MaterialTheme.colorScheme.onSurfaceVariant)
        } else {
            actions.forEach { (machineNode, transitionIdent) ->
                Button(
                    onClick = {
                        val result = DmmlBridge.fireDir(repo.commitsDir.path, SELF_NODE, machineNode, transitionIdent)
                        previewText = if (DmmlBridge.isError(result)) {
                            "refused: ${result.removePrefix("ERROR: ")}"
                        } else {
                            result
                        }
                    },
                    modifier = Modifier.fillMaxWidth().padding(vertical = 6.dp).heightIn(min = 56.dp)
                ) {
                    Text(transitionIdent, style = MaterialTheme.typography.titleMedium)
                }
            }
        }
    }
}
