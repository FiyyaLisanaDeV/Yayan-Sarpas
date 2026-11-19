package id.go.rsudkendari.sarpras.mobile.ui.penerimaan

import android.content.Context
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import id.go.rsudkendari.sarpras.mobile.data.model.PenerimaanRow
import id.go.rsudkendari.sarpras.mobile.data.model.PoHeader
import id.go.rsudkendari.sarpras.mobile.data.model.SiapTerimaRow
import id.go.rsudkendari.sarpras.mobile.data.model.TerimaBody
import id.go.rsudkendari.sarpras.mobile.data.repo.MobileRepository
import id.go.rsudkendari.sarpras.mobile.ui.common.ErrorState
import id.go.rsudkendari.sarpras.mobile.ui.common.LoadingState
import id.go.rsudkendari.sarpras.mobile.ui.common.StatusBadge
import id.go.rsudkendari.sarpras.mobile.ui.common.errMsg
import id.go.rsudkendari.sarpras.mobile.ui.common.formatDate
import id.go.rsudkendari.sarpras.mobile.ui.common.formatRupiah
import kotlinx.coroutines.launch
import java.io.File

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PenerimaanScreen(repo: MobileRepository, onBack: () -> Unit) {
    val scope = rememberCoroutineScope()
    val snack = remember { SnackbarHostState() }
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }
    var poSent by remember { mutableStateOf<List<PoHeader>>(emptyList()) }
    var riwayat by remember { mutableStateOf<List<PenerimaanRow>>(emptyList()) }
    var selectedPo by remember { mutableStateOf<PoHeader?>(null) }

    fun load() {
        scope.launch {
            loading = true; error = null
            try {
                poSent = repo.poSent()
                riwayat = runCatching { repo.penerimaanList() }.getOrDefault(emptyList())
            } catch (e: Exception) { error = errMsg(e) }
            finally { loading = false }
        }
    }
    LaunchedEffect(Unit) { load() }

    val po = selectedPo
    if (po != null) {
        TerimaFormScreen(
            repo = repo, po = po,
            onBack = { selectedPo = null },
            onDone = { msg -> selectedPo = null; scope.launch { snack.showSnackbar(msg) }; load() },
        )
        return
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Penerimaan Barang") },
                navigationIcon = { IconButton(onClick = onBack) { Icon(Icons.Default.ArrowBack, contentDescription = "Kembali") } },
                actions = { IconButton(onClick = { load() }) { Icon(Icons.Default.Refresh, contentDescription = "Muat ulang") } },
            )
        },
        snackbarHost = { SnackbarHost(snack) },
    ) { padding ->
        when {
            loading -> Column(Modifier.padding(padding)) { LoadingState() }
            error != null -> Column(Modifier.padding(padding)) { ErrorState(error!!, onRetry = { load() }) }
            else -> LazyColumn(Modifier.fillMaxSize().padding(padding).padding(12.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                item {
                    Text("Menunggu kedatangan barang (PO dikirim)", fontWeight = FontWeight.SemiBold)
                    Text("Pilih PO untuk mencatat penerimaan. Dokumen DO, Invoice, dan Faktur Pajak wajib diisi.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                if (poSent.isEmpty()) item { Text("Tidak ada PO dalam perjalanan.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
                items(poSent, key = { it.id }) { p ->
                    Card(
                        modifier = Modifier.fillMaxWidth().clickable { selectedPo = p },
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        elevation = CardDefaults.cardElevation(1.dp),
                    ) {
                        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                Text(p.noPo, fontWeight = FontWeight.SemiBold)
                                StatusBadge(p.status)
                            }
                            Text(p.supplier.nama)
                            Text("${formatRupiah(p.nilaiTotal)} · ${p.tanggalPo ?: ""}", style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
                item { Spacer(Modifier.height(8.dp)); Text("Riwayat penerimaan", fontWeight = FontWeight.SemiBold) }
                if (riwayat.isEmpty()) item { Text("Belum ada penerimaan tercatat.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
                items(riwayat, key = { it.id }) { r ->
                    Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
                        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text("${r.noTerima} · ${r.noPo}", fontWeight = FontWeight.SemiBold)
                            Text("${r.item} — diterima ${r.qtyTerima} / ${r.qtyPo} (tolak ${r.qtyReject})")
                            Text(formatDate(r.tanggalTerima), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                }
                item { Spacer(Modifier.height(24.dp)) }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TerimaFormScreen(repo: MobileRepository, po: PoHeader, onBack: () -> Unit, onDone: (String) -> Unit) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }
    var submitError by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var rows by remember { mutableStateOf<List<SiapTerimaRow>>(emptyList()) }
    var selected by remember { mutableStateOf<SiapTerimaRow?>(null) }
    var qtyTerima by remember { mutableStateOf("") }
    var qtyReject by remember { mutableStateOf("0") }
    var kondisi by remember { mutableStateOf("Baik") }
    var noDo by remember { mutableStateOf("") }
    var noInvoice by remember { mutableStateOf("") }
    var noFaktur by remember { mutableStateOf("") }
    var catatan by remember { mutableStateOf("") }
    var photoUri by remember { mutableStateOf<Uri?>(null) }

    val pickPhoto = rememberLauncherForActivityResult(ActivityResultContracts.GetContent()) { uri -> photoUri = uri }

    LaunchedEffect(po.id) {
        try {
            rows = repo.penerimaanSiap(po.id)
            selected = rows.firstOrNull()
            qtyTerima = selected?.sisa ?: ""
        } catch (e: Exception) { error = errMsg(e) } finally { loading = false }
    }

    fun pilih(r: SiapTerimaRow) {
        selected = r; qtyTerima = r.sisa; qtyReject = "0"
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Terima — ${po.noPo}") },
                navigationIcon = { IconButton(onClick = onBack) { Icon(Icons.Default.ArrowBack, contentDescription = "Kembali") } },
            )
        },
    ) { padding ->
        when {
            loading -> Column(Modifier.padding(padding)) { LoadingState() }
            error != null -> Column(Modifier.padding(padding)) { ErrorState(error!!) }
            rows.isEmpty() -> Column(Modifier.padding(padding).padding(16.dp)) { Text("Tidak ada sisa barang untuk diterima pada PO ini.") }
            else -> Column(
                Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Text(po.supplier.nama, fontWeight = FontWeight.SemiBold)

                var expanded by remember { mutableStateOf(false) }
                ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
                    OutlinedTextField(
                        value = selected?.let { "${it.item} (sisa ${it.sisa} ${it.satuan})" } ?: "",
                        onValueChange = {}, readOnly = true, label = { Text("Item yang datang") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) },
                        modifier = Modifier.menuAnchor().fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                        rows.forEach { r ->
                            DropdownMenuItem(text = { Text("${r.item} — sisa ${r.sisa} ${r.satuan}") }, onClick = { pilih(r); expanded = false })
                        }
                    }
                }

                OutlinedTextField(value = qtyTerima, onValueChange = { qtyTerima = it }, label = { Text("Jumlah diterima") }, modifier = Modifier.fillMaxWidth(), singleLine = true)
                OutlinedTextField(value = qtyReject, onValueChange = { qtyReject = it }, label = { Text("Jumlah ditolak/rusak") }, modifier = Modifier.fillMaxWidth(), singleLine = true)

                var kExp by remember { mutableStateOf(false) }
                ExposedDropdownMenuBox(expanded = kExp, onExpandedChange = { kExp = it }) {
                    OutlinedTextField(
                        value = kondisi, onValueChange = {}, readOnly = true, label = { Text("Kondisi barang") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(kExp) },
                        modifier = Modifier.menuAnchor().fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = kExp, onDismissRequest = { kExp = false }) {
                        listOf("Baik", "Rusak ringan", "Rusak berat").forEach { k ->
                            DropdownMenuItem(text = { Text(k) }, onClick = { kondisi = k; kExp = false })
                        }
                    }
                }

                Text("Dokumen wajib", fontWeight = FontWeight.SemiBold)
                OutlinedTextField(value = noDo, onValueChange = { noDo = it }, label = { Text("No. Delivery Order (DO)") }, modifier = Modifier.fillMaxWidth(), singleLine = true)
                OutlinedTextField(value = noInvoice, onValueChange = { noInvoice = it }, label = { Text("No. Invoice") }, modifier = Modifier.fillMaxWidth(), singleLine = true)
                OutlinedTextField(value = noFaktur, onValueChange = { noFaktur = it }, label = { Text("No. Faktur Pajak") }, modifier = Modifier.fillMaxWidth(), singleLine = true)
                OutlinedTextField(value = catatan, onValueChange = { catatan = it }, label = { Text("Catatan (opsional)") }, modifier = Modifier.fillMaxWidth(), minLines = 2)

                Text("Foto dokumen (opsional)", fontWeight = FontWeight.SemiBold)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedButton(onClick = { pickPhoto.launch("image/*") }) { Text(if (photoUri == null) "Pilih foto" else "Ganti foto") }
                    if (photoUri != null) Text("Foto dipilih", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
                }

                if (submitError != null) Text(submitError!!, color = MaterialTheme.colorScheme.error)
                Button(
                    enabled = !busy && selected != null,
                    onClick = {
                        val sel = selected ?: return@Button
                        if (noDo.isBlank() || noInvoice.isBlank() || noFaktur.isBlank()) {
                            submitError = "No. DO, Invoice, dan Faktur Pajak wajib diisi."; return@Button
                        }
                        val qt = qtyTerima.trim().replace(',', '.')
                        if (qt.toDoubleOrNull() == null || qt.toDouble() <= 0.0) {
                            submitError = "Jumlah diterima harus lebih dari 0."; return@Button
                        }
                        scope.launch {
                            busy = true; submitError = null
                            try {
                                val res = repo.terima(
                                    TerimaBody(
                                        poDetailId = sel.poDetailId,
                                        qtyTerima = qt,
                                        qtyReject = qtyReject.trim().ifBlank { "0" }.replace(',', '.'),
                                        kondisi = kondisi,
                                        noDo = noDo.trim(),
                                        noInvoice = noInvoice.trim(),
                                        noFakturPajak = noFaktur.trim(),
                                        catatan = catatan.trim().ifBlank { null },
                                    )
                                )
                                photoUri?.let { uri ->
                                    val file = uriToTempFile(context, uri)
                                    if (file != null) runCatching { repo.upload("penerimaan", res.id, file) }
                                    file?.delete()
                                }
                                onDone("Penerimaan ${res.noTerima} tersimpan.")
                            } catch (e: Exception) { submitError = errMsg(e) } finally { busy = false }
                        }
                    },
                    modifier = Modifier.fillMaxWidth(),
                ) { Text(if (busy) "Menyimpan…" else "Simpan penerimaan") }
                Spacer(Modifier.height(24.dp))
            }
        }
    }
}

private fun uriToTempFile(context: Context, uri: Uri): File? = try {
    val input = context.contentResolver.openInputStream(uri) ?: return null
    val file = File.createTempFile("upload_", ".jpg", context.cacheDir)
    file.outputStream().use { out -> input.use { it.copyTo(out) } }
    file
} catch (_: Exception) { null }
