package org.jasonedelman.writtenworld

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

// BYOK (bring-your-own-key) storage for NativeBridge.llmChatComplete's
// OpenRouter API key -- a real secret, so it goes into
// EncryptedSharedPreferences (a real Android Keystore-backed AES key
// wraps the file on disk) rather than plain SharedPreferences, which
// stores values as cleartext XML any app with root or a backup-extraction
// exploit could read. This module holds no key of its own -- Jason
// supplies the actual value himself via VerifyScreen's UI; nothing here
// hardcodes, generates, or invents one.
object ApiKeyStore {
    private const val PREFS_NAME = "byok_secrets"
    private const val KEY_OPENROUTER_API_KEY = "openrouter_api_key"

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

    fun getApiKey(context: Context): String? =
        prefs(context).getString(KEY_OPENROUTER_API_KEY, null)?.takeIf { it.isNotBlank() }

    fun setApiKey(context: Context, apiKey: String) {
        prefs(context).edit().putString(KEY_OPENROUTER_API_KEY, apiKey).apply()
    }

    fun clearApiKey(context: Context) {
        prefs(context).edit().remove(KEY_OPENROUTER_API_KEY).apply()
    }
}
