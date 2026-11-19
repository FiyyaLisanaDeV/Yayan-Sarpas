package id.go.rsudkendari.sarpras.mobile.data.remote

import id.go.rsudkendari.sarpras.mobile.data.local.TokenStore
import okhttp3.Interceptor
import okhttp3.Response

class AuthInterceptor(private val tokenStore: TokenStore) : Interceptor {
    override fun intercept(chain: Interceptor.Chain): Response {
        val req = chain.request()
        val path = req.url.encodedPath
        if (path == "/auth/login" || path == "/auth/refresh") {
            return chain.proceed(req)
        }
        val token = tokenStore.getAccess() ?: return chain.proceed(req)
        val authed = req.newBuilder()
            .header("Authorization", "Bearer $token")
            .build()
        return chain.proceed(authed)
    }
}
