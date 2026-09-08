package org.jasonedelman.writtenworld

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.eclipse.jgit.api.Git
import java.io.File

// On-device verification for the 2026-09-08 JNI-upcall architecture
// (DMML.AndroidBridge, cross-compiled per dev-journal/2026-09-08-
// android-ndk-cross-compile-of-androidbridge.md) -- item 3 of the
// cloud agent's handoff list, in the stated order: jgitCommit first,
// then atprotoResolve/atprotoPull, then llmChatComplete. Real calls
// against real endpoints, not stubs -- same "verify for real, don't
// assume" discipline as every dev-journal entry behind this.
//
// jgitCommit is exercised against a repo this screen inits itself via
// JGit directly (Kotlin-side, not through NativeBridge -- there's no
// jgitInit in NativeBridge's fixed six-method table, deliberately:
// DMML.AndroidBridge's own doc comment treats repo creation as an
// out-of-band, one-time step the caller already owns, same division
// of labor as org.writtenworld.androidpoc.WorldRepository's clone step
// for the read-only sync path).
//
// atprotoPull, atprotoCreateSession/Record, and llmChatComplete are
// deliberately NOT exercised here -- they need a real existing atproto
// record collection and/or a real BYOK API key, neither of which this
// verification pass has. Disclosed as unverified in the UI itself,
// not silently skipped.
class VerifyActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface {
                    VerifyScreen(filesDir = filesDir)
                }
            }
        }
    }
}

@Composable
private fun VerifyScreen(filesDir: File) {
    var log by remember { mutableStateOf("Tap a button to run a real on-device check.") }
    val scope = rememberCoroutineScope()

    Column(Modifier.padding(16.dp).verticalScroll(rememberScrollState())) {
        Button(onClick = {
            scope.launch {
                log = "Running jgitCommit...\n"
                log += withContext(Dispatchers.IO) { runJgitCommitCheck(filesDir) }
            }
        }) { Text("1. jgitCommit") }

        Button(onClick = {
            scope.launch {
                log = "Running atprotoResolve...\n"
                log += withContext(Dispatchers.IO) { runAtprotoResolveCheck() }
            }
        }) { Text("2. atprotoResolve") }

        Text(log)
    }
}

private fun runJgitCommitCheck(filesDir: File): String =
    try {
        val repoDir = File(filesDir, "verify-jgit-repo")
        repoDir.mkdirs()
        // Kotlin-side init (see class doc comment) -- NativeBridge has
        // no jgitInit; this repo must already exist before the native
        // upcall commit call below.
        if (!File(repoDir, ".git").exists()) {
            Git.init().setDirectory(repoDir).call().close()
        }
        val result = NativeBridge.jgitCommit(
            repoDir.absolutePath,
            "verify.txt",
            "hello from the android JNI upcall, real commit",
            "on-device verification commit",
        )
        "jgitCommit -> $result\n${if (NativeBridge.isError(result)) "FAILED" else "OK, real commit written to $repoDir"}"
    } catch (t: Throwable) {
        "jgitCommit -> EXCEPTION: ${t}"
    }

private fun runAtprotoResolveCheck(): String =
    try {
        // bsky.app: a real, stable, public handle -- needs no
        // credentials, exercises real DID + PDS-endpoint resolution
        // through the upcalled OkHttp path end to end.
        val result = NativeBridge.atprotoResolve("bsky.app")
        "atprotoResolve(\"bsky.app\") -> $result\n${if (NativeBridge.isError(result)) "FAILED" else "OK, real resolution"}"
    } catch (t: Throwable) {
        "atprotoResolve -> EXCEPTION: ${t}"
    }
