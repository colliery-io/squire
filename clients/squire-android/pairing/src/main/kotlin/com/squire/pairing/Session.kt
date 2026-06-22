package com.squire.pairing

import kotlinx.serialization.Serializable

/**
 * A paired device's durable session (ADR SQUIRE-A-0010): the LAN address of the api, the household
 * it belongs to, and the **per-user tenant-scoped token** obtained from `POST /pair` (or the debug
 * demo login). Persisted encrypted by [SessionStore]; the app's transport adapter reads [baseUrl] +
 * [token] from it instead of any baked credentials.
 */
@Serializable
data class Session(
    val host: String,
    val port: Int,
    val household: String,
    val token: String,
    val user: Long,
    val role: String,
    /** The member's display name, for the "Hi, <name>!" header. Default "" so older stored sessions
     *  (saved before this field existed) still deserialize; the app falls back to a generic header. */
    val displayName: String = "",
) {
    val baseUrl: String get() = "http://$host:$port"
}
