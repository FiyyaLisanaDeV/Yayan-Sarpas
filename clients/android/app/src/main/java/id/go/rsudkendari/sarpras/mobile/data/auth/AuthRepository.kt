package id.go.rsudkendari.sarpras.mobile.data.auth

import id.go.rsudkendari.sarpras.mobile.data.local.ServerPrefs
import id.go.rsudkendari.sarpras.mobile.data.local.TokenStore
import id.go.rsudkendari.sarpras.mobile.data.model.ApiException
import id.go.rsudkendari.sarpras.mobile.data.model.LoginRequest
import id.go.rsudkendari.sarpras.mobile.data.model.LogoutRequest
import id.go.rsudkendari.sarpras.mobile.data.model.User
import id.go.rsudkendari.sarpras.mobile.data.remote.ApiClient
import id.go.rsudkendari.sarpras.mobile.data.repo.MobileRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.withContext

class AuthRepository(
    private val apiClient: ApiClient,
    private val tokenStore: TokenStore,
    private val serverPrefs: ServerPrefs,
    private val mobileRepository: MobileRepository? = null,
) {
    private val api get() = apiClient.api
    private val baseUrlHolder get() = apiClient.baseUrlHolder

    fun currentUser(): User? = tokenStore.getUser()
    fun isLoggedIn(): Boolean = tokenStore.getAccess() != null && tokenStore.getUser() != null
    val apiBaseFlow: Flow<String> = serverPrefs.apiBaseFlow

    suspend fun initBaseUrl() {
        val base = serverPrefs.getApiBase()
        baseUrlHolder.set(base)
    }

    suspend fun login(serverUrl: String, username: String, password: String): Result<User> =
        withContext(Dispatchers.IO) {
            val base = ServerPrefs.normalize(serverUrl)
            serverPrefs.setApiBase(base)
            baseUrlHolder.set(base)
            runCatching {
                val res = api.login(LoginRequest(username.trim(), password))
                val body = res.body()
                if (!res.isSuccessful || body?.success != true || body.data == null) {
                    throw ApiException(
                        body?.error?.code,
                        body?.error?.message ?: "Gagal masuk (HTTP ${res.code()})",
                        res.code(),
                    )
                }
                tokenStore.save(body.data.accessToken, body.data.refreshToken, body.data.user)
                body.data.user
            }
        }

    suspend fun restoreSession(): Result<User?> = withContext(Dispatchers.IO) {
        initBaseUrl()
        if (tokenStore.getAccess() == null) return@withContext Result.success(null)
        runCatching {
            val res = api.me()
            val body = res.body()
            if (res.isSuccessful && body?.success == true && body.data != null) {
                tokenStore.updateUser(body.data)
                body.data
            } else {
                tokenStore.clear()
                null
            }
        }.recover {
            // Keep cached user if network issue but token may still be valid; cleared on 401 by authenticator.
            tokenStore.getUser()
        }
    }

    suspend fun logout() = withContext(Dispatchers.IO) {
        val refresh = tokenStore.getRefresh()
        if (refresh != null) {
            runCatching { api.logout(LogoutRequest(refresh)) }
        }
        tokenStore.clear()
        mobileRepository?.clearCache()
    }
}
