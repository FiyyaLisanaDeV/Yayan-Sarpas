package id.go.rsudkendari.sarpras.mobile.data.remote

import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.Interceptor
import okhttp3.Response

class BaseUrlInterceptor(private val baseUrlHolder: BaseUrlHolder) : Interceptor {
    override fun intercept(chain: Interceptor.Chain): Response {
        val original = chain.request()
        val base = baseUrlHolder.get().toHttpUrlOrNull()
            ?: return chain.proceed(original)
        val newUrl = original.url.newBuilder()
            .scheme(base.scheme)
            .host(base.host)
            .port(base.port)
            .build()
        val newReq = original.newBuilder().url(newUrl).build()
        return chain.proceed(newReq)
    }
}
