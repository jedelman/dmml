package org.jasonedelman.writtenworld.oauth

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

// Stores the OAuth session's non-key material -- the DPoP private key
// itself never lands here at all, it stays in AndroidKeyStore (see
// DpopKeyManager). What IS stored here (access token, refresh token,
// DID, PDS endpoint, authorization server issuer) are real bearer-ish
// secrets in their own right -- DPoP binding makes a stolen access/
// refresh token useless without the matching private key, but they're
// still sensitive, so EncryptedSharedPreferences, same pattern as
// ApiKeyStore.
object OAuthTokenStore {
    private const val PREFS_NAME = "atproto_oauth_session"
    private const val KEY_ACCESS_TOKEN = "access_token"
    private const val KEY_REFRESH_TOKEN = "refresh_token"
    private const val KEY_DID = "did"
    private const val KEY_PDS_ENDPOINT = "pds_endpoint"
    private const val KEY_AUTH_SERVER_ISSUER = "auth_server_issuer"

    data class Session(
        val accessToken: String,
        val refreshToken: String,
        val did: String,
        val pdsEndpoint: String,
        val authServerIssuer: String,
    )

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

    /** `commit()`, not `apply()` -- this is called right before
     * OAuthCallbackActivity finishes (its whole job is done and the
     * process is a real backgrounding/kill candidate immediately
     * after), same real async-write-loss risk `apply()` had in
     * OAuthPendingAuthStore.save, confirmed on-device 2026-09-09. */
    fun save(context: Context, session: Session) {
        prefs(context).edit()
            .putString(KEY_ACCESS_TOKEN, session.accessToken)
            .putString(KEY_REFRESH_TOKEN, session.refreshToken)
            .putString(KEY_DID, session.did)
            .putString(KEY_PDS_ENDPOINT, session.pdsEndpoint)
            .putString(KEY_AUTH_SERVER_ISSUER, session.authServerIssuer)
            .commit()
    }

    fun load(context: Context): Session? {
        val p = prefs(context)
        val accessToken = p.getString(KEY_ACCESS_TOKEN, null) ?: return null
        val refreshToken = p.getString(KEY_REFRESH_TOKEN, null) ?: return null
        val did = p.getString(KEY_DID, null) ?: return null
        val pdsEndpoint = p.getString(KEY_PDS_ENDPOINT, null) ?: return null
        val authServerIssuer = p.getString(KEY_AUTH_SERVER_ISSUER, null) ?: return null
        return Session(accessToken, refreshToken, did, pdsEndpoint, authServerIssuer)
    }

    fun clear(context: Context) {
        prefs(context).edit().clear().apply()
    }
}
