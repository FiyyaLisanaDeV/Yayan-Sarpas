package id.go.rsudkendari.sarpras.mobile

import android.app.Application
import id.go.rsudkendari.sarpras.mobile.di.AppContainer

class SarprasApp : Application() {
    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        container = AppContainer(this)
    }
}
