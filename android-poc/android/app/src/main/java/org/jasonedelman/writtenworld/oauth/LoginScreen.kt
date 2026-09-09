package org.jasonedelman.writtenworld.oauth

import android.content.Context
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch

// Real atproto OAuth login (PAR + PKCE + DPoP) -- see
// AtprotoOAuthClient's own doc comment for the protocol details and
// dev-journal/2026-09-09-atproto-oauth-login-screen.md for what's
// verified vs. still open. Not the app-password path
// (NativeBridge.atprotoCreateSession) -- Jason chose OAuth explicitly,
// the current Bluesky-recommended path for third-party apps.
@Composable
fun LoginScreen(context: Context, onLoggedIn: (OAuthTokenStore.Session) -> Unit) {
    var handle by remember { mutableStateOf("") }
    var status by remember { mutableStateOf(OAuthTokenStore.load(context)?.let { "Already logged in as ${it.did}" } ?: "") }
    val scope = rememberCoroutineScope()

    Column(Modifier.padding(16.dp)) {
        Text("Log in with your atproto handle or DID:")
        OutlinedTextField(
            value = handle,
            onValueChange = { handle = it },
            singleLine = true,
            placeholder = { Text("e.g. jasonedelman.bsky.social") },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Text),
        )
        Button(onClick = {
            scope.launch {
                status = "Resolving + starting OAuth..."
                try {
                    val pending = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                        AtprotoOAuthClient.beginLogin(context, handle.trim())
                    }
                    val deferred = OAuthPendingAuthHolder.start(pending)
                    status = "Continue in the browser tab that just opened..."
                    val result = deferred.await()
                    result.fold(
                        onSuccess = { session ->
                            OAuthTokenStore.save(context, session)
                            status = "Logged in as ${session.did}"
                            onLoggedIn(session)
                        },
                        onFailure = { err ->
                            status = "Login failed: $err"
                        },
                    )
                } catch (t: Throwable) {
                    status = "Login failed: $t"
                }
            }
        }) { Text("Log in with Bluesky") }

        Button(onClick = {
            OAuthTokenStore.clear(context)
            status = "Logged out."
        }) { Text("Log out") }

        Text(status)
    }
}
