package org.jasonedelman.writtenworld.oauth

import android.util.Log
import kotlinx.coroutines.CompletableDeferred

private const val TAG = "AtprotoOAuth"

// In-process handoff between the screen that starts a login
// (AtprotoOAuthClient.beginLogin, running inside a Composable's
// coroutine scope) and OAuthCallbackActivity (a separate Activity the
// OS launches when the Custom Tab redirects back into this app) --
// they can't pass this data through an Intent extra since PendingAuth
// carries a PKCE code_verifier that must never appear in a URI or
// system-visible Intent log. See OAuthCallbackActivity's own doc
// comment for the real, disclosed limitation this in-memory-only
// design carries.
object OAuthPendingAuthHolder {
    @Volatile private var pending: AtprotoOAuthClient.PendingAuth? = null
    @Volatile private var resultDeferred: CompletableDeferred<Result<OAuthTokenStore.Session>>? = null

    /** Called by the login screen right after AtprotoOAuthClient.beginLogin
     * returns, before the Custom Tab gains focus. */
    fun start(auth: AtprotoOAuthClient.PendingAuth): CompletableDeferred<Result<OAuthTokenStore.Session>> {
        Log.d(TAG, "OAuthPendingAuthHolder.start state=${auth.state}")
        pending = auth
        val deferred = CompletableDeferred<Result<OAuthTokenStore.Session>>()
        resultDeferred = deferred
        return deferred
    }

    /** Called once by OAuthCallbackActivity -- consumes the pending
     * auth so a stray duplicate redirect can't replay the same code. */
    fun take(): AtprotoOAuthClient.PendingAuth? {
        val p = pending
        pending = null
        Log.d(TAG, "OAuthPendingAuthHolder.take -> ${if (p != null) "state=${p.state}" else "null"}")
        return p
    }

    fun reportResult(result: Result<OAuthTokenStore.Session>) {
        resultDeferred?.complete(result)
        resultDeferred = null
    }
}
