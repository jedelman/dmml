package org.jasonedelman.writtenworld.oauth

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

// Persisted counterpart to OAuthPendingAuthHolder's in-memory-only
// storage -- real, on-device-confirmed necessity, not a hypothetical
// edge case: a real login took ~76 real seconds on Bluesky's own
// login page, and Android killed this app's backgrounded process
// during that wait (real, normal OS behavior, not a bug), losing the
// in-memory PendingAuth before the redirect ever came back. Carries
// the PKCE code_verifier, so EncryptedSharedPreferences, same pattern
// as ApiKeyStore/OAuthTokenStore -- not plaintext.
//
// OAuthCallbackActivity checks OAuthPendingAuthHolder (fast path, same
// process) first; only falls back to this persisted store when that's
// empty, i.e. exactly the process-was-killed case above.
object OAuthPendingAuthStore {
    private const val PREFS_NAME = "atproto_oauth_pending"
    private const val KEY_STATE = "state"
    private const val KEY_CODE_VERIFIER = "code_verifier"
    private const val KEY_TOKEN_ENDPOINT = "token_endpoint"
    private const val KEY_AUTH_SERVER_ISSUER = "auth_server_issuer"
    private const val KEY_PDS_ENDPOINT = "pds_endpoint"
    private const val KEY_DID = "did"

    private fun prefs(context: Context): SharedPreferences {
        val masterKey = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        return EncryptedSharedPreferences.create(
            context,
            PREFS_NAME,
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )
    }

    fun save(context: Context, pending: AtprotoOAuthClient.PendingAuth) {
        prefs(context).edit()
            .putString(KEY_STATE, pending.state)
            .putString(KEY_CODE_VERIFIER, pending.codeVerifier)
            .putString(KEY_TOKEN_ENDPOINT, pending.tokenEndpoint)
            .putString(KEY_AUTH_SERVER_ISSUER, pending.authServerIssuer)
            .putString(KEY_PDS_ENDPOINT, pending.pdsEndpoint)
            .putString(KEY_DID, pending.did)
            .apply()
    }

    /** Returns the persisted PendingAuth only if its own `state`
     * matches `expectedState` (the redirect's own state param) --
     * refuses a stale or mismatched entry rather than ever completing
     * against the wrong pending login. */
    fun loadMatching(context: Context, expectedState: String): AtprotoOAuthClient.PendingAuth? {
        val p = prefs(context)
        val state = p.getString(KEY_STATE, null) ?: return null
        if (state != expectedState) return null
        val codeVerifier = p.getString(KEY_CODE_VERIFIER, null) ?: return null
        val tokenEndpoint = p.getString(KEY_TOKEN_ENDPOINT, null) ?: return null
        val authServerIssuer = p.getString(KEY_AUTH_SERVER_ISSUER, null) ?: return null
        val pdsEndpoint = p.getString(KEY_PDS_ENDPOINT, null) ?: return null
        val did = p.getString(KEY_DID, null) ?: return null
        return AtprotoOAuthClient.PendingAuth(state, codeVerifier, tokenEndpoint, authServerIssuer, pdsEndpoint, did)
    }

    fun clear(context: Context) {
        prefs(context).edit().clear().apply()
    }
}
