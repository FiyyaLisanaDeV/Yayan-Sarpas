package id.go.rsudkendari.sarpras.mobile.data.remote

import id.go.rsudkendari.sarpras.mobile.data.model.ApiEnvelope
import id.go.rsudkendari.sarpras.mobile.data.model.AuthData
import id.go.rsudkendari.sarpras.mobile.data.model.BuatSerahBody
import id.go.rsudkendari.sarpras.mobile.data.model.CatatanBody
import id.go.rsudkendari.sarpras.mobile.data.model.CreatePermintaanBody
import id.go.rsudkendari.sarpras.mobile.data.model.CreatePermintaanResult
import id.go.rsudkendari.sarpras.mobile.data.model.InboxItem
import id.go.rsudkendari.sarpras.mobile.data.model.ItemOpt
import id.go.rsudkendari.sarpras.mobile.data.model.KeputusanBody
import id.go.rsudkendari.sarpras.mobile.data.model.LoginRequest
import id.go.rsudkendari.sarpras.mobile.data.model.LogoutRequest
import id.go.rsudkendari.sarpras.mobile.data.model.Notifikasi
import id.go.rsudkendari.sarpras.mobile.data.model.PenerimaanRow
import id.go.rsudkendari.sarpras.mobile.data.model.PendingCount
import id.go.rsudkendari.sarpras.mobile.data.model.PermintaanRow
import id.go.rsudkendari.sarpras.mobile.data.model.PoDetailResponse
import id.go.rsudkendari.sarpras.mobile.data.model.PoHeader
import id.go.rsudkendari.sarpras.mobile.data.model.RefreshRequest
import id.go.rsudkendari.sarpras.mobile.data.model.RuanganOpt
import id.go.rsudkendari.sarpras.mobile.data.model.SatuanOpt
import id.go.rsudkendari.sarpras.mobile.data.model.SerahRow
import id.go.rsudkendari.sarpras.mobile.data.model.SiapSerahRow
import id.go.rsudkendari.sarpras.mobile.data.model.SiapTerimaRow
import id.go.rsudkendari.sarpras.mobile.data.model.SubmitBody
import id.go.rsudkendari.sarpras.mobile.data.model.TerimaBody
import id.go.rsudkendari.sarpras.mobile.data.model.UnreadCount
import id.go.rsudkendari.sarpras.mobile.data.model.UploadResult
import id.go.rsudkendari.sarpras.mobile.data.model.User
import id.go.rsudkendari.sarpras.mobile.data.model.UserOpt
import kotlinx.serialization.json.JsonElement
import okhttp3.MultipartBody
import okhttp3.RequestBody
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.Multipart
import retrofit2.http.POST
import retrofit2.http.Part
import retrofit2.http.Path
import retrofit2.http.Query

interface SarprasApi {
    // --- Auth ---
    @POST("/auth/login")
    suspend fun login(@Body body: LoginRequest): Response<ApiEnvelope<AuthData>>

    @POST("/auth/refresh")
    suspend fun refresh(@Body body: RefreshRequest): Response<ApiEnvelope<AuthData>>

    @GET("/auth/me")
    suspend fun me(): Response<ApiEnvelope<User>>

    @POST("/auth/logout")
    suspend fun logout(@Body body: LogoutRequest): Response<ApiEnvelope<JsonElement?>>

    // --- Notifikasi ---
    @GET("/notifikasi")
    suspend fun notifikasi(
        @Query("page") page: Int = 1,
        @Query("per_page") perPage: Int = 50,
        @Query("only_unread") onlyUnread: Boolean = false,
    ): Response<ApiEnvelope<List<Notifikasi>>>

    @GET("/notifikasi/unread-count")
    suspend fun unreadCount(): Response<ApiEnvelope<UnreadCount>>

    @POST("/notifikasi/{id}/read")
    suspend fun markRead(@Path("id") id: Long): Response<ApiEnvelope<JsonElement?>>

    @POST("/notifikasi/read-all")
    suspend fun markAllRead(): Response<ApiEnvelope<JsonElement?>>

    // --- Approval (permintaan) ---
    @GET("/approval/inbox")
    suspend fun approvalInbox(
        @Query("page") page: Int = 1,
        @Query("per_page") perPage: Int = 50,
        @Query("q") q: String? = null,
    ): Response<ApiEnvelope<List<InboxItem>>>

    @GET("/approval/pending-count")
    suspend fun approvalPendingCount(): Response<ApiEnvelope<PendingCount>>

    @POST("/approval/{batch_id}/setujui")
    suspend fun approvalSetujui(
        @Path("batch_id") batchId: String,
        @Body body: KeputusanBody,
    ): Response<ApiEnvelope<List<InboxItem>>>

    @POST("/approval/{batch_id}/tolak")
    suspend fun approvalTolak(
        @Path("batch_id") batchId: String,
        @Body body: KeputusanBody,
    ): Response<ApiEnvelope<List<InboxItem>>>

    // --- Permintaan ---
    @GET("/permintaan")
    suspend fun permintaanList(
        @Query("page") page: Int = 1,
        @Query("per_page") perPage: Int = 50,
        @Query("q") q: String? = null,
        @Query("status") status: String? = null,
        @Query("tahun") tahun: Int? = null,
    ): Response<ApiEnvelope<List<PermintaanRow>>>

    @POST("/permintaan")
    suspend fun permintaanCreate(@Body body: CreatePermintaanBody): Response<ApiEnvelope<CreatePermintaanResult>>

    @GET("/permintaan/batch/{batch_id}")
    suspend fun permintaanBatch(@Path("batch_id") batchId: String): Response<ApiEnvelope<List<PermintaanRow>>>

    @POST("/permintaan/batch/{batch_id}/submit")
    suspend fun permintaanSubmit(
        @Path("batch_id") batchId: String,
        @Body body: SubmitBody,
    ): Response<ApiEnvelope<List<PermintaanRow>>>

    @DELETE("/permintaan/batch/{batch_id}")
    suspend fun permintaanDeleteBatch(@Path("batch_id") batchId: String): Response<ApiEnvelope<JsonElement?>>

    // --- PO ---
    @GET("/po")
    suspend fun poList(
        @Query("page") page: Int = 1,
        @Query("per_page") perPage: Int = 50,
        @Query("status") status: String? = null,
        @Query("tahun") tahun: Int? = null,
        @Query("q") q: String? = null,
    ): Response<ApiEnvelope<List<PoHeader>>>

    @GET("/po/{id}")
    suspend fun poDetail(@Path("id") id: Long): Response<ApiEnvelope<PoDetailResponse>>

    @POST("/po/{id}/approve")
    suspend fun poApprove(@Path("id") id: Long): Response<ApiEnvelope<PoHeader>>

    @POST("/po/{id}/reject")
    suspend fun poReject(@Path("id") id: Long, @Body body: CatatanBody): Response<ApiEnvelope<PoHeader>>

    @POST("/po/{id}/kirim")
    suspend fun poKirim(@Path("id") id: Long): Response<ApiEnvelope<PoHeader>>

    // --- Penerimaan ---
    @GET("/po")
    suspend fun poSent(
        @Query("status") status: String = "SENT",
        @Query("per_page") perPage: Int = 50,
    ): Response<ApiEnvelope<List<PoHeader>>>

    @GET("/penerimaan/po/{po_id}/siap")
    suspend fun penerimaanSiap(@Path("po_id") poId: Long): Response<ApiEnvelope<List<SiapTerimaRow>>>

    @POST("/penerimaan")
    suspend fun terima(@Body body: TerimaBody): Response<ApiEnvelope<PenerimaanRow>>

    @GET("/penerimaan")
    suspend fun penerimaanList(
        @Query("page") page: Int = 1,
        @Query("per_page") perPage: Int = 20,
    ): Response<ApiEnvelope<List<PenerimaanRow>>>

    @Multipart
    @POST("/uploads")
    suspend fun upload(
        @Part("ref_type") refType: RequestBody,
        @Part("ref_id") refId: RequestBody,
        @Part file: MultipartBody.Part,
    ): Response<ApiEnvelope<UploadResult>>

    // --- Serah Terima ---
    @GET("/serah-terima")
    suspend fun serahTerimaList(
        @Query("page") page: Int = 1,
        @Query("per_page") perPage: Int = 50,
        @Query("status") status: String? = null,
    ): Response<ApiEnvelope<List<SerahRow>>>

    @GET("/serah-terima/siap")
    suspend fun serahTerimaSiap(): Response<ApiEnvelope<List<SiapSerahRow>>>

    @POST("/serah-terima")
    suspend fun serahTerimaBuat(@Body body: BuatSerahBody): Response<ApiEnvelope<SerahRow>>

    @POST("/serah-terima/{id}/konfirmasi")
    suspend fun serahTerimaKonfirmasi(@Path("id") id: Long): Response<ApiEnvelope<SerahRow>>

    @POST("/serah-terima/{id}/dispute")
    suspend fun serahTerimaDispute(@Path("id") id: Long, @Body body: CatatanBody): Response<ApiEnvelope<SerahRow>>

    // --- Referensi (opsi) ---
    @GET("/ruangan")
    suspend fun ruangan(@Query("per_page") perPage: Int = 200): Response<ApiEnvelope<List<RuanganOpt>>>

    @GET("/users/options")
    suspend fun usersOptions(): Response<ApiEnvelope<List<UserOpt>>>

    @GET("/items/options")
    suspend fun itemsOptions(): Response<ApiEnvelope<List<ItemOpt>>>

    @GET("/satuan")
    suspend fun satuan(@Query("per_page") perPage: Int = 100): Response<ApiEnvelope<List<SatuanOpt>>>
}
