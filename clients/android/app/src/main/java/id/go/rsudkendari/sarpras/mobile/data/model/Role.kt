package id.go.rsudkendari.sarpras.mobile.data.model

object Role {
    const val USER_RUANGAN = "USER_RUANGAN"
    const val KASI_SARPRAS = "KASI_SARPRAS"
    const val KABID_YANMED = "KABID_YANMED"
    const val WADIR_YANMED = "WADIR_YANMED"
    const val PETUGAS_GUDANG = "PETUGAS_GUDANG"
    const val KABAG_KEUANGAN = "KABAG_KEUANGAN"
    const val ADMIN_ASET = "ADMIN_ASET"
    const val ADMIN = "ADMIN"
    const val AUDITOR = "AUDITOR"

    private val labels = mapOf(
        USER_RUANGAN to "User Ruangan",
        KASI_SARPRAS to "Kasi Sarpras",
        KABID_YANMED to "Kabid Yanmed",
        WADIR_YANMED to "Wadir Yanmed",
        PETUGAS_GUDANG to "Petugas Gudang",
        KABAG_KEUANGAN to "Kabag Keuangan",
        ADMIN_ASET to "Admin Aset",
        ADMIN to "Administrator",
        AUDITOR to "Auditor",
    )

    fun label(role: String): String = labels[role] ?: role
}
