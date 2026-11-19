package id.go.rsudkendari.sarpras.mobile.data.repo

import android.content.Context
import id.go.rsudkendari.sarpras.mobile.data.model.ApiEnvelope
import id.go.rsudkendari.sarpras.mobile.data.model.ApiException
import id.go.rsudkendari.sarpras.mobile.data.model.BuatSerahBody
import id.go.rsudkendari.sarpras.mobile.data.model.CatatanBody
import id.go.rsudkendari.sarpras.mobile.data.model.CreatePermintaanBody
import id.go.rsudkendari.sarpras.mobile.data.model.CreatePermintaanResult
import id.go.rsudkendari.sarpras.mobile.data.model.InboxItem
import id.go.rsudkendari.sarpras.mobile.data.model.ItemOpt
import id.go.rsudkendari.sarpras.mobile.data.model.KeputusanBody
import id.go.rsudkendari.sarpras.mobile.data.model.Notifikasi
import id.go.rsudkendari.sarpras.mobile.data.model.PenerimaanRow
import id.go.rsudkendari.sarpras.mobile.data.model.PermintaanRow
import id.go.rsudkendari.sarpras.mobile.data.model.PoDetailResponse
import id.go.rsudkendari.sarpras.mobile.data.model.PoHeader
import id.go.rsudkendari.sarpras.mobile.data.model.RuanganOpt
import id.go.rsudkendari.sarpras.mobile.data.model.SatuanOpt
import id.go.rsudkendari.sarpras.mobile.data.model.SerahRow
import id.go.rsudkendari.sarpras.mobile.data.model.SiapSerahRow
import id.go.rsudkendari.sarpras.mobile.data.model.SiapTerimaRow
import id.go.rsudkendari.sarpras.mobile.data.model.SubmitBody
import id.go.rsudkendari.sarpras.mobile.data.model.TerimaBody
import id.go.rsudkendari.sarpras.mobile.data.model.UploadResult
import id.go.rsudkendari.sarpras.mobile.data.model.UserOpt
import id.go.rsudkendari.sarpras.mobile.data.remote.ApiClient
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import retrofit2.Response
import java.io.File

data class Cached<T>(val data: T, val updatedAt: Long, val fromCache: Boolean)

class MobileRepository(
    private val context: Context,
    private val apiClient: ApiClient,
) {
    private val api get() = apiClient.api
    private val json = Json { ignoreUnknownKeys = true; explicitNulls = false }

    private fun cacheFile(name: String) = File(context.filesDir, "cache_$name.json")

    private inline fun <reified T> readCache(name: String): T? = runCatching {
        val f = cacheFile(name)
        if (!f.exists()) return null
        json.decodeFromString<T>(f.readText())
    }.getOrNull()

    private inline fun <reified T> writeCache(name: String, value: T) {
        runCatching { cacheFile(name).writeText(json.encodeToString(value)) }
    }

    suspend fun <T> unwrap(call: suspend () -> Response<ApiEnvelope<T>>): T =
        withContext(Dispatchers.IO) {
            val res = call()
            val body = res.body()
            if (res.isSuccessful && body?.success == true) {
                @Suppress("UNCHECKED_CAST")
                body.data as T
            } else {
                throw ApiException(
                    body?.error?.code,
                    body?.error?.message ?: "Permintaan gagal (HTTP ${res.code()})",
                    res.code(),
                )
            }
        }

    // ---------- Counts (home) ----------
    suspend fun unreadCount(): Int = unwrap { api.unreadCount() }.unread
    suspend fun approvalPendingCount(): Int = unwrap { api.approvalPendingCount() }.pending

    suspend fun poSentCount(): Int = withContext(Dispatchers.IO) {
        val res = api.poSent(status = "SENT", perPage = 1)
        val body = res.body()
        if (res.isSuccessful && body?.success == true) body.meta?.total ?: (body.data?.size ?: 0)
        else 0
    }

    suspend fun serahPendingCount(): Int = withContext(Dispatchers.IO) {
        val res = api.serahTerimaList(status = "PENDING", perPage = 1)
        val body = res.body()
        if (res.isSuccessful && body?.success == true) body.meta?.total ?: (body.data?.size ?: 0)
        else 0
    }

    suspend fun serahSiapCount(): Int = runCatching { unwrap { api.serahTerimaSiap() }.size }.getOrDefault(0)

    // ---------- Notifikasi ----------
    suspend fun notifikasi(): Cached<List<Notifikasi>> = withContext(Dispatchers.IO) {
        val cacheKey = "notifikasi"
        try {
            val list = unwrap { api.notifikasi(perPage = 50) }
            writeCache(cacheKey, list)
            Cached(list, System.currentTimeMillis(), false)
        } catch (e: Exception) {
            val c = readCache<List<Notifikasi>>(cacheKey)
            if (c != null) Cached(c, cacheFile(cacheKey).lastModified(), true) else throw e
        }
    }

    suspend fun markRead(id: Long) { unwrap { api.markRead(id) } }
    suspend fun markAllRead() { unwrap { api.markAllRead() } }

    // ---------- Approval ----------
    suspend fun approvalInbox(): Cached<List<InboxItem>> = withContext(Dispatchers.IO) {
        val cacheKey = "approval_inbox"
        try {
            val list = unwrap { api.approvalInbox(perPage = 50) }
            writeCache(cacheKey, list)
            Cached(list, System.currentTimeMillis(), false)
        } catch (e: Exception) {
            val c = readCache<List<InboxItem>>(cacheKey)
            if (c != null) Cached(c, cacheFile(cacheKey).lastModified(), true) else throw e
        }
    }

    suspend fun setujui(batchId: String, catatan: String?) = unwrap { api.approvalSetujui(batchId, KeputusanBody(catatan)) }
    suspend fun tolak(batchId: String, catatan: String?) = unwrap { api.approvalTolak(batchId, KeputusanBody(catatan)) }

    // ---------- Permintaan ----------
    suspend fun permintaanList(status: String? = null): Cached<List<PermintaanRow>> = withContext(Dispatchers.IO) {
        val cacheKey = "permintaan_${status ?: "all"}"
        try {
            val list = unwrap { api.permintaanList(perPage = 50, status = status) }
            writeCache(cacheKey, list)
            Cached(list, System.currentTimeMillis(), false)
        } catch (e: Exception) {
            val c = readCache<List<PermintaanRow>>(cacheKey)
            if (c != null) Cached(c, cacheFile(cacheKey).lastModified(), true) else throw e
        }
    }

    suspend fun permintaanBatch(batchId: String): List<PermintaanRow> = unwrap { api.permintaanBatch(batchId) }
    suspend fun permintaanCreate(body: CreatePermintaanBody): CreatePermintaanResult = unwrap { api.permintaanCreate(body) }
    suspend fun permintaanSubmit(batchId: String): List<PermintaanRow> = unwrap { api.permintaanSubmit(batchId, SubmitBody()) }
    suspend fun permintaanDeleteBatch(batchId: String) { unwrap { api.permintaanDeleteBatch(batchId) } }

    // ---------- PO ----------
    suspend fun poList(status: String? = null): Cached<List<PoHeader>> = withContext(Dispatchers.IO) {
        val cacheKey = "po_${status ?: "all"}"
        try {
            val list = unwrap { api.poList(perPage = 50, status = status) }
            writeCache(cacheKey, list)
            Cached(list, System.currentTimeMillis(), false)
        } catch (e: Exception) {
            val c = readCache<List<PoHeader>>(cacheKey)
            if (c != null) Cached(c, cacheFile(cacheKey).lastModified(), true) else throw e
        }
    }

    suspend fun poDetail(id: Long): PoDetailResponse = unwrap { api.poDetail(id) }
    suspend fun poApprove(id: Long): PoHeader = unwrap { api.poApprove(id) }
    suspend fun poReject(id: Long, catatan: String?): PoHeader = unwrap { api.poReject(id, CatatanBody(catatan)) }
    suspend fun poKirim(id: Long): PoHeader = unwrap { api.poKirim(id) }
    suspend fun poSent(): List<PoHeader> = unwrap { api.poSent(perPage = 50) }

    // ---------- Penerimaan ----------
    suspend fun penerimaanSiap(poId: Long): List<SiapTerimaRow> = unwrap { api.penerimaanSiap(poId) }
    suspend fun terima(body: TerimaBody): PenerimaanRow = unwrap { api.terima(body) }
    suspend fun penerimaanList(): List<PenerimaanRow> = unwrap { api.penerimaanList(perPage = 20) }

    suspend fun upload(refType: String, refId: Long, file: File): UploadResult = withContext(Dispatchers.IO) {
        val mime = guessMime(file.name)
        val refTypeBody = refType.toRequestBody("text/plain".toMediaType())
        val refIdBody = refId.toString().toRequestBody("text/plain".toMediaType())
        val filePart = MultipartBody.Part.createFormData("file", file.name, file.asRequestBody(mime.toMediaType()))
        unwrap { api.upload(refTypeBody, refIdBody, filePart) }
    }

    // ---------- Serah Terima ----------
    suspend fun serahTerimaList(status: String? = null): Cached<List<SerahRow>> = withContext(Dispatchers.IO) {
        val cacheKey = "serah_${status ?: "all"}"
        try {
            val list = unwrap { api.serahTerimaList(perPage = 50, status = status) }
            writeCache(cacheKey, list)
            Cached(list, System.currentTimeMillis(), false)
        } catch (e: Exception) {
            val c = readCache<List<SerahRow>>(cacheKey)
            if (c != null) Cached(c, cacheFile(cacheKey).lastModified(), true) else throw e
        }
    }

    suspend fun serahTerimaSiap(): List<SiapSerahRow> = unwrap { api.serahTerimaSiap() }
    suspend fun serahTerimaBuat(body: BuatSerahBody): SerahRow = unwrap { api.serahTerimaBuat(body) }
    suspend fun serahTerimaKonfirmasi(id: Long): SerahRow = unwrap { api.serahTerimaKonfirmasi(id) }
    suspend fun serahTerimaDispute(id: Long, catatan: String?): SerahRow = unwrap { api.serahTerimaDispute(id, CatatanBody(catatan)) }

    // ---------- Referensi ----------
    suspend fun ruangan(): List<RuanganOpt> = unwrap { api.ruangan() }
    suspend fun usersOptions(): List<UserOpt> = unwrap { api.usersOptions() }
    suspend fun itemsOptions(): List<ItemOpt> = unwrap { api.itemsOptions() }
    suspend fun satuan(): List<SatuanOpt> = unwrap { api.satuan() }

    fun clearCache() {
        context.filesDir.listFiles()?.filter { it.name.startsWith("cache_") }?.forEach { it.delete() }
    }

    private fun guessMime(name: String): String = when (name.substringAfterLast('.', "").lowercase()) {
        "jpg", "jpeg" -> "image/jpeg"
        "png" -> "image/png"
        "pdf" -> "application/pdf"
        else -> "application/octet-stream"
    }
}
