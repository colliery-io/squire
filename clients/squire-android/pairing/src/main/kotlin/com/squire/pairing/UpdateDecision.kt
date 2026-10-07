package com.squire.pairing

import java.io.File
import java.security.MessageDigest

/**
 * The pure part of OTA update detection (SQUIRE-T-0085, phase 2): whether to offer the server's
 * build, and the content hash both sides compare. No Android types, so it is JVM-tested.
 */
internal object UpdateDecision {

    /**
     * Offer the server's build when its content differs from the installed APK. [serverHash] is the
     * server-computed `sha256` from the manifest; [installedHash] is the SHA-256 of this app's own
     * installed APK. The version is only a guard here, not the signal: a lower [serverVersionCode]
     * is never offered, because Android refuses a downgrade and the banner would never clear.
     *
     * When either hash is missing (a server from before 0.7.2, or the installed APK is unreadable),
     * fall back to "a higher versionCode is an update".
     */
    fun shouldOffer(
        serverHash: String?,
        installedHash: String?,
        serverVersionCode: Int,
        currentVersionCode: Int,
    ): Boolean {
        if (serverHash == null || installedHash == null) return serverVersionCode > currentVersionCode
        if (serverVersionCode < currentVersionCode) return false
        return !serverHash.equals(installedHash, ignoreCase = true)
    }

    /** Lower-hex SHA-256 of [file] — the same form as the server's `apk_sha256`. */
    fun sha256Hex(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().buffered().use { input ->
            val buf = ByteArray(64 * 1024)
            while (true) {
                val n = input.read(buf)
                if (n < 0) break
                digest.update(buf, 0, n)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }
}
