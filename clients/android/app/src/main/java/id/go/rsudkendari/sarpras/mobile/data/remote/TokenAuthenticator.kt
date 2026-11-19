package id.go.rsudkendari.sarpras.mobile.data.remote

import id.go.rsudkendari.sarpras.mobile.data.local.TokenStore
import id.go.rsudkendari.sarpras.mobile.data.model.RefreshRequest
import kotlinx.coroutines.runBlocking
import okhttp3.Authenticator
import okhttp3.Request
import okhttp3.Response
import okhttp3.Route

class TokenAuthenticator(
    private val tokenStore: TokenStore,
    private val refreshApiProvider: () -> SarprasApi,
) : Authenticator {

    override fun authenticate(route: Route?, response: Response): Request? {
        if (responseCount(response) >= 2) return null
        val path = response.request.url.encodedPath
        if (path == "/auth/login" || path == "/auth/refresh") return null

        synchronized(this) {
            val refreshToken = tokenStore.getRefresh() ?: return null
            val currentAccess = tokenStore.getAccess()
            val reqAccess = response.request.header("Authorization")?.removePrefix("Bearer ")
            // Token was refreshed by another request; retry with current token.
            if (!currentAccess.isNullOrBlank() && currentAccess != reqAccess) {
                return response.request.newBuilder()
                    .header("Authorization", "Bearer $currentAccess")
                    .build()
            }

            val result = runCatching {
                runBlocking { refreshApiProvider().refresh(RefreshRequest(refreshToken)) }
            }.getOrNull() ?: return fail()

            val body = result.body()
            if (!result.isSuccessful || body?.success != true || body.data == null) {
                return fail()
            }

            tokenStore.updateTokens(body.data.accessToken, body.data.refreshToken)
            tokenStore.updateUser(body.data.user)
            return response.request.newBuilder()
                .header("Authorization", "Bearer ${body.data.accessToken}")
                .build()
        }
    }

    private fun fail(): Request? {
        tokenStore.clear()
        return null
    }

    private fun responseCount(response: Response): Int {
        var count = 1
        var prior = response.priorResponse
        while (prior != null) {
            count++
            prior = prior.priorResponse
        }
        return count
    }
}
