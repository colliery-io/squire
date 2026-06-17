package com.squire.pairing

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import kotlinx.serialization.json.Json

/**
 * Keystore-backed encrypted storage for the paired [Session] (NFR-5 / ADR SQUIRE-A-0010).
 *
 * The token lives in `EncryptedSharedPreferences` under an Android-Keystore master key, so it is
 * not readable as plaintext from a backup or a rooted dump. One session per device (single
 * household/member); [clear] "forgets" the device and returns the app to the pairing screen.
 */
class SessionStore(context: Context) {
    private val json = Json { ignoreUnknownKeys = true }

    private val prefs = run {
        val app = context.applicationContext
        val master = MasterKey.Builder(app).setKeyScheme(MasterKey.KeyScheme.AES256_GCM).build()
        EncryptedSharedPreferences.create(
            app,
            "squire_session",
            master,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )
    }

    fun load(): Session? =
        prefs.getString(KEY, null)?.let {
            runCatching { json.decodeFromString(Session.serializer(), it) }.getOrNull()
        }

    fun save(session: Session) {
        prefs.edit().putString(KEY, json.encodeToString(Session.serializer(), session)).apply()
    }

    fun clear() {
        prefs.edit().remove(KEY).apply()
    }

    private companion object {
        const val KEY = "session"
    }
}
