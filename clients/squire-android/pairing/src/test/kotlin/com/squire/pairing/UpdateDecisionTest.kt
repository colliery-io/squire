package com.squire.pairing

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class UpdateDecisionTest {
    private val a = "a".repeat(64)
    private val b = "b".repeat(64)

    @Test
    fun `different content at the same version is offered`() {
        // The case phase 2 exists for: a rebuilt APK that kept its versionCode.
        assertTrue(UpdateDecision.shouldOffer(serverHash = b, installedHash = a, serverVersionCode = 30, currentVersionCode = 30))
    }

    @Test
    fun `different content at a higher version is offered`() {
        assertTrue(UpdateDecision.shouldOffer(serverHash = b, installedHash = a, serverVersionCode = 31, currentVersionCode = 30))
    }

    @Test
    fun `the same content is not offered, whatever the versions say`() {
        assertFalse(UpdateDecision.shouldOffer(serverHash = a, installedHash = a, serverVersionCode = 31, currentVersionCode = 30))
    }

    @Test
    fun `hashes compare without regard to case`() {
        assertFalse(UpdateDecision.shouldOffer(serverHash = a.uppercase(), installedHash = a, serverVersionCode = 30, currentVersionCode = 30))
    }

    @Test
    fun `a lower version is never offered, because Android refuses the downgrade`() {
        assertFalse(UpdateDecision.shouldOffer(serverHash = b, installedHash = a, serverVersionCode = 29, currentVersionCode = 30))
    }

    @Test
    fun `without the server hash, only a higher version is offered`() {
        assertTrue(UpdateDecision.shouldOffer(serverHash = null, installedHash = a, serverVersionCode = 31, currentVersionCode = 30))
        assertFalse(UpdateDecision.shouldOffer(serverHash = null, installedHash = a, serverVersionCode = 30, currentVersionCode = 30))
    }

    @Test
    fun `without the installed hash, only a higher version is offered`() {
        assertTrue(UpdateDecision.shouldOffer(serverHash = b, installedHash = null, serverVersionCode = 31, currentVersionCode = 30))
        assertFalse(UpdateDecision.shouldOffer(serverHash = b, installedHash = null, serverVersionCode = 30, currentVersionCode = 30))
    }

    @Test
    fun `sha256Hex is the lower-hex SHA-256 of the file bytes`() {
        val f = File.createTempFile("apk", ".bin").apply { deleteOnExit(); writeText("abc") }
        // FIPS 180-2 test vector for "abc" — the server's `apk_sha256` produces the same lower hex.
        assertEquals(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            UpdateDecision.sha256Hex(f),
        )
    }
}
