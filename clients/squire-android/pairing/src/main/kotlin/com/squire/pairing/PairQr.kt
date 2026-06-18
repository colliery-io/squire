package com.squire.pairing

import android.graphics.Bitmap
import android.net.Uri
import com.google.zxing.BarcodeFormat
import com.journeyapps.barcodescanner.BarcodeEncoder

/**
 * Builds the `squire://pair?host=&port=&household=&code=` payload and renders it as a QR bitmap
 * (SQUIRE-T-0075). The reverse of [PairTarget.parse]: the parent phone, when pairing a *new* device,
 * mints a one-time code and shows this QR for the new device's scanner.
 *
 * Generation reuses the ZXing core already on the classpath via `zxing-android-embedded` (the same
 * dependency that powers the scanner) — no new library, and the rendering matches what the Keep
 * produces server-side, so a code minted from the phone scans identically.
 */
object PairQr {
    /** Assemble the canonical pairing URI a scanner ([PairTarget.parse]) will accept. */
    fun payload(target: PairTarget): String =
        Uri.Builder()
            .scheme("squire")
            .authority("pair")
            .appendQueryParameter("host", target.host)
            .appendQueryParameter("port", target.port.toString())
            .appendQueryParameter("household", target.household)
            .appendQueryParameter("code", target.code)
            .build()
            .toString()

    /** Encode [content] as a square QR [Bitmap] of [sizePx] on a side, or null if encoding fails. */
    fun bitmap(content: String, sizePx: Int = 600): Bitmap? =
        runCatching { BarcodeEncoder().encodeBitmap(content, BarcodeFormat.QR_CODE, sizePx, sizePx) }
            .getOrNull()
}
