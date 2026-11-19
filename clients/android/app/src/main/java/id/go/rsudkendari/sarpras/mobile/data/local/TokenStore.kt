package id.go.rsudkendari.sarpras.mobile.data.local

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import id.go.rsudkendari.sarpras.mobile.data.model.User
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

class TokenStore(context: Context) {
    private val json = Json { ignoreUnknownKeys = true }

    private val prefs: SharedPreferences by lazy {
        try {
            val masterKey = MasterKey.Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()
            EncryptedSharedPreferences.create(
                context,
                "sarpras_secure",
                masterKey,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
            )
        } catch (_: Exception) {
            context.getSharedPreferences("sarpras_secure_fallback", Context.MODE_PRIVATE)
        }
    }

    @Synchronized
    fun getAccess(): String? = prefs.getString(KEY_ACCESS, null)

    @Synchronized
    fun getRefresh(): String? = prefs.getString(KEY_REFRESH, null)

    @Synchronized
    fun getUser(): User? = prefs.getString(KEY_USER, null)?.let {
        runCatching { json.decodeFromString<User>(it) }.getOrNull()
    }

    @Synchronized
    fun save(access: String, refresh: String, user: User) {
        prefs.edit()
            .putString(KEY_ACCESS, access)
            .putString(KEY_REFRESH, refresh)
            .putString(KEY_USER, json.encodeToString(user))
            .apply()
    }

    @Synchronized
    fun updateTokens(access: String, refresh: String) {
        prefs.edit()
            .putString(KEY_ACCESS, access)
            .putString(KEY_REFRESH, refresh)
            .apply()
    }

    @Synchronized
    fun updateUser(user: User) {
        prefs.edit().putString(KEY_USER, json.encodeToString(user)).apply()
    }

    @Synchronized
    fun clear() {
        prefs.edit().clear().apply()
    }

    companion object {
        private const val KEY_ACCESS = "access_token"
        private const val KEY_REFRESH = "refresh_token"
        private const val KEY_USER = "user"
    }
}
