package id.go.rsudkendari.sarpras.mobile.ui.common

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ErrorOutline
import androidx.compose.material.icons.filled.Inbox
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import java.text.NumberFormat
import java.text.SimpleDateFormat
import java.util.Locale
import java.util.TimeZone

@Composable
fun LoadingState(text: String = "Memuat…") {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp)) {
            CircularProgressIndicator()
            Text(text, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
fun ErrorState(message: String, onRetry: (() -> Unit)? = null) {
    Box(Modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Icon(Icons.Default.ErrorOutline, contentDescription = null, tint = MaterialTheme.colorScheme.error)
            Text(message, color = MaterialTheme.colorScheme.error)
            if (onRetry != null) OutlinedButton(onClick = onRetry) { Text("Coba lagi") }
        }
    }
}

@Composable
fun EmptyState(text: String = "Belum ada data.") {
    Box(Modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Icon(Icons.Default.Inbox, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(text, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
fun CacheBanner(text: String = "Menampilkan data terakhir (offline/cache). Muat ulang saat online.") {
    Surface(color = MaterialTheme.colorScheme.tertiaryContainer, modifier = Modifier.fillMaxWidth()) {
        Text(
            text,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onTertiaryContainer,
        )
    }
}

@Composable
fun StatusBadge(status: String) {
    val (bg, fg) = statusColors(status)
    Box(
        Modifier
            .clip(MaterialTheme.shapes.small)
            .background(bg)
            .padding(horizontal = 8.dp, vertical = 2.dp),
    ) {
        Text(labelStatus(status), color = fg, style = MaterialTheme.typography.labelSmall, fontWeight = FontWeight.SemiBold)
    }
}

@Composable
fun CountBadge(count: Int) {
    if (count <= 0) return
    Box(
        Modifier.size(22.dp).clip(CircleShape).background(MaterialTheme.colorScheme.error),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            if (count > 99) "99+" else count.toString(),
            color = MaterialTheme.colorScheme.onError,
            style = MaterialTheme.typography.labelSmall,
            fontWeight = FontWeight.Bold,
        )
    }
}

@Composable
fun SectionCard(title: String? = null, content: @Composable () -> Unit) {
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            if (title != null) Text(title, fontWeight = FontWeight.SemiBold)
            content()
        }
    }
}

fun labelStatus(code: String): String = when (code.uppercase()) {
    "DRAFT" -> "Draft"
    "SUBMITTED" -> "Diajukan"
    "REJECTED", "RETURNED" -> "Ditolak / Perlu perbaikan"
    "VERIFIKASI_KASI", "VERIFIKASI" -> "Verifikasi Kasi"
    "APPROVED_KABID" -> "Disetujui Kabid"
    "APPROVED_WADIR", "APPROVED" -> "Disetujui"
    "FINAL" -> "Final"
    "IN_PO" -> "Dalam PO"
    "CLOSED" -> "Selesai"
    "SENT" -> "Dikirim / Dalam perjalanan"
    "PARTIAL_RECEIVED" -> "Diterima sebagian"
    "PENDING" -> "Menunggu konfirmasi"
    "CONFIRMED" -> "Dikonfirmasi"
    "DISPUTED" -> "Sengketa"
    "RESOLVED" -> "Diselesaikan"
    else -> code.replace('_', ' ').lowercase().replaceFirstChar { it.titlecase(Locale("id", "ID")) }
}

fun statusColors(status: String): Pair<Color, Color> = when (status.uppercase()) {
    "FINAL", "APPROVED", "APPROVED_WADIR", "CONFIRMED", "CLOSED", "RESOLVED" -> Color(0xFFDCFCE7) to Color(0xFF166534)
    "REJECTED", "RETURNED", "DISPUTED" -> Color(0xFFFEE2E2) to Color(0xFF991B1B)
    "DRAFT" -> Color(0xFFE2E8F0) to Color(0xFF334155)
    "SENT", "PARTIAL_RECEIVED", "IN_PO" -> Color(0xFFDBEAFE) to Color(0xFF1D4ED8)
    else -> Color(0xFFFEF3C7) to Color(0xFF92400E)
}

fun formatDate(iso: String?): String {
    if (iso.isNullOrBlank()) return "—"
    return try {
        val cleaned = iso.replace("T", " ").take(19)
        val inFmt = SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.US).apply { timeZone = TimeZone.getTimeZone("UTC") }
        val date = inFmt.parse(cleaned) ?: return iso.take(10)
        SimpleDateFormat("dd MMM yyyy, HH:mm 'WITA'", Locale("id", "ID")).apply {
            timeZone = TimeZone.getTimeZone("Asia/Makassar")
        }.format(date)
    } catch (_: Exception) {
        iso.take(10)
    }
}

fun formatRupiah(v: String?): String {
    val n = v?.toDoubleOrNull() ?: return "Rp 0"
    val fmt = NumberFormat.getCurrencyInstance(Locale("id", "ID")).apply { maximumFractionDigits = 0 }
    return fmt.format(n)
}

fun errMsg(e: Throwable): String = when (e) {
    is id.go.rsudkendari.sarpras.mobile.data.model.ApiException -> e.message
    else -> {
        val m = e.message ?: ""
        when {
            m.contains("Unable to resolve host", true) || m.contains("Failed to connect", true) ||
                m.contains("timeout", true) -> "Tidak dapat terhubung ke server. Periksa jaringan/alamat server."
            else -> m.ifBlank { "Terjadi kesalahan, coba lagi." }
        }
    }
}
