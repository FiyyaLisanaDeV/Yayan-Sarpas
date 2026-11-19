package id.go.rsudkendari.sarpras.mobile.data.model

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

@Serializable
data class ApiEnvelope<T>(
    val success: Boolean,
    val data: T? = null,
    val error: ApiErrorPayload? = null,
    val meta: ApiMeta? = null,
)

@Serializable
data class ApiErrorPayload(
    val code: String? = null,
    val message: String? = null,
    val details: JsonElement? = null,
)

@Serializable
data class ApiMeta(
    val page: Int? = null,
    val per_page: Int? = null,
    val total: Int? = null,
)
