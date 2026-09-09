package org.jasonedelman.writtenworld.oauth

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.widget.Toast
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

// Receives the org.jason-edelman:/callback redirect (registered as an
// intent-filter in AndroidManifest.xml, matching client-metadata.json's
// redirect_uris -- both MUST match exactly, per spec) once the user
// finishes logging in on the PDS/authorization server's own hosted
// page inside the Custom Tab. This activity does not render a UI of
// its own -- it exchanges the code for tokens and immediately
// finishes, returning control to whichever screen started the login.
//
// Two real paths, both exercised on-device (2026-09-09): the fast
// path uses OAuthPendingAuthHolder (same process, in-memory,
// LoginScreen's own coroutine is still alive to show the result
// immediately). The fallback path uses OAuthPendingAuthStore
// (persisted, EncryptedSharedPreferences) -- needed because a real
// login on Bluesky's own page took ~76 real seconds, and Android
// killed this app's backgrounded process during that wait (real,
// normal OS behavior), losing the in-memory copy before the redirect
// came back. In the fallback path there's no live LoginScreen
// coroutine left to notify, so this activity completes the exchange
// itself, saves the session directly via OAuthTokenStore, and shows a
// Toast -- the user sees "Already logged in as ..." the next time
// LoginScreen composes (it reads OAuthTokenStore on init), not a live
// status update.
//
// Real, on-device-confirmed complication (2026-09-09): Chrome/Custom
// Tabs can redeliver this activity's VIEW intent more than once for a
// SINGLE completed authorization (same code/state each time) --
// onCreate then onNewIntent, or multiple onNewIntent calls. Since
// OAuthPendingAuthHolder.take() consumes its entry, only the first
// delivery can use the fast path; a second delivery arriving before
// the first's async completeLogin() call has finished (and cleared
// OAuthPendingAuthStore) can still find a match on disk and attempt a
// SECOND token exchange with the same, now-already-used authorization
// code -- which the real authorization server will reject (auth codes
// are single-use). Logged verbosely (tag "AtprotoOAuth") specifically
// to see this sequence for real rather than guess at it.
private const val TAG = "AtprotoOAuth"

class OAuthCallbackActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Log.d(TAG, "OAuthCallbackActivity.onCreate intent=${intent?.data}")
        handleRedirect(intent)
        finish()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        Log.d(TAG, "OAuthCallbackActivity.onNewIntent intent=${intent.data}")
        handleRedirect(intent)
        finish()
    }

    private fun handleRedirect(intent: Intent?) {
        val appContext = applicationContext
        val uri = intent?.data
        if (uri == null) {
            Log.e(TAG, "handleRedirect: no data URI on intent")
            Toast.makeText(appContext, "OAuth redirect arrived with no data URI", Toast.LENGTH_LONG).show()
            return
        }
        val error = uri.getQueryParameter("error")
        val code = uri.getQueryParameter("code")
        val state = uri.getQueryParameter("state")
        Log.d(TAG, "handleRedirect: error=$error code=${code?.take(8)}... state=$state")

        val pendingFromMemory = OAuthPendingAuthHolder.take()
        Log.d(TAG, "handleRedirect: OAuthPendingAuthHolder.take() -> ${if (pendingFromMemory != null) "HIT (state=${pendingFromMemory.state})" else "MISS"}")
        if (pendingFromMemory != null) {
            // Fast path: same process, LoginScreen's coroutine is
            // still alive and awaiting this.
            if (error != null) {
                OAuthPendingAuthHolder.reportResult(Result.failure(IllegalStateException("authorization server returned error: $error ${uri.getQueryParameter("error_description")}")))
                return
            }
            if (code == null || state == null) {
                OAuthPendingAuthHolder.reportResult(Result.failure(IllegalStateException("redirect missing code/state: $uri")))
                return
            }
            CoroutineScope(Dispatchers.IO).launch {
                Log.d(TAG, "fast path: calling completeLogin")
                val result = runCatching { AtprotoOAuthClient.completeLogin(pendingFromMemory, state, code) }
                Log.d(TAG, "fast path: completeLogin result = $result")
                result.onSuccess { session ->
                    OAuthTokenStore.save(appContext, session)
                    OAuthPendingAuthStore.clear(appContext)
                    Log.d(TAG, "fast path: saved session, cleared pending store")
                }
                OAuthPendingAuthHolder.reportResult(result)
            }
            return
        }

        // Fallback path: fresh process, or a redundant redelivery of
        // an already-consumed intent -- no live coroutine to notify --
        // complete the login for real anyway if we have a matching
        // persisted PendingAuth, so the login isn't just lost.
        if (state == null) {
            Log.e(TAG, "fallback path: redirect has no state param at all")
            Toast.makeText(appContext, "OAuth redirect arrived with no pending login and no state to recover one -- retry from the login screen", Toast.LENGTH_LONG).show()
            return
        }
        val pendingFromDisk = OAuthPendingAuthStore.loadMatching(this, state)
        Log.d(TAG, "fallback path: OAuthPendingAuthStore.loadMatching(state=$state) -> ${if (pendingFromDisk != null) "HIT" else "MISS"}")
        if (pendingFromDisk == null) {
            Toast.makeText(appContext, "OAuth redirect arrived with no matching pending login (app process was likely killed, or this is a redundant redelivery already consumed) -- retry from the login screen", Toast.LENGTH_LONG).show()
            return
        }
        if (error != null) {
            OAuthPendingAuthStore.clear(this)
            Toast.makeText(appContext, "Login failed: authorization server returned $error", Toast.LENGTH_LONG).show()
            return
        }
        if (code == null) {
            OAuthPendingAuthStore.clear(this)
            Toast.makeText(appContext, "Login failed: redirect missing code", Toast.LENGTH_LONG).show()
            return
        }
        CoroutineScope(Dispatchers.IO).launch {
            Log.d(TAG, "fallback path: calling completeLogin")
            val result = runCatching { AtprotoOAuthClient.completeLogin(pendingFromDisk, state, code) }
            Log.d(TAG, "fallback path: completeLogin result = $result")
            OAuthPendingAuthStore.clear(appContext)
            result.fold(
                onSuccess = { session ->
                    OAuthTokenStore.save(appContext, session)
                    Toast.makeText(appContext, "Logged in as ${session.did}", Toast.LENGTH_LONG).show()
                },
                onFailure = { err ->
                    Toast.makeText(appContext, "Login failed: $err", Toast.LENGTH_LONG).show()
                },
            )
        }
    }
}
