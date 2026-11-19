package id.go.rsudkendari.sarpras.mobile.ui.login

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import id.go.rsudkendari.sarpras.mobile.data.auth.AuthRepository
import id.go.rsudkendari.sarpras.mobile.data.local.ServerPrefs
import id.go.rsudkendari.sarpras.mobile.data.model.ApiException
import id.go.rsudkendari.sarpras.mobile.data.model.User
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class LoginUiState(
    val serverUrl: String = ServerPrefs.DEFAULT_BASE,
    val username: String = "",
    val password: String = "",
    val loading: Boolean = false,
    val error: String? = null,
)

class LoginViewModel(private val authRepository: AuthRepository) : ViewModel() {
    private val _state = MutableStateFlow(LoginUiState())
    val state: StateFlow<LoginUiState> = _state.asStateFlow()

    init {
        viewModelScope.launch {
            authRepository.apiBaseFlow.collect { base ->
                _state.update { if (it.serverUrl == ServerPrefs.DEFAULT_BASE) it.copy(serverUrl = base) else it }
            }
        }
    }

    fun onServerUrlChange(v: String) = _state.update { it.copy(serverUrl = v, error = null) }
    fun onUsernameChange(v: String) = _state.update { it.copy(username = v, error = null) }
    fun onPasswordChange(v: String) = _state.update { it.copy(password = v, error = null) }

    fun login(onSuccess: (User) -> Unit) {
        val s = _state.value
        if (s.username.isBlank() || s.password.isBlank()) {
            _state.update { it.copy(error = "Nama pengguna dan kata sandi wajib diisi.") }
            return
        }
        viewModelScope.launch {
            _state.update { it.copy(loading = true, error = null) }
            val res = authRepository.login(s.serverUrl, s.username, s.password)
            _state.update { it.copy(loading = false) }
            res.onSuccess { onSuccess(it) }
                .onFailure { e ->
                    val msg = when (e) {
                        is ApiException -> e.message
                        else -> if ((e.message ?: "").contains("Unable to resolve host") ||
                            (e.message ?: "").contains("Failed to connect")
                        ) "Tidak dapat terhubung ke server. Periksa alamat server dan jaringan."
                        else e.message ?: "Gagal masuk, coba lagi."
                    }
                    _state.update { it.copy(error = msg) }
                }
        }
    }

    companion object {
        fun factory(authRepository: AuthRepository): ViewModelProvider.Factory =
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <T : ViewModel> create(modelClass: Class<T>): T =
                    LoginViewModel(authRepository) as T
            }
    }
}
