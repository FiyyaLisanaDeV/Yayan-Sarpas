package id.go.rsudkendari.sarpras.mobile.ui.notifications

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.DoneAll
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import id.go.rsudkendari.sarpras.mobile.data.model.Notifikasi
import id.go.rsudkendari.sarpras.mobile.data.repo.MobileRepository
import id.go.rsudkendari.sarpras.mobile.ui.common.CacheBanner
import id.go.rsudkendari.sarpras.mobile.ui.common.EmptyState
import id.go.rsudkendari.sarpras.mobile.ui.common.ErrorState
import id.go.rsudkendari.sarpras.mobile.ui.common.LoadingState
import id.go.rsudkendari.sarpras.mobile.ui.common.errMsg
import id.go.rsudkendari.sarpras.mobile.ui.common.formatDate
import kotlinx.coroutines.launch
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NotificationsScreen(
    repo: MobileRepository,
    onBack: (() -> Unit)? = null,
    onUnreadChanged: () -> Unit = {},
    embedded: Boolean = false,
) {
    val scope = rememberCoroutineScope()
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }
    var items by remember { mutableStateOf<List<Notifikasi>>(emptyList()) }
    var fromCache by remember { mutableStateOf(false) }

    fun load() {
        scope.launch {
            loading = true; error = null
            try {
                val res = repo.notifikasi()
                items = res.data; fromCache = res.fromCache
            } catch (e: Exception) {
                error = errMsg(e)
            } finally { loading = false }
        }
    }

    LaunchedEffect(Unit) { load() }

    val body: @Composable (Modifier) -> Unit = { mod ->
        Column(mod.fillMaxSize()) {
            if (embedded) {
                Row(
                    Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp),
                    horizontalArrangement = Arrangement.End,
                ) {
                    IconButton(onClick = {
                        scope.launch { runCatching { repo.markAllRead() }; onUnreadChanged(); load() }
                    }) { Icon(Icons.Default.DoneAll, contentDescription = "Tandai semua dibaca") }
                    IconButton(onClick = { load() }) { Icon(Icons.Default.Refresh, contentDescription = "Muat ulang") }
                }
            }
            if (fromCache) CacheBanner()
            when {
                loading -> LoadingState()
                error != null -> ErrorState(error!!, onRetry = { load() })
                items.isEmpty() -> EmptyState("Belum ada notifikasi.")
                else -> LazyColumn(
                    Modifier.fillMaxSize().padding(12.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    items(items, key = { it.id }) { n ->
                        Card(
                            modifier = Modifier.fillMaxWidth().clickable {
                                if (!n.isRead) scope.launch {
                                    runCatching { repo.markRead(n.id) }
                                    onUnreadChanged(); load()
                                }
                            },
                            colors = CardDefaults.cardColors(
                                containerColor = if (n.isRead) MaterialTheme.colorScheme.surface
                                else MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.35f),
                            ),
                        ) {
                            Row(Modifier.padding(12.dp), verticalAlignment = Alignment.Top) {
                                if (!n.isRead) {
                                    Box(
                                        Modifier.padding(top = 6.dp, end = 8.dp).size(8.dp)
                                            .clip(CircleShape).background(MaterialTheme.colorScheme.primary),
                                    )
                                }
                                Column(Modifier.weight(1f)) {
                                    Text(n.pesan, fontWeight = if (n.isRead) FontWeight.Normal else FontWeight.SemiBold)
                                    Spacer(Modifier.height(2.dp))
                                    Text(
                                        "${tipeLabel(n.tipe)} · ${formatDate(n.createdAt)}",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (embedded) {
        body(Modifier)
    } else {
        Scaffold(
            topBar = {
                TopAppBar(
                    title = { Text("Notifikasi") },
                    navigationIcon = {
                        if (onBack != null) {
                            IconButton(onClick = onBack) { Icon(Icons.Default.ArrowBack, contentDescription = "Kembali") }
                        }
                    },
                    actions = {
                        IconButton(onClick = {
                            scope.launch { runCatching { repo.markAllRead() }; onUnreadChanged(); load() }
                        }) { Icon(Icons.Default.DoneAll, contentDescription = "Tandai semua dibaca") }
                        IconButton(onClick = { load() }) { Icon(Icons.Default.Refresh, contentDescription = "Muat ulang") }
                    },
                )
            },
        ) { padding -> body(Modifier.padding(padding)) }
    }
}

private fun tipeLabel(t: String) = when (t.uppercase()) {
    "SUBMIT", "PERMINTAAN_SUBMITTED" -> "Permintaan diajukan"
    "APPROVE", "PERMINTAAN_DISETUJUI" -> "Disetujui"
    "REJECT", "PERMINTAAN_DITOLAK" -> "Ditolak"
    "SLA_WARNING" -> "Peringatan SLA"
    "BARANG_TIBA" -> "Barang tiba"
    "SERAH_TERIMA" -> "Serah terima"
    "PO_APPROVED" -> "PO disetujui"
    else -> t.replace('_', ' ').lowercase().replaceFirstChar { it.titlecase(Locale("id", "ID")) }
}
