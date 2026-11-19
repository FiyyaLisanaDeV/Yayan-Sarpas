package id.go.rsudkendari.sarpras.mobile.data.remote

import com.jakewharton.retrofit2.converter.kotlinx.serialization.asConverterFactory
import id.go.rsudkendari.sarpras.mobile.data.local.TokenStore
import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import retrofit2.Retrofit
import java.util.concurrent.TimeUnit

class ApiClient(
    private val tokenStore: TokenStore,
    val baseUrlHolder: BaseUrlHolder,
) {
    private val json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
        coerceInputValues = true
    }

    private val contentType = "application/json".toMediaType()

    private val refreshRetrofit: Retrofit by lazy {
        Retrofit.Builder()
            .baseUrl("http://placeholder.local/")
            .client(
                OkHttpClient.Builder()
                    .addInterceptor(BaseUrlInterceptor(baseUrlHolder))
                    .addInterceptor(HttpLoggingInterceptor().apply { level = HttpLoggingInterceptor.Level.BASIC })
                    .connectTimeout(20, TimeUnit.SECONDS)
                    .readTimeout(20, TimeUnit.SECONDS)
                    .build()
            )
            .addConverterFactory(json.asConverterFactory(contentType))
            .build()
    }

    private val refreshApi: SarprasApi by lazy { refreshRetrofit.create(SarprasApi::class.java) }

    private val okHttp: OkHttpClient by lazy {
        OkHttpClient.Builder()
            .addInterceptor(BaseUrlInterceptor(baseUrlHolder))
            .addInterceptor(AuthInterceptor(tokenStore))
            .authenticator(TokenAuthenticator(tokenStore) { refreshApi })
            .addInterceptor(HttpLoggingInterceptor().apply { level = HttpLoggingInterceptor.Level.BASIC })
            .connectTimeout(20, TimeUnit.SECONDS)
            .readTimeout(20, TimeUnit.SECONDS)
            .build()
    }

    val api: SarprasApi by lazy {
        Retrofit.Builder()
            .baseUrl("http://placeholder.local/")
            .client(okHttp)
            .addConverterFactory(json.asConverterFactory(contentType))
            .build()
            .create(SarprasApi::class.java)
    }
}
