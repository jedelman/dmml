package org.jasonedelman.writtenworld

import android.content.Context
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.eclipse.jgit.api.Git
import java.io.File

// On-device verification for the 2026-09-08 JNI-upcall architecture
// (DMML.AndroidBridge, cross-compiled per dev-journal/2026-09-08-
// android-ndk-cross-compile-of-androidbridge.md) -- items 3 and 4 of
// the cloud agent's handoff list: on-device verification of all six
// upcall entry points (jgitCommit, atprotoResolve, atprotoPull,
// atprotoCreateSession/Record, llmChatComplete), then Broker.hs's
// `incorporate` orchestration ported onto the same upcall (see
// DMML.AndroidBridge.brokerIncorporateBridge). Real calls against real
// endpoints, not stubs -- same "verify for real, don't assume"
// discipline as every dev-journal entry behind this.
//
// jgitCommit/brokerIncorporate are exercised against a repo this
// screen inits itself via JGit directly (Kotlin-side, not through
// NativeBridge -- there's no jgitInit in NativeBridge's fixed table,
// deliberately: DMML.AndroidBridge's own doc comment treats repo
// creation as an out-of-band, one-time step the caller already owns).
//
// llmChatComplete needs a real BYOK OpenRouter API key -- entered here
// and stored via ApiKeyStore (EncryptedSharedPreferences), never
// hardcoded. atprotoCreateSession/Record still need real atproto
// credentials this screen has no UI for yet -- not exercised.
class VerifyActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface {
                    VerifyScreen(filesDir = filesDir, context = this)
                }
            }
        }
    }
}

@Composable
private fun VerifyScreen(filesDir: File, context: Context) {
    var log by remember { mutableStateOf("Tap a button to run a real on-device check.") }
    var apiKeyField by remember { mutableStateOf(ApiKeyStore.getApiKey(context) ?: "") }
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

        Button(onClick = {
            scope.launch {
                log = "Running brokerIncorporate...\n"
                log += withContext(Dispatchers.IO) { runBrokerIncorporateCheck(filesDir) }
            }
        }) { Text("3. brokerIncorporate") }

        HorizontalDivider(Modifier.padding(vertical = 12.dp))

        Text("OpenRouter API key (BYOK, stored encrypted on-device):")
        Row {
            OutlinedTextField(
                value = apiKeyField,
                onValueChange = { apiKeyField = it },
                singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                modifier = Modifier.weight(1f),
            )
            Button(onClick = {
                ApiKeyStore.setApiKey(context, apiKeyField)
                log = "API key saved (encrypted on-device)."
            }) { Text("Save") }
        }

        Button(onClick = {
            scope.launch {
                log = "Running llmChatComplete...\n"
                log += withContext(Dispatchers.IO) { runLlmChatCompleteCheck(context) }
            }
        }) { Text("4. llmChatComplete") }

        Text(log)
    }
}

private fun runJgitCommitCheck(filesDir: File): String =
    try {
        val repoDir = verifyRepoDir(filesDir)
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

private fun runBrokerIncorporateCheck(filesDir: File): String =
    try {
        val repoDir = verifyRepoDir(filesDir)
        // bsky.app almost certainly has no records in the
        // org.jason-edelman.writtenworld.commit collection -- this
        // still exercises the real pull + (likely empty) incorporate
        // path end to end; a genuinely populated peer is real follow-up
        // work once one exists to point this at.
        val result = NativeBridge.brokerIncorporate(repoDir.absolutePath, "bsky.app", "verify-cursor.txt", "commits")
        "brokerIncorporate(repo, \"bsky.app\", ...) -> $result\n${if (NativeBridge.isError(result)) "FAILED" else "OK, real pull + incorporate attempt"}"
    } catch (t: Throwable) {
        "brokerIncorporate -> EXCEPTION: ${t}"
    }

private fun runLlmChatCompleteCheck(context: Context): String {
    val apiKey = ApiKeyStore.getApiKey(context)
        ?: return "llmChatComplete -> SKIPPED: no API key saved yet. Enter one above and tap Save."
    return try {
        val result = NativeBridge.llmChatComplete(
            apiKey,
            "deepseek/deepseek-v4-flash-0731",
            "You are a terse test assistant.",
            "Reply with exactly the word: pong",
        )
        "llmChatComplete -> $result\n${if (NativeBridge.isError(result)) "FAILED" else "OK, real BYOK chat completion"}"
    } catch (t: Throwable) {
        "llmChatComplete -> EXCEPTION: ${t}"
    }
}

// Shared by jgitCommit and brokerIncorporate -- both need an already-
// `git init`'d repo (see class doc comment); idempotent, safe to call
// from either check in either order.
private fun verifyRepoDir(filesDir: File): File {
    val repoDir = File(filesDir, "verify-jgit-repo")
    repoDir.mkdirs()
    if (!File(repoDir, ".git").exists()) {
        Git.init().setDirectory(repoDir).call().close()
    }
    return repoDir
}
