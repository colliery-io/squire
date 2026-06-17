package com.squire.pairing

import android.net.Uri

/**
 * The parsed contents of a pairing QR (or a manually-entered set): where to reach the api and the
 * one-time code to exchange. The Keep encodes this as `squire://pair?host=&port=&household=&code=`
 * (ADR SQUIRE-A-0010 / SQUIRE-T-0045).
 */
data class PairTarget(
    val host: String,
    val port: Int,
    val household: String,
    val code: String,
) {
    companion object {
        /** Parse a scanned `squire://pair?...` URI; returns null if it isn't a valid pairing URI. */
        fun parse(raw: String): PairTarget? {
            val uri = runCatching { Uri.parse(raw.trim()) }.getOrNull() ?: return null
            if (uri.scheme != "squire" || uri.host != "pair") return null
            val host = uri.getQueryParameter("host")?.takeIf { it.isNotBlank() } ?: return null
            val port = uri.getQueryParameter("port")?.toIntOrNull() ?: return null
            val household = uri.getQueryParameter("household")?.takeIf { it.isNotBlank() } ?: return null
            val code = uri.getQueryParameter("code")?.takeIf { it.isNotBlank() } ?: return null
            return PairTarget(host, port, household, code)
        }
    }
}
