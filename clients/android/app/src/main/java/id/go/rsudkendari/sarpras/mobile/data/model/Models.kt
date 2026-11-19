package id.go.rsudkendari.sarpras.mobile.data.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

// ---------- Generic / paging ----------
@Serializable
data class PagedResult<T>(
    val items: List<T> = emptyList(),
    val total: Int = 0,
)

@Serializable
data class UnreadCount(val unread: Int = 0)

@Serializable
data class SimpleOk(val ok: Boolean? = null)

// ---------- Notifikasi ----------
@Serializable
data class Notifikasi(
    val id: Long,
    val tipe: String = "",
    @SerialName("ref_id") val refId: Long? = null,
    val pesan: String = "",
    @SerialName("is_read") val isRead: Boolean = false,
    @SerialName("created_at") val createdAt: String? = null,
)

// ---------- Referensi ----------
@Serializable
data class Ref(val id: Long = 0, val nama: String = "", val kode: String? = null)

@Serializable
data class RefRuangan(val id: Long = 0, val nama: String = "", val kode: String? = null)

@Serializable
data class ItemRef(
    val id: Long = 0,
    val nama: String = "",
    val kategori: String = "",
)

@Serializable
data class SatuanRef(val id: Long = 0, val kode: String = "")

@Serializable
data class RuanganOpt(val id: Long = 0, val nama: String = "", val kode: String? = null, @SerialName("is_active") val isActive: Boolean = true)

@Serializable
data class UserOpt(
    val id: Long = 0,
    val nama: String = "",
    val ruangan: RefRuangan? = null,
)

@Serializable
data class ItemOpt(
    val id: Long = 0,
    val nama: String = "",
    @SerialName("satuan_id") val satuanId: Long = 0,
    @SerialName("satuan_kode") val satuanKode: String = "",
    val kategori: String = "",
)

@Serializable
data class SatuanOpt(val id: Long = 0, val kode: String = "", val nama: String = "")

// ---------- Approval / Permintaan ----------
@Serializable
data class InboxItem(
    val id: Long = 0,
    @SerialName("batch_id") val batchId: String? = null,
    val tahun: Int? = null,
    val ruangan: String = "",
    val item: String = "",
    val kategori: String = "",
    val jumlah: String = "0",
    val satuan: String = "",
    val keterangan: String? = null,
    val status: String = "",
    @SerialName("current_step") val currentStep: Int = 0,
    @SerialName("sla_deadline") val slaDeadline: String? = null,
    val pemohon: String = "",
    @SerialName("submitted_at") val submittedAt: String? = null,
    @SerialName("updated_at") val updatedAt: String? = null,
)

@Serializable
data class ApprovalRiwayat(
    val id: Long = 0,
    @SerialName("permintaan_id") val permintaanId: Long = 0,
    val step: Int = 0,
    val actor: String = "",
    @SerialName("actor_role") val actorRole: String = "",
    val aksi: String = "",
    val catatan: String? = null,
    val timestamp: String? = null,
)

@Serializable
data class KeputusanBody(val catatan: String? = null)

@Serializable
data class PermintaanRow(
    val id: Long = 0,
    @SerialName("batch_id") val batchId: String? = null,
    val tahun: Int? = null,
    val ruangan: RefRuangan = RefRuangan(),
    val item: ItemRef = ItemRef(),
    val jumlah: String = "0",
    val satuan: SatuanRef = SatuanRef(),
    val keterangan: String? = null,
    val status: String = "",
    @SerialName("current_step") val currentStep: Int = 0,
    @SerialName("revisi_ke") val revisiKe: Int = 1,
    @SerialName("sla_deadline") val slaDeadline: String? = null,
    @SerialName("sla_status") val slaStatus: String? = null,
    @SerialName("created_by") val createdBy: Long = 0,
    @SerialName("submitted_at") val submittedAt: String? = null,
    @SerialName("closed_at") val closedAt: String? = null,
    @SerialName("updated_at") val updatedAt: String? = null,
)

@Serializable
data class ItemInputBody(
    @SerialName("item_id") val itemId: Long,
    val jumlah: String,
    @SerialName("satuan_id") val satuanId: Long,
    val keterangan: String? = null,
)

@Serializable
data class CreatePermintaanBody(
    @SerialName("ruangan_id") val ruanganId: Long? = null,
    val items: List<ItemInputBody>,
    @SerialName("lampiran_ada") val lampiranAda: Boolean = false,
)

@Serializable
data class CreatePermintaanResult(
    @SerialName("batch_id") val batchId: String = "",
    @SerialName("jumlah_baris") val jumlahBaris: Int = 0,
    val status: String = "",
    val data: List<PermintaanRow> = emptyList(),
)

@Serializable
data class SubmitBody(val urgent: Boolean = false)

// ---------- PO ----------
@Serializable
data class SupplierRef(val id: Long = 0, val nama: String = "")

@Serializable
data class PoHeader(
    val id: Long = 0,
    @SerialName("no_po") val noPo: String = "",
    val tahun: Int? = null,
    val supplier: SupplierRef = SupplierRef(),
    @SerialName("tanggal_po") val tanggalPo: String? = null,
    @SerialName("nilai_total") val nilaiTotal: String = "0",
    val ppn: String = "0",
    val status: String = "",
    @SerialName("created_by") val createdBy: Long? = null,
    @SerialName("approved_by") val approvedBy: Long? = null,
    @SerialName("approved_at") val approvedAt: String? = null,
    val catatan: String? = null,
    @SerialName("sla_deadline") val slaDeadline: String? = null,
    @SerialName("sla_status") val slaStatus: String? = null,
    @SerialName("updated_at") val updatedAt: String? = null,
)

@Serializable
data class PoDetail(
    val id: Long = 0,
    @SerialName("permintaan_id") val permintaanId: Long? = null,
    val item: ItemRef = ItemRef(),
    val qty: String = "0",
    @SerialName("harga_satuan") val hargaSatuan: String = "0",
    val subtotal: String = "0",
    val satuan: String = "",
    @SerialName("qty_diterima") val qtyDiterima: String = "0",
)

@Serializable
data class PoDetailResponse(
    val header: PoHeader = PoHeader(),
    val details: List<PoDetail> = emptyList(),
)

@Serializable
data class CatatanBody(val catatan: String? = null)

// ---------- Penerimaan ----------
@Serializable
data class SiapTerimaRow(
    @SerialName("po_detail_id") val poDetailId: Long = 0,
    @SerialName("item_id") val itemId: Long = 0,
    val item: String = "",
    val satuan: String = "",
    @SerialName("qty_po") val qtyPo: String = "0",
    @SerialName("sudah_terima") val sudahTerima: String = "0",
    val sisa: String = "0",
    @SerialName("harga_satuan") val hargaSatuan: String = "0",
)

@Serializable
data class TerimaBody(
    @SerialName("po_detail_id") val poDetailId: Long,
    @SerialName("qty_terima") val qtyTerima: String,
    @SerialName("qty_reject") val qtyReject: String = "0",
    val kondisi: String? = null,
    @SerialName("no_do") val noDo: String? = null,
    @SerialName("no_invoice") val noInvoice: String? = null,
    @SerialName("no_faktur_pajak") val noFakturPajak: String? = null,
    val catatan: String? = null,
    @SerialName("tanggal_terima") val tanggalTerima: String? = null,
)

@Serializable
data class PenerimaanRow(
    val id: Long = 0,
    @SerialName("no_terima") val noTerima: String = "",
    @SerialName("no_po") val noPo: String = "",
    @SerialName("po_detail_id") val poDetailId: Long = 0,
    val item: String = "",
    @SerialName("tanggal_terima") val tanggalTerima: String? = null,
    @SerialName("qty_terima") val qtyTerima: String = "0",
    @SerialName("qty_reject") val qtyReject: String = "0",
    @SerialName("qty_po") val qtyPo: String = "0",
    val kondisi: String? = null,
    @SerialName("no_do") val noDo: String? = null,
    @SerialName("no_invoice") val noInvoice: String? = null,
    @SerialName("no_faktur_pajak") val noFakturPajak: String? = null,
    @SerialName("diterima_oleh") val diterimaOleh: Long? = null,
    val catatan: String? = null,
    @SerialName("updated_at") val updatedAt: String? = null,
)

@Serializable
data class UploadResult(
    val id: Long = 0,
    @SerialName("ref_type") val refType: String = "",
    @SerialName("ref_id") val refId: Long = 0,
    val path: String = "",
    val jenis: String? = null,
    @SerialName("uploaded_at") val uploadedAt: String? = null,
)

// ---------- Serah Terima ----------
@Serializable
data class SiapSerahRow(
    @SerialName("penerimaan_id") val penerimaanId: Long = 0,
    @SerialName("no_terima") val noTerima: String = "",
    val item: String = "",
    @SerialName("item_id") val itemId: Long = 0,
    val qty: String = "0",
    @SerialName("tanggal_terima") val tanggalTerima: String? = null,
    @SerialName("no_po") val noPo: String = "",
    @SerialName("harga_satuan") val hargaSatuan: String = "0",
    val nilai: String = "0",
)

@Serializable
data class PenerimaRef(val id: Long = 0, val nama: String = "")

@Serializable
data class SerahRow(
    val id: Long = 0,
    @SerialName("no_ba") val noBa: String? = null,
    @SerialName("no_po") val noPo: String = "",
    @SerialName("no_terima") val noTerima: String = "",
    @SerialName("penerimaan_id") val penerimaanId: Long = 0,
    val item: String = "",
    val ruangan: RefRuangan = RefRuangan(),
    @SerialName("tanggal_serah") val tanggalSerah: String? = null,
    @SerialName("qty_serah") val qtySerah: String = "0",
    val penerima: PenerimaRef = PenerimaRef(),
    val pemberi: PenerimaRef = PenerimaRef(),
    @SerialName("file_ba") val fileBa: String? = null,
    val catatan: String? = null,
    @SerialName("status_konfirmasi") val statusKonfirmasi: String = "",
    @SerialName("updated_at") val updatedAt: String? = null,
)

@Serializable
data class BuatSerahBody(
    @SerialName("penerimaan_id") val penerimaanId: Long,
    @SerialName("ruangan_id") val ruanganId: Long,
    @SerialName("penerima_ruangan_id") val penerimaRuanganId: Long,
    val qty: String? = null,
    @SerialName("tanggal_serah") val tanggalSerah: String? = null,
    val catatan: String? = null,
)

// ---------- Dashboard helpers ----------
@Serializable
data class PendingCount(val pending: Int = 0)

@Serializable
data class GenericData<T>(val data: T)
