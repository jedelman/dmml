package org.jasonedelman.writtenworld.oauth

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.widget.Toast
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

// Receives the org.jason-edelman.written-world:/callback redirect
// (registered as an intent-filter in AndroidManifest.xml, matching
// client-metadata.json's redirect_uris -- both MUST match exactly, per
// spec) once the user finishes logging in on the PDS/authorization
// server's own hosted page inside the Custom Tab. This activity does
// not render a UI of its own -- it exchanges the code for tokens (via
// AtprotoOAuthClient.completeLogin, using the PendingAuth this same
// process's OAuthPendingAuthHolder stashed when the flow began) and
// immediately finishes, returning control to whichever screen started
// the login.
//
// Real, disclosed limitation: PendingAuth lives in an in-memory
// singleton (OAuthPendingAuthHolder), not persisted storage -- if the
// OS kills this app's process while the Custom Tab has focus (rare,
// since the originating process is still the Custom Tab's back stack,
// but real), the redirect arrives with no PendingAuth to complete
// against and the login must be retried from the beginning. Fine for
// this verification pass; a production app would persist code_verifier
// + state + endpoints instead so an interrupted flow could resume.
class OAuthCallbackActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        handleRedirect(intent)
        finish()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleRedirect(intent)
        finish()
    }

    private fun handleRedirect(intent: Intent?) {
        val uri = intent?.data
        val pending = OAuthPendingAuthHolder.take()
        if (uri == null || pending == null) {
            Toast.makeText(this, "OAuth redirect arrived with no pending login -- retry from the login screen", Toast.LENGTH_LONG).show()
            return
        }
        val error = uri.getQueryParameter("error")
        if (error != null) {
            OAuthPendingAuthHolder.reportResult(Result.failure(IllegalStateException("authorization server returned error: $error ${uri.getQueryParameter("error_description")}")))
            return
        }
        val code = uri.getQueryParameter("code")
        val state = uri.getQueryParameter("state")
        if (code == null || state == null) {
            OAuthPendingAuthHolder.reportResult(Result.failure(IllegalStateException("redirect missing code/state: $uri")))
            return
        }
        // Token exchange is a real blocking network call -- must not
        // run on the main thread (NetworkOnMainThreadException, real,
        // not hypothetical).
        CoroutineScope(Dispatchers.IO).launch {
            OAuthPendingAuthHolder.reportResult(
                runCatching { AtprotoOAuthClient.completeLogin(pending, state, code) }
            )
        }
    }
}
