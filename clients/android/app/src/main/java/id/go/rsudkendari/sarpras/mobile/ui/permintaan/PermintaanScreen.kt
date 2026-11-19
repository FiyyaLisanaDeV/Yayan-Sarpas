package id.go.rsudkendari.sarpras.mobile.ui.permintaan

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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import id.go.rsudkendari.sarpras.mobile.data.model.CreatePermintaanBody
import id.go.rsudkendari.sarpras.mobile.data.model.ItemInputBody
import id.go.rsudkendari.sarpras.mobile.data.model.ItemOpt
import id.go.rsudkendari.sarpras.mobile.data.model.PermintaanRow
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
fun PermintaanScreen(repo: MobileRepository, canCreate: Boolean, onBack: () -> Unit) {
    val scope = rememberCoroutineScope()
    val snack = remember { SnackbarHostState() }
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }
    var rows by remember { mutableStateOf<List<PermintaanRow>>(emptyList()) }
    var fromCache by remember { mutableStateOf(false) }
    var selected by remember { mutableStateOf<PermintaanRow?>(null) }
    var showCreate by remember { mutableStateOf(false) }

    fun load() {
        scope.launch {
            loading = true; error = null
            try {
                val res = repo.permintaanList()
                rows = res.data; fromCache = res.fromCache
            } catch (e: Exception) { error = errMsg(e) }
            finally { loading = false }
        }
    }
    LaunchedEffect(Unit) { load() }

    if (showCreate) {
        CreatePermintaanScreen(
            repo = repo,
            onBack = { showCreate = false },
            onDone = { msg -> showCreate = false; scope.launch { snack.showSnackbar(msg) }; load() },
        )
        return
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Permintaan Saya") },
                navigationIcon = { IconButton(onClick = onBack) { Icon(Icons.Default.ArrowBack, contentDescription = "Kembali") } },
                actions = { IconButton(onClick = { load() }) { Icon(Icons.Default.Refresh, contentDescription = "Muat ulang") } },
            )
        },
        floatingActionButton = {
            if (canCreate) FloatingActionButton(onClick = { showCreate = true }) {
                Icon(Icons.Default.Add, contentDescription = "Buat permintaan")
            }
        },
        snackbarHost = { SnackbarHost(snack) },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            if (fromCache) CacheBanner()
            when {
                loading -> LoadingState()
                error != null -> ErrorState(error!!, onRetry = { load() })
                rows.isEmpty() -> EmptyState("Belum ada permintaan.")
                else -> LazyColumn(Modifier.fillMaxSize().padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    val grouped = groupPermintaan(rows)
                    items(grouped, key = { it.key }) { g ->
                        Card(
                            modifier = Modifier.fillMaxWidth().clickable { selected = g.representative },
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            elevation = CardDefaults.cardElevation(1.dp),
                        ) {
                            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                    Text(g.ruangan, fontWeight = FontWeight.SemiBold)
                                    StatusBadge(g.status)
                                }
                                Text(g.ringkas)
                                if (g.items.size > 1) Text("${g.items.size} item dalam pengajuan ini", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                if (g.revisiKe > 1) Text("Revisi ${g.revisiKe}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                                Text(formatDate(g.tanggal), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }
                    }
                }
            }
        }
    }

    val sel = selected
    if (sel != null) {
        val batchId = sel.batchId
        val canAct = batchId != null && (sel.status == "DRAFT" || sel.status == "RETURNED" || sel.status == "REJECTED")
        AlertDialog(
            onDismissRequest = { selected = null },
            title = { Text("Detail Permintaan") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("${sel.ruangan.nama} · ${labelStatus(sel.status)}", fontWeight = FontWeight.SemiBold)
                    Text("${sel.item.nama} (${sel.item.kategori})")
                    Text("Jumlah: ${sel.jumlah} ${sel.satuan.kode}")
                    if (!sel.keterangan.isNullOrBlank()) Text("Keterangan: ${sel.keterangan}")
                    if (sel.revisiKe > 1) Text("Revisi ke-${sel.revisiKe}")
                    Text("Diperbarui: ${formatDate(sel.updatedAt)}", style = MaterialTheme.typography.bodySmall)
                }
            },
            confirmButton = {
                if (canAct) TextButton(onClick = {
                    scope.launch {
                        try {
                            repo.permintaanSubmit(batchId!!)
                            snack.showSnackbar(if (sel.status == "DRAFT") "Permintaan diajukan." else "Revisi diajukan ulang.")
                            selected = null; load()
                        } catch (e: Exception) { snack.showSnackbar(errMsg(e)) }
                    }
                }) { Text(if (sel.status == "DRAFT") "Ajukan" else "Ajukan Ulang") }
            },
            dismissButton = {
                Row {
                    if (canAct && sel.status == "DRAFT") TextButton(onClick = {
                        scope.launch {
                            try {
                                repo.permintaanDeleteBatch(batchId!!)
                                snack.showSnackbar("Draft dibatalkan.")
                                selected = null; load()
                            } catch (e: Exception) { snack.showSnackbar(errMsg(e)) }
                        }
                    }) { Text("Batalkan Draft", color = MaterialTheme.colorScheme.error) }
                    TextButton(onClick = { selected = null }) { Text("Tutup") }
                }
            },
        )
    }
}

private data class GroupedPermintaan(
    val key: String,
    val representative: PermintaanRow,
    val ruangan: String,
    val items: List<String>,
    val ringkas: String,
    val status: String,
    val revisiKe: Int,
    val tanggal: String?,
)

private fun groupPermintaan(rows: List<PermintaanRow>): List<GroupedPermintaan> {
    val map = linkedMapOf<String, MutableList<PermintaanRow>>()
    for (r in rows) {
        val k = r.batchId ?: "single-${r.id}"
        map.getOrPut(k) { mutableListOf() }.add(r)
    }
    return map.map { (k, list) ->
        val r = list.first()
        val items = list.map { "${it.item.nama.trim()} (${it.jumlah} ${it.satuan.kode})" }
        GroupedPermintaan(
            key = k, representative = r, ruangan = r.ruangan.nama, items = items,
            ringkas = if (items.size == 1) items[0] else "${items[0]} +${items.size - 1} item lain",
            status = r.status, revisiKe = r.revisiKe, tanggal = r.submittedAt ?: r.updatedAt,
        )
    }
}

private class BarisState(
    item: ItemOpt? = null,
    jumlah: String = "",
    keterangan: String = "",
) {
    var item by mutableStateOf(item)
    var jumlah by mutableStateOf(jumlah)
    var keterangan by mutableStateOf(keterangan)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CreatePermintaanScreen(
    repo: MobileRepository,
    onBack: () -> Unit,
    onDone: (String) -> Unit,
) {
    val scope = rememberCoroutineScope()
    var loading by remember { mutableStateOf(true) }
    var loadError by remember { mutableStateOf<String?>(null) }
    var submitError by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var itemsOpt by remember { mutableStateOf<List<ItemOpt>>(emptyList()) }
    val baris = remember { mutableStateListOf(BarisState()) }

    LaunchedEffect(Unit) {
        try { itemsOpt = repo.itemsOptions() } catch (e: Exception) { loadError = errMsg(e) } finally { loading = false }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Buat Permintaan") },
                navigationIcon = { IconButton(onClick = onBack) { Icon(Icons.Default.ArrowBack, contentDescription = "Kembali") } },
            )
        },
    ) { padding ->
        when {
            loading -> Column(Modifier.padding(padding)) { LoadingState() }
            loadError != null -> Column(Modifier.padding(padding)) { ErrorState(loadError!!) }
            else -> Column(
                Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("Tambah item kebutuhan ruangan Anda, lalu simpan draft atau ajukan.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                baris.forEachIndexed { idx, b ->
                    Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface), elevation = CardDefaults.cardElevation(1.dp)) {
                        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                Text("Item ${idx + 1}", fontWeight = FontWeight.SemiBold)
                                if (baris.size > 1) TextButton(onClick = { baris.removeAt(idx) }) { Text("Hapus") }
                            }
                            var expanded by remember { mutableStateOf(false) }
                            var query by remember(b.item) { mutableStateOf(b.item?.nama ?: "") }
                            ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
                                OutlinedTextField(
                                    value = query, onValueChange = { query = it; expanded = true },
                                    label = { Text("Cari & pilih item") }, modifier = Modifier.menuAnchor().fillMaxWidth(),
                                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) }, singleLine = true,
                                )
                                val filtered = if (query.isBlank()) itemsOpt.take(40) else itemsOpt.filter {
                                    it.nama.contains(query, true) || it.kategori.contains(query, true)
                                }.take(40)
                                ExposedDropdownMenu(expanded = expanded && filtered.isNotEmpty(), onDismissRequest = { expanded = false }) {
                                    filtered.forEach { opt ->
                                        DropdownMenuItem(
                                            text = { Text("${opt.nama} (${opt.kategori}, ${opt.satuanKode})") },
                                            onClick = { b.item = opt; query = opt.nama; expanded = false },
                                        )
                                    }
                                }
                            }
                            OutlinedTextField(
                                value = b.jumlah, onValueChange = { b.jumlah = it },
                                label = { Text("Jumlah${b.item?.let { " (${it.satuanKode})" } ?: ""}") },
                                modifier = Modifier.fillMaxWidth(), singleLine = true,
                            )
                            OutlinedTextField(
                                value = b.keterangan, onValueChange = { b.keterangan = it },
                                label = { Text("Keterangan (opsional)") }, modifier = Modifier.fillMaxWidth(),
                            )
                        }
                    }
                }
                OutlinedButton(onClick = { baris.add(BarisState()) }) { Text("+ Tambah item") }
                if (submitError != null) Text(submitError!!, color = MaterialTheme.colorScheme.error)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(
                        enabled = !busy,
                        onClick = {
                            scope.launch {
                                busy = true; submitError = null
                                try {
                                    val payload = buildPayload(baris) ?: run {
                                        submitError = "Isi minimal satu item dengan jumlah yang valid."; busy = false; return@launch
                                    }
                                    val res = repo.permintaanCreate(payload)
                                    onDone("Draft tersimpan (${res.jumlahBaris} item).")
                                } catch (e: Exception) { submitError = errMsg(e) } finally { busy = false }
                            }
                        },
                    ) { Text(if (busy) "Menyimpan…" else "Simpan Draft") }
                    OutlinedButton(
                        enabled = !busy,
                        onClick = {
                            scope.launch {
                                busy = true; submitError = null
                                try {
                                    val payload = buildPayload(baris) ?: run {
                                        submitError = "Isi minimal satu item dengan jumlah yang valid."; busy = false; return@launch
                                    }
                                    val res = repo.permintaanCreate(payload)
                                    repo.permintaanSubmit(res.batchId)
                                    onDone("Permintaan diajukan.")
                                } catch (e: Exception) { submitError = errMsg(e) } finally { busy = false }
                            }
                        },
                    ) { Text("Simpan & Ajukan") }
                }
                Spacer(Modifier.height(24.dp))
            }
        }
    }
}

private fun buildPayload(baris: List<BarisState>): CreatePermintaanBody? {
    val items = baris.mapNotNull { b ->
        val it = b.item ?: return@mapNotNull null
        val qty = b.jumlah.trim().replace(',', '.')
        if (qty.isBlank() || qty.toDoubleOrNull() == null || qty.toDouble() <= 0.0) return@mapNotNull null
        ItemInputBody(itemId = it.id, jumlah = qty, satuanId = it.satuanId, keterangan = b.keterangan.trim().ifBlank { null })
    }
    if (items.isEmpty()) return null
    return CreatePermintaanBody(ruanganId = null, items = items)
}
