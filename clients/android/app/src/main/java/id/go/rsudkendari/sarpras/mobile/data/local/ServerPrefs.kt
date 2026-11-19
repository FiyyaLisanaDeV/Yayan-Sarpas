package id.go.rsudkendari.sarpras.mobile.data.local

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map

private val Context.dataStore by preferencesDataStore(name = "sarpras_prefs")

class ServerPrefs(private val context: Context) {

    val apiBaseFlow: Flow<String> = context.dataStore.data.map { prefs ->
        normalize(prefs[KEY_API_BASE] ?: DEFAULT_BASE)
    }

    suspend fun getApiBase(): String = normalize(apiBaseFlow.first())

    suspend fun setApiBase(url: String) {
        context.dataStore.edit { prefs ->
            prefs[KEY_API_BASE] = normalize(url)
        }
    }

    companion object {
        private val KEY_API_BASE = stringPreferencesKey("api_base")
        const val DEFAULT_BASE = "http://10.0.2.2:8080"

        fun normalize(url: String): String {
            val t = url.trim().removeSuffix("/")
            return if (t.isBlank()) DEFAULT_BASE else t
        }
    }
}
