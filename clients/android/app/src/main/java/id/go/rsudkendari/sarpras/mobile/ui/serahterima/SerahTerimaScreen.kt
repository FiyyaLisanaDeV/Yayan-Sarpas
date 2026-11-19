package id.go.rsudkendari.sarpras.mobile.ui.serahterima

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
import androidx.compose.material3.AlertDialog
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
import id.go.rsudkendari.sarpras.mobile.data.model.BuatSerahBody
import id.go.rsudkendari.sarpras.mobile.data.model.Role
import id.go.rsudkendari.sarpras.mobile.data.model.RuanganOpt
import id.go.rsudkendari.sarpras.mobile.data.model.SerahRow
import id.go.rsudkendari.sarpras.mobile.data.model.SiapSerahRow
import id.go.rsudkendari.sarpras.mobile.data.model.User
import id.go.rsudkendari.sarpras.mobile.data.model.UserOpt
import id.go.rsudkendari.sarpras.mobile.data.repo.MobileRepository
import id.go.rsudkendari.sarpras.mobile.ui.common.CacheBanner
import id.go.rsudkendari.sarpras.mobile.ui.common.EmptyState
import id.go.rsudkendari.sarpras.mobile.ui.common.ErrorState
import id.go.rsudkendari.sarpras.mobile.ui.common.LoadingState
import id.go.rsudkendari.sarpras.mobile.ui.common.StatusBadge
import id.go.rsudkendari.sarpras.mobile.ui.common.errMsg
import id.go.rsudkendari.sarpras.mobile.ui.common.formatDate
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SerahTerimaScreen(repo: MobileRepository, user: User, onBack: () -> Unit) {
    val scope = rememberCoroutineScope()
    val snack = remember { SnackbarHostState() }
    val bolehBuat = user.role in listOf(Role.ADMIN, Role.PETUGAS_GUDANG, Role.KASI_SARPRAS)

    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }
    var daftar by remember { mutableStateOf<List<SerahRow>>(emptyList()) }
    var siap by remember { mutableStateOf<List<SiapSerahRow>>(emptyList()) }
    var fromCache by remember { mutableStateOf(false) }
    var showCreate by remember { mutableStateOf(false) }
    var confirmTarget by remember { mutableStateOf<SerahRow?>(null) }
    var disputeTarget by remember { mutableStateOf<SerahRow?>(null) }
    var disputeNote by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }

    fun load() {
        scope.launch {
            loading = true; error = null
            try {
                val res = repo.serahTerimaList()
                daftar = res.data; fromCache = res.fromCache
                siap = if (bolehBuat) runCatching { repo.serahTerimaSiap() }.getOrDefault(emptyList()) else emptyList()
            } catch (e: Exception) { error = errMsg(e) }
            finally { loading = false }
        }
    }
    LaunchedEffect(Unit) { load() }

    if (showCreate) {
        BuatBastScreen(
            repo = repo,
            onBack = { showCreate = false },
            onDone = { msg -> showCreate = false; scope.launch { snack.showSnackbar(msg) }; load() },
        )
        return
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Serah Terima (BAST)") },
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
                if (fromCache) item { CacheBanner() }

                if (bolehBuat) {
                    item { Text("Barang siap diserahkan", fontWeight = FontWeight.SemiBold) }
                    if (siap.isEmpty()) item { Text("Tidak ada barang menunggu diserahkan.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
                    items(siap, key = { it.penerimaanId }) { s ->
                        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface), elevation = CardDefaults.cardElevation(1.dp)) {
                            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                Text("${s.item} — ${s.qty} unit", fontWeight = FontWeight.SemiBold)
                                Text("${s.noPo} · ${s.noTerima}", style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                    item {
                        Button(onClick = { showCreate = true }, modifier = Modifier.fillMaxWidth(), enabled = siap.isNotEmpty()) {
                            Text("Pilih & buat BAST dari barang siap")
                        }
                    }
                    item { Spacer(Modifier.height(4.dp)) }
                }

                item { Text("Berita Acara Serah Terima", fontWeight = FontWeight.SemiBold) }
                if (daftar.isEmpty()) item { EmptyState("Belum ada BAST.") }
                items(daftar, key = { it.id }) { s ->
                    val bolehKonfirmasi = s.statusKonfirmasi == "PENDING" && (user.id == s.penerima.id || user.role == Role.ADMIN)
                    Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface), elevation = CardDefaults.cardElevation(1.dp)) {
                        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                Text(s.noBa ?: "BAST #${s.id}", fontWeight = FontWeight.SemiBold)
                                StatusBadge(s.statusKonfirmasi)
                            }
                            Text("${s.item} — ${s.qtySerah} unit")
                            Text("Ruangan: ${s.ruangan.nama} · Penerima: ${s.penerima.nama}", style = MaterialTheme.typography.bodySmall)
                            Text(formatDate(s.tanggalSerah), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            if (bolehKonfirmasi) {
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    Button(onClick = { confirmTarget = s }) { Text("Konfirmasi") }
                                    OutlinedButton(onClick = { disputeTarget = s; disputeNote = "" }) { Text("Laporkan masalah") }
                                }
                            }
                        }
                    }
                }
                item { Spacer(Modifier.height(24.dp)) }
            }
        }
    }

    val ct = confirmTarget
    if (ct != null) {
        AlertDialog(
            onDismissRequest = { if (!busy) confirmTarget = null },
            title = { Text("Konfirmasi penerimaan") },
            text = { Text("Konfirmasi BAST ${ct.noBa ?: ""} untuk ${ct.item}? Setelah dikonfirmasi, aset didaftarkan otomatis bila memenuhi ambang nilai.") },
            confirmButton = {
                TextButton(enabled = !busy, onClick = {
                    scope.launch {
                        busy = true
                        try {
                            repo.serahTerimaKonfirmasi(ct.id)
                            snack.showSnackbar("BAST dikonfirmasi.")
                            confirmTarget = null; load()
                        } catch (e: Exception) { snack.showSnackbar(errMsg(e)) } finally { busy = false }
                    }
                }) { Text(if (busy) "Memproses…" else "Konfirmasi") }
            },
            dismissButton = { TextButton(onClick = { if (!busy) confirmTarget = null }, enabled = !busy) { Text("Batal") } },
        )
    }

    val dt = disputeTarget
    if (dt != null) {
        AlertDialog(
            onDismissRequest = { if (!busy) disputeTarget = null },
            title = { Text("Laporkan masalah") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Jelaskan masalah pada barang/BAST ini (akan diteruskan sebagai sengketa).")
                    OutlinedTextField(value = disputeNote, onValueChange = { disputeNote = it }, label = { Text("Catatan masalah") }, modifier = Modifier.fillMaxWidth(), minLines = 2)
                }
            },
            confirmButton = {
                TextButton(enabled = !busy && disputeNote.isNotBlank(), onClick = {
                    scope.launch {
                        busy = true
                        try {
                            repo.serahTerimaDispute(dt.id, disputeNote.trim())
                            snack.showSnackbar("Masalah dilaporkan.")
                            disputeTarget = null; load()
                        } catch (e: Exception) { snack.showSnackbar(errMsg(e)) } finally { busy = false }
                    }
                }) { Text(if (busy) "Mengirim…" else "Kirim") }
            },
            dismissButton = { TextButton(onClick = { if (!busy) disputeTarget = null }, enabled = !busy) { Text("Batal") } },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun BuatBastScreen(repo: MobileRepository, onBack: () -> Unit, onDone: (String) -> Unit) {
    val scope = rememberCoroutineScope()
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }
    var submitError by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var siap by remember { mutableStateOf<List<SiapSerahRow>>(emptyList()) }
    var ruangan by remember { mutableStateOf<List<RuanganOpt>>(emptyList()) }
    var users by remember { mutableStateOf<List<UserOpt>>(emptyList()) }
    var selectedSiap by remember { mutableStateOf<SiapSerahRow?>(null) }
    var selectedRuangan by remember { mutableStateOf<RuanganOpt?>(null) }
    var selectedPenerima by remember { mutableStateOf<UserOpt?>(null) }
    var qty by remember { mutableStateOf("") }
    var catatan by remember { mutableStateOf("") }

    LaunchedEffect(Unit) {
        try {
            siap = repo.serahTerimaSiap()
            ruangan = repo.ruangan()
            users = repo.usersOptions()
            selectedSiap = siap.firstOrNull()
            qty = selectedSiap?.qty ?: ""
        } catch (e: Exception) { error = errMsg(e) } finally { loading = false }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Buat BAST") },
                navigationIcon = { IconButton(onClick = onBack) { Icon(Icons.Default.ArrowBack, contentDescription = "Kembali") } },
            )
        },
    ) { padding ->
        when {
            loading -> Column(Modifier.padding(padding)) { LoadingState() }
            error != null -> Column(Modifier.padding(padding)) { ErrorState(error!!) }
            siap.isEmpty() -> Column(Modifier.padding(padding).padding(16.dp)) { Text("Tidak ada barang yang siap diserahkan.") }
            else -> Column(
                Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                var sExp by remember { mutableStateOf(false) }
                ExposedDropdownMenuBox(expanded = sExp, onExpandedChange = { sExp = it }) {
                    OutlinedTextField(
                        value = selectedSiap?.let { "${it.item} — ${it.qty} (${it.noTerima})" } ?: "",
                        onValueChange = {}, readOnly = true, label = { Text("Barang (penerimaan)") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(sExp) },
                        modifier = Modifier.menuAnchor().fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = sExp, onDismissRequest = { sExp = false }) {
                        siap.forEach { s ->
                            DropdownMenuItem(text = { Text("${s.item} — ${s.qty} (${s.noPo}/${s.noTerima})") }, onClick = { selectedSiap = s; qty = s.qty; sExp = false })
                        }
                    }
                }

                var rExp by remember { mutableStateOf(false) }
                ExposedDropdownMenuBox(expanded = rExp, onExpandedChange = { rExp = it }) {
                    OutlinedTextField(
                        value = selectedRuangan?.nama ?: "", onValueChange = {}, readOnly = true, label = { Text("Ruangan penerima") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(rExp) },
                        modifier = Modifier.menuAnchor().fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = rExp, onDismissRequest = { rExp = false }) {
                        ruangan.forEach { r ->
                            DropdownMenuItem(text = { Text(r.nama) }, onClick = { selectedRuangan = r; rExp = false })
                        }
                    }
                }

                var uExp by remember { mutableStateOf(false) }
                ExposedDropdownMenuBox(expanded = uExp, onExpandedChange = { uExp = it }) {
                    OutlinedTextField(
                        value = selectedPenerima?.let { it.nama + (it.ruangan?.let { rr -> " — ${rr.nama}" } ?: "") } ?: "",
                        onValueChange = {}, readOnly = true, label = { Text("Petugas ruangan yang menerima") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(uExp) },
                        modifier = Modifier.menuAnchor().fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = uExp, onDismissRequest = { uExp = false }) {
                        users.forEach { u ->
                            DropdownMenuItem(text = { Text(u.nama + (u.ruangan?.let { " — ${it.nama}" } ?: "")) }, onClick = { selectedPenerima = u; uExp = false })
                        }
                    }
                }

                OutlinedTextField(value = qty, onValueChange = { qty = it }, label = { Text("Jumlah diserahkan") }, modifier = Modifier.fillMaxWidth(), singleLine = true)
                OutlinedTextField(value = catatan, onValueChange = { catatan = it }, label = { Text("Catatan (opsional)") }, modifier = Modifier.fillMaxWidth(), minLines = 2)

                if (submitError != null) Text(submitError!!, color = MaterialTheme.colorScheme.error)
                Button(
                    enabled = !busy,
                    onClick = {
                        val s = selectedSiap; val r = selectedRuangan; val p = selectedPenerima
                        if (s == null || r == null || p == null) { submitError = "Pilih barang, ruangan, dan penerima."; return@Button }
                        scope.launch {
                            busy = true; submitError = null
                            try {
                                val res = repo.serahTerimaBuat(
                                    BuatSerahBody(
                                        penerimaanId = s.penerimaanId,
                                        ruanganId = r.id,
                                        penerimaRuanganId = p.id,
                                        qty = qty.trim().ifBlank { null },
                                        catatan = catatan.trim().ifBlank { null },
                                    )
                                )
                                onDone("BAST ${res.noBa ?: ""} dibuat. Menunggu konfirmasi ruangan.")
                            } catch (e: Exception) { submitError = errMsg(e) } finally { busy = false }
                        }
                    },
                    modifier = Modifier.fillMaxWidth(),
                ) { Text(if (busy) "Menyimpan…" else "Buat BAST") }
                Spacer(Modifier.height(24.dp))
            }
        }
    }
}
