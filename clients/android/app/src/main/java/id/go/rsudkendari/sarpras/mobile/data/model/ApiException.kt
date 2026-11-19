package id.go.rsudkendari.sarpras.mobile.data.model

class ApiException(
    val code: String?,
    override val message: String,
    val status: Int = 0,
) : Exception(message)
