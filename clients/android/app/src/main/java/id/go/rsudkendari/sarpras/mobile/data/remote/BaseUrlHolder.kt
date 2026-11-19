package id.go.rsudkendari.sarpras.mobile.data.remote

import java.util.concurrent.atomic.AtomicReference

class BaseUrlHolder(initial: String) {
    private val ref = AtomicReference(normalize(initial))

    fun get(): String = ref.get()

    fun set(url: String) {
        ref.set(normalize(url))
    }

    companion object {
        fun normalize(url: String): String = url.trim().removeSuffix("/").ifBlank { DEFAULT }
        const val DEFAULT = "http://10.0.2.2:8080"
    }
}
