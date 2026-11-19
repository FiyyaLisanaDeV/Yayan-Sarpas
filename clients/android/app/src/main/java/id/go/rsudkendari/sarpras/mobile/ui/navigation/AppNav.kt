package id.go.rsudkendari.sarpras.mobile.ui.navigation

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.viewmodel.compose.viewModel
import id.go.rsudkendari.sarpras.mobile.SarprasApp
import id.go.rsudkendari.sarpras.mobile.data.model.User
import id.go.rsudkendari.sarpras.mobile.ui.home.HomeScreen
import id.go.rsudkendari.sarpras.mobile.ui.login.LoginScreen
import id.go.rsudkendari.sarpras.mobile.ui.login.LoginViewModel
import kotlinx.coroutines.launch

@Composable
fun SarprasNav() {
    val app = LocalContext.current.applicationContext as SarprasApp
    val authRepo = app.container.authRepository
    val mobileRepo = app.container.mobileRepository
    val scope = rememberCoroutineScope()

    var loading by remember { mutableStateOf(true) }
    var user by remember { mutableStateOf<User?>(null) }

    LaunchedEffect(Unit) {
        val restored = authRepo.restoreSession().getOrNull()
        user = restored ?: authRepo.currentUser()
        loading = false
    }

    when {
        loading -> {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                CircularProgressIndicator()
            }
        }

        user == null -> {
            val loginVm: LoginViewModel = viewModel(factory = LoginViewModel.factory(authRepo))
            LoginScreen(viewModel = loginVm, onLoggedIn = { user = it })
        }

        else -> {
            HomeScreen(user = user!!, repo = mobileRepo, onLogout = {
                scope.launch {
                    authRepo.logout()
                    user = null
                }
            })
        }
    }
}
