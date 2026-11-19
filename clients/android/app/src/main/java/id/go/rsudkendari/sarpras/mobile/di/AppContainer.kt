package id.go.rsudkendari.sarpras.mobile.di

import android.content.Context
import id.go.rsudkendari.sarpras.mobile.data.auth.AuthRepository
import id.go.rsudkendari.sarpras.mobile.data.local.ServerPrefs
import id.go.rsudkendari.sarpras.mobile.data.local.TokenStore
import id.go.rsudkendari.sarpras.mobile.data.remote.ApiClient
import id.go.rsudkendari.sarpras.mobile.data.remote.BaseUrlHolder
import id.go.rsudkendari.sarpras.mobile.data.repo.MobileRepository

class AppContainer(context: Context) {
    val tokenStore = TokenStore(context)
    val serverPrefs = ServerPrefs(context)
    val baseUrlHolder = BaseUrlHolder(ServerPrefs.DEFAULT_BASE)
    val apiClient = ApiClient(tokenStore, baseUrlHolder)
    val mobileRepository = MobileRepository(context, apiClient)
    val authRepository = AuthRepository(apiClient, tokenStore, serverPrefs, mobileRepository)
}
