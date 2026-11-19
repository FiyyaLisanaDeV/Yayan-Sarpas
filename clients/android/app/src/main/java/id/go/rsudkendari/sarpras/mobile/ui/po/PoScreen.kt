package id.go.rsudkendari.sarpras.mobile.ui.po

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
import id.go.rsudkendari.sarpras.mobile.data.model.PoHeader
import id.go.rsudkendari.sarpras.mobile.data.model.Role
import id.go.rsudkendari.sarpras.mobile.data.repo.MobileRepository
import id.go.rsudkendari.sarpras.mobile.ui.common.CacheBanner
import id.go.rsudkendari.sarpras.mobile.ui.common.EmptyState
import id.go.rsudkendari.sarpras.mobile.ui.common.ErrorState
import id.go.rsudkendari.sarpras.mobile.ui.common.LoadingState
import id.go.rsudkendari.sarpras.mobile.ui.common.StatusBadge
import id.go.rsudkendari.sarpras.mobile.ui.common.errMsg
import id.go.rsudkendari.sarpras.mobile.ui.common.formatRupiah
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PoScreen(repo: MobileRepository, role: String, onBack: () -> Unit) {
    val scope = rememberCoroutineScope()
    val snack = remember { SnackbarHostState() }
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }
    var items by remember { mutableStateOf<List<PoHeader>>(emptyList()) }
    var fromCache by remember { mutableStateOf(false) }
    var selected by remember { mutableStateOf<PoHeader?>(null) }
    var action by remember { mutableStateOf<String?>(null) } // approve/reject/kirim/detail
    var catatan by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var detailText by remember { mutableStateOf<String?>(null) }

    val bisaSetujui = role in listOf(Role.ADMIN, Role.KASI_SARPRAS, Role.KABAG_KEUANGAN, Role.WADIR_YANMED)
    val bisaKirim = role in listOf(Role.ADMIN, Role.KASI_SARPRAS)

    fun load() {
        scope.launch {
            loading = true; error = null
            try {
                val res = repo.poList()
                items = res.data; fromCache = res.fromCache
            } catch (e: Exception) { error = errMsg(e) }
            finally { loading = false }
        }
    }

    LaunchedEffect(Unit) { load() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Pesanan (PO)") },
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
                items.isEmpty() -> EmptyState("Belum ada PO.")
                else -> LazyColumn(Modifier.fillMaxSize().padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    items(items, key = { it.id }) { po ->
                        Card(
                            modifier = Modifier.fillMaxWidth().clickable { selected = po; action = null; detailText = null; catatan = "" },
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            elevation = CardDefaults.cardElevation(1.dp),
                        ) {
                            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                    Text(po.noPo, fontWeight = FontWeight.SemiBold)
                                    StatusBadge(po.status)
                                }
                                Text(po.supplier.nama)
                                Text("Nilai: ${formatRupiah(po.nilaiTotal)} · ${po.tanggalPo ?: "—"}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
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
            title = { Text(sel.noPo) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("Supplier: ${sel.supplier.nama}")
                    Text("Nilai: ${formatRupiah(sel.nilaiTotal)} (PPN ${formatRupiah(sel.ppn)})")
                    Text("Status: ${id.go.rsudkendari.sarpras.mobile.ui.common.labelStatus(sel.status)}")
                    Text("Tanggal: ${sel.tanggalPo ?: "—"}", style = MaterialTheme.typography.bodySmall)
                    if (!sel.catatan.isNullOrBlank()) Text("Catatan: ${sel.catatan}")
                    if (detailText != null) { Spacer(Modifier.height(4.dp)); Text(detailText!!, style = MaterialTheme.typography.bodySmall) }
                }
            },
            confirmButton = {
                Row {
                    if (sel.status == "DRAFT" && bisaSetujui) TextButton(onClick = { action = "approve" }) { Text("Setujui") }
                    if (sel.status == "APPROVED" && bisaKirim) TextButton(onClick = { action = "kirim" }) { Text("Kirim") }
                }
            },
            dismissButton = {
                Row {
                    if (sel.status == "DRAFT" && bisaSetujui) TextButton(onClick = { action = "reject" }) { Text("Tolak", color = MaterialTheme.colorScheme.error) }
                    TextButton(onClick = {
                        scope.launch {
                            try {
                                val d = repo.poDetail(sel.id)
                                detailText = d.details.joinToString("\n") { "• ${it.item.nama}: ${it.qty} ${it.satuan} @ ${formatRupiah(it.hargaSatuan)}" }
                            } catch (e: Exception) { snack.showSnackbar(errMsg(e)) }
                        }
                    }) { Text("Rincian") }
                    TextButton(onClick = { selected = null }) { Text("Tutup") }
                }
            },
        )
    }

    if (sel != null && (action == "approve" || action == "kirim" || action == "reject")) {
        val isReject = action == "reject"
        AlertDialog(
            onDismissRequest = { if (!busy) action = null },
            title = { Text(when (action) { "approve" -> "Setujui PO"; "kirim" -> "Kirim PO"; else -> "Tolak PO" }) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(when (action) {
                        "approve" -> "Setujui PO ${sel.noPo}?"
                        "kirim" -> "Tandai PO ${sel.noPo} sudah dikirim ke supplier?"
                        else -> "Berikan alasan penolakan (opsional)."
                    })
                    if (isReject) OutlinedTextField(value = catatan, onValueChange = { catatan = it }, label = { Text("Catatan") }, modifier = Modifier.fillMaxWidth())
                }
            },
            confirmButton = {
                TextButton(enabled = !busy, onClick = {
                    scope.launch {
                        busy = true
                        try {
                            when (action) {
                                "approve" -> repo.poApprove(sel.id)
                                "kirim" -> repo.poKirim(sel.id)
                                "reject" -> repo.poReject(sel.id, catatan.trim().ifBlank { null })
                            }
                            snack.showSnackbar("Berhasil diproses.")
                            selected = null; action = null; load()
                        } catch (e: Exception) { snack.showSnackbar(errMsg(e)) } finally { busy = false }
                    }
                }) { Text(if (busy) "Memproses…" else "Lanjutkan") }
            },
            dismissButton = { TextButton(onClick = { if (!busy) action = null }, enabled = !busy) { Text("Batal") } },
        )
    }
}
