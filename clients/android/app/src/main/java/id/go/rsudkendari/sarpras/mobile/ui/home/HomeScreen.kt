package id.go.rsudkendari.sarpras.mobile.ui.home

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
import androidx.compose.material.icons.filled.AccountCircle
import androidx.compose.material.icons.filled.Assignment
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Inventory
import androidx.compose.material.icons.filled.LocalShipping
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material.icons.filled.ShoppingCart
import androidx.compose.material.icons.filled.TaskAlt
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import id.go.rsudkendari.sarpras.mobile.data.model.Role
import id.go.rsudkendari.sarpras.mobile.data.model.User
import id.go.rsudkendari.sarpras.mobile.data.repo.MobileRepository
import id.go.rsudkendari.sarpras.mobile.ui.approval.ApprovalScreen
import id.go.rsudkendari.sarpras.mobile.ui.notifications.NotificationsScreen
import id.go.rsudkendari.sarpras.mobile.ui.penerimaan.PenerimaanScreen
import id.go.rsudkendari.sarpras.mobile.ui.permintaan.PermintaanScreen
import id.go.rsudkendari.sarpras.mobile.ui.po.PoScreen
import id.go.rsudkendari.sarpras.mobile.ui.serahterima.SerahTerimaScreen
import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import kotlinx.coroutines.supervisorScope

private enum class Dest { HOME, APPROVAL, PO, PERMINTAAN, PENERIMAAN, SERAH }

private data class TaskCardModel(
    val dest: Dest,
    val title: String,
    val desc: String,
    val icon: ImageVector,
    val badge: Int = 0,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    user: User,
    repo: MobileRepository,
    onLogout: () -> Unit,
) {
    var tab by rememberSaveable { mutableIntStateOf(0) }
    var sub by rememberSaveable { mutableStateOf(Dest.HOME) }
    var unread by remember { mutableIntStateOf(0) }
    var pendingApproval by remember { mutableIntStateOf(0) }
    var pendingPoDraft by remember { mutableIntStateOf(0) }
    var poSent by remember { mutableIntStateOf(0) }
    var serahPending by remember { mutableIntStateOf(0) }
    var serahSiap by remember { mutableIntStateOf(0) }
    var loadingCounts by remember { mutableStateOf(true) }
    val scope = rememberCoroutineScope()

    fun refreshCounts() {
        scope.launch {
            loadingCounts = true
            supervisorScope {
                val u = async { runCatching { repo.unreadCount() }.getOrDefault(0) }
                val a = async {
                    if (user.role in listOf(Role.KASI_SARPRAS, Role.KABID_YANMED, Role.WADIR_YANMED, Role.ADMIN))
                        runCatching { repo.approvalPendingCount() }.getOrDefault(0) else 0
                }
                val poDraft = async {
                    if (user.role in listOf(Role.KABAG_KEUANGAN, Role.WADIR_YANMED, Role.KASI_SARPRAS, Role.ADMIN))
                        runCatching { repo.poList(status = "DRAFT").data.size }.getOrDefault(0) else 0
                }
                val sent = async {
                    if (user.role in listOf(Role.PETUGAS_GUDANG, Role.KASI_SARPRAS, Role.ADMIN))
                        runCatching { repo.poSentCount() }.getOrDefault(0) else 0
                }
                val sp = async {
                    if (user.role == Role.USER_RUANGAN)
                        runCatching { repo.serahPendingCount() }.getOrDefault(0) else 0
                }
                val ss = async {
                    if (user.role in listOf(Role.PETUGAS_GUDANG, Role.KASI_SARPRAS, Role.ADMIN))
                        runCatching { repo.serahSiapCount() }.getOrDefault(0) else 0
                }
                unread = u.await(); pendingApproval = a.await(); pendingPoDraft = poDraft.await()
                poSent = sent.await(); serahPending = sp.await(); serahSiap = ss.await()
            }
            loadingCounts = false
        }
    }

    LaunchedEffect(user.id) { refreshCounts() }

    val tasks = remember(user.role, pendingApproval, pendingPoDraft, poSent, serahPending, serahSiap) {
        tasksFor(user.role, pendingApproval, pendingPoDraft, poSent, serahPending, serahSiap)
    }

    if (sub != Dest.HOME && tab == 0) {
        when (sub) {
            Dest.APPROVAL -> ApprovalScreen(repo, onBack = { sub = Dest.HOME; refreshCounts() })
            Dest.PO -> PoScreen(repo, role = user.role, onBack = { sub = Dest.HOME; refreshCounts() })
            Dest.PERMINTAAN -> PermintaanScreen(repo, canCreate = user.role in listOf(Role.USER_RUANGAN, Role.KASI_SARPRAS, Role.ADMIN), onBack = { sub = Dest.HOME; refreshCounts() })
            Dest.PENERIMAAN -> PenerimaanScreen(repo, onBack = { sub = Dest.HOME; refreshCounts() })
            Dest.SERAH -> SerahTerimaScreen(repo, user = user, onBack = { sub = Dest.HOME; refreshCounts() })
            else -> {}
        }
        return
    }

    Scaffold(
        topBar = { TopAppBar(title = { Text(if (tab == 0) "Tugas Anda" else if (tab == 1) "Notifikasi" else "Profil") }) },
        bottomBar = {
            NavigationBar {
                NavigationBarItem(
                    selected = tab == 0,
                    onClick = { tab = 0 },
                    icon = { Icon(Icons.Default.Home, contentDescription = null) },
                    label = { Text("Beranda") },
                )
                NavigationBarItem(
                    selected = tab == 1,
                    onClick = { tab = 1 },
                    icon = {
                        BadgedBox(badge = { if (unread > 0) Badge { Text(if (unread > 99) "99+" else unread.toString()) } }) {
                            Icon(Icons.Default.Notifications, contentDescription = null)
                        }
                    },
                    label = { Text("Notifikasi") },
                )
                NavigationBarItem(
                    selected = tab == 2,
                    onClick = { tab = 2 },
                    icon = { Icon(Icons.Default.AccountCircle, contentDescription = null) },
                    label = { Text("Profil") },
                )
            }
        },
    ) { padding ->
        when (tab) {
            0 -> BerandaTab(
                user = user, tasks = tasks, loadingCounts = loadingCounts,
                onOpen = { sub = it },
                modifier = Modifier.padding(padding),
            )
            1 -> Column(Modifier.padding(padding)) {
                NotificationsScreen(repo, onBack = null, onUnreadChanged = { refreshCounts() }, embedded = true)
            }
            2 -> ProfileTab(user, onLogout, Modifier.padding(padding))
        }
    }
}

@Composable
private fun BerandaTab(
    user: User,
    tasks: List<TaskCardModel>,
    loadingCounts: Boolean,
    onOpen: (Dest) -> Unit,
    modifier: Modifier = Modifier,
) {
    LazyColumn(
        modifier = modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            Text("Halo, ${user.nama}", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
            Text(Role.label(user.role), color = MaterialTheme.colorScheme.primary)
            Spacer(Modifier.height(4.dp))
            Text(
                "Tugas operasional hari ini (data dari server). Buka tiap layar untuk memuat ulang data terbaru.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        if (tasks.isEmpty()) {
            item {
                Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text("Tidak ada tugas operasional di Android", fontWeight = FontWeight.SemiBold)
                        Text(
                            "Peran Anda menggunakan aplikasi web/desktop untuk pengelolaan, laporan, master, dan administrasi. Anda tetap dapat menerima notifikasi.",
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        } else {
            items(tasks) { t ->
                Card(
                    modifier = Modifier.fillMaxWidth().clickable { onOpen(t.dest) },
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                ) {
                    Row(Modifier.padding(14.dp), verticalAlignment = Alignment.Top) {
                        Box(
                            Modifier.size(40.dp).clip(CircleShape).background(MaterialTheme.colorScheme.primaryContainer),
                            contentAlignment = Alignment.Center,
                        ) {
                            Icon(t.icon, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                        }
                        Column(Modifier.padding(start = 12.dp).weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Text(t.title, fontWeight = FontWeight.SemiBold)
                                if (t.badge > 0) {
                                    Box(
                                        Modifier.clip(CircleShape).background(MaterialTheme.colorScheme.error).padding(horizontal = 8.dp, vertical = 2.dp),
                                    ) { Text(t.badge.toString(), color = MaterialTheme.colorScheme.onError, style = MaterialTheme.typography.labelSmall) }
                                }
                            }
                            Text(t.desc, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                }
            }
        }
        item {
            Spacer(Modifier.height(4.dp))
            Text(
                if (loadingCounts) "Memperbarui ringkasan tugas…" else "Ketuk kartu untuk membuka tugas. Cache baca digunakan saat offline.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun ProfileTab(user: User, onLogout: () -> Unit, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
            Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(user.nama, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                Text("Peran: ${Role.label(user.role)}")
                Text("Nama pengguna/NIP: ${user.nip.ifBlank { "-" }}")
                Text("Email: ${user.email.ifBlank { "-" }}")
            }
        }
        Button(onClick = onLogout, modifier = Modifier.fillMaxWidth()) { Text("Keluar") }
        Text(
            "Keluar akan menghapus sesi dan cache baca perangkat ini. Semua mutasi divalidasi di server (RBAC).",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

private fun tasksFor(
    role: String,
    pendingApproval: Int,
    pendingPoDraft: Int,
    poSent: Int,
    serahPending: Int,
    serahSiap: Int,
): List<TaskCardModel> = when (role) {
    Role.USER_RUANGAN -> listOf(
        TaskCardModel(Dest.PERMINTAAN, "Permintaan Saya", "Buat, ajukan, dan pantau status permintaan ruangan Anda.", Icons.Default.Assignment),
        TaskCardModel(Dest.SERAH, "Konfirmasi Serah Terima (BAST)", "Konfirmasi barang yang diterima atau laporkan masalah.", Icons.Default.TaskAlt, badge = serahPending),
    )
    Role.KASI_SARPRAS -> listOf(
        TaskCardModel(Dest.APPROVAL, "Persetujuan Permintaan", "Periksa dan setujui/tolak permintaan yang menunggu.", Icons.Default.TaskAlt, badge = pendingApproval),
        TaskCardModel(Dest.PO, "Pesanan (PO)", "Lihat dan kelola pesanan pembelian.", Icons.Default.ShoppingCart, badge = pendingPoDraft),
    )
    Role.KABID_YANMED -> listOf(
        TaskCardModel(Dest.APPROVAL, "Persetujuan Permintaan", "Setujui atau tolak permintaan pada tahap Anda.", Icons.Default.TaskAlt, badge = pendingApproval),
    )
    Role.WADIR_YANMED -> listOf(
        TaskCardModel(Dest.APPROVAL, "Persetujuan Permintaan", "Setujui atau tolak permintaan pada tahap Anda.", Icons.Default.TaskAlt, badge = pendingApproval),
        TaskCardModel(Dest.PO, "Persetujuan Pesanan (PO)", "Setujui atau tolak pesanan pembelian.", Icons.Default.ShoppingCart, badge = pendingPoDraft),
    )
    Role.KABAG_KEUANGAN -> listOf(
        TaskCardModel(Dest.PO, "Persetujuan Pesanan (PO)", "Tinjau dan setujui/tolak pesanan pembelian.", Icons.Default.ShoppingCart, badge = pendingPoDraft),
    )
    Role.PETUGAS_GUDANG -> listOf(
        TaskCardModel(Dest.PENERIMAAN, "Penerimaan Barang", "Catat barang datang (qty, kondisi, dokumen, foto).", Icons.Default.Inventory, badge = poSent),
        TaskCardModel(Dest.SERAH, "Serah Terima (BAST)", "Buat BAST dan serahkan barang ke ruangan.", Icons.Default.LocalShipping, badge = serahSiap),
    )
    Role.ADMIN -> listOf(
        TaskCardModel(Dest.APPROVAL, "Persetujuan Permintaan", "Kelola persetujuan permintaan.", Icons.Default.TaskAlt, badge = pendingApproval),
        TaskCardModel(Dest.PO, "Pesanan (PO)", "Lihat dan kelola pesanan.", Icons.Default.ShoppingCart),
        TaskCardModel(Dest.PERMINTAAN, "Permintaan", "Pantau permintaan.", Icons.Default.Assignment),
        TaskCardModel(Dest.PENERIMAAN, "Penerimaan Barang", "Catat penerimaan.", Icons.Default.Inventory),
        TaskCardModel(Dest.SERAH, "Serah Terima (BAST)", "Kelola BAST.", Icons.Default.LocalShipping),
    )
    Role.ADMIN_ASET, Role.AUDITOR -> emptyList()
    else -> emptyList()
}
