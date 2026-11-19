package id.go.rsudkendari.sarpras.mobile.ui.approval

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import id.go.rsudkendari.sarpras.mobile.data.model.InboxItem
import id.go.rsudkendari.sarpras.mobile.data.repo.MobileRepository
import id.go.rsudkendari.sarpras.mobile.ui.common.CacheBanner
import id.go.rsudkendari.sarpras.mobile.ui.common.EmptyState
import id.go.rsudkendari.sarpras.mobile.ui.common.ErrorState
import id.go.rsudkendari.sarpras.mobile.ui.common.LoadingState
import id.go.rsudkendari.sarpras.mobile.ui.common.StatusBadge
import id.go.rsudkendari.sarpras.mobile.ui.common.errMsg
import id.go.rsudkendari.sarpras.mobile.ui.common.formatDate
import id.go.rsudkendari.sarpras.mobile.ui.common.labelStatus
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ApprovalScreen(repo: MobileRepository, onBack: () -> Unit) {
    val scope = rememberCoroutineScope()
    val snack = remember { SnackbarHostState() }
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }
    var items by remember { mutableStateOf<List<InboxItem>>(emptyList()) }
    var fromCache by remember { mutableStateOf(false) }
    var selected by remember { mutableStateOf<InboxItem?>(null) }
    var action by remember { mutableStateOf<String?>(null) } // setujui / tolak
    var catatan by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }

    fun load() {
        scope.launch {
            loading = true; error = null
            try {
                val res = repo.approvalInbox()
                items = res.data; fromCache = res.fromCache
            } catch (e: Exception) { error = errMsg(e) }
            finally { loading = false }
        }
    }

    LaunchedEffect(Unit) { load() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Persetujuan Permintaan") },
                navigationIcon = { IconButton(onClick = onBack) { Icon(Icons.Default.ArrowBack, contentDescription = "Kembali") } },
                actions = { IconButton(onClick = { load() }) { Icon(Icons.Default.Refresh, contentDescription = "Muat ulang") } },
            )
        },
        snackbarHost = { SnackbarHost(snack) },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            if (fromCache) CacheBanner()
            when {
                loading -> LoadingState()
                error != null -> ErrorState(error!!, onRetry = { load() })
                items.isEmpty() -> EmptyState("Tidak ada permintaan menunggu keputusan Anda.")
                else -> LazyColumn(Modifier.fillMaxSize().padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    items(items, key = { it.id }) { item ->
                        Card(
                            modifier = Modifier.fillMaxWidth().clickable { selected = item; action = null; catatan = "" },
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            elevation = CardDefaults.cardElevation(1.dp),
                        ) {
                            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                    Text(item.item, fontWeight = FontWeight.SemiBold, modifier = Modifier.weight(1f))
                                    StatusBadge(item.status)
                                }
                                Text("${item.ruangan} · ${item.jumlah} ${item.satuan} (${item.kategori})")
                                Text("Pemohon: ${item.pemohon}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("Diajukan: ${formatDate(item.submittedAt)} · ${labelStatus(item.status)}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }
                    }
                }
            }
        }
    }

    val sel = selected
    if (sel != null && action == null) {
        AlertDialog(
            onDismissRequest = { selected = null },
            title = { Text("Detail Permintaan") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(sel.item, fontWeight = FontWeight.SemiBold)
                    Text("Ruangan: ${sel.ruangan}")
                    Text("Jumlah: ${sel.jumlah} ${sel.satuan} (${sel.kategori})")
                    Text("Pemohon: ${sel.pemohon}")
                    Text("Status: ${labelStatus(sel.status)}")
                    if (!sel.keterangan.isNullOrBlank()) Text("Keterangan: ${sel.keterangan}")
                    Text("Diajukan: ${formatDate(sel.submittedAt)}", style = MaterialTheme.typography.bodySmall)
                    Spacer(Modifier.height(4.dp))
                    Text("Pilih tindakan (berlaku untuk pengajuan ini):", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            },
            confirmButton = { TextButton(onClick = { action = "setujui" }) { Text("Setujui") } },
            dismissButton = {
                Row {
                    TextButton(onClick = { action = "tolak" }) { Text("Tolak", color = MaterialTheme.colorScheme.error) }
                    TextButton(onClick = { selected = null }) { Text("Tutup") }
                }
            },
        )
    }

    if (sel != null && action != null) {
        val isTolak = action == "tolak"
        AlertDialog(
            onDismissRequest = { if (!busy) action = null },
            title = { Text(if (isTolak) "Tolak Permintaan" else "Setujui Permintaan") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(if (isTolak) "Alasan penolakan wajib diisi agar pemohon tahu yang harus diperbaiki." else "Catatan opsional.")
                    OutlinedTextField(
                        value = catatan,
                        onValueChange = { catatan = it },
                        label = { Text(if (isTolak) "Catatan penolakan (wajib)" else "Catatan (opsional)") },
                        modifier = Modifier.fillMaxWidth(),
                        minLines = 2,
                    )
                }
            },
            confirmButton = {
                TextButton(
                    enabled = !busy && (!isTolak || catatan.isNotBlank()) && sel.batchId != null,
                    onClick = {
                        scope.launch {
                            busy = true
                            try {
                                if (isTolak) repo.tolak(sel.batchId!!, catatan.trim())
                                else repo.setujui(sel.batchId!!, catatan.trim().ifBlank { null })
                                snack.showSnackbar(if (isTolak) "Permintaan ditolak." else "Permintaan disetujui.")
                                selected = null; action = null; catatan = ""
                                load()
                            } catch (e: Exception) {
                                snack.showSnackbar(errMsg(e))
                            } finally { busy = false }
                        }
                    },
                ) { Text(if (busy) "Memproses…" else if (isTolak) "Tolak" else "Setujui") }
            },
            dismissButton = { TextButton(onClick = { if (!busy) action = null }, enabled = !busy) { Text("Kembali") } },
        )
    }
}
