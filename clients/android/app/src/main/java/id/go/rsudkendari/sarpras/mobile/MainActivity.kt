package id.go.rsudkendari.sarpras.mobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import id.go.rsudkendari.sarpras.mobile.ui.navigation.SarprasNav
import id.go.rsudkendari.sarpras.mobile.ui.theme.SarprasTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            SarprasTheme {
                SarprasNav()
            }
        }
    }
}
