package com.squire.pairing

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.withTimeoutOrNull

/**
 * Best-effort mDNS/NSD discovery of the api on the LAN (ADR SQUIRE-A-0010, NFR-6): browse
 * `_squire._tcp` and resolve the first responder's host:port, or return null on timeout.
 *
 * NSD is environment-sensitive (notably unreliable on emulators), so this is strictly a convenience
 * to prefill the host — callers always offer a manual host/port entry, and the pairing QR already
 * carries host/port directly. Requires the server to advertise the service (a small `squire-home`
 * addition, tracked separately); until then `discover` simply times out to null.
 */
class NsdDiscovery(context: Context) {
    private val nsd =
        context.applicationContext.getSystemService(Context.NSD_SERVICE) as NsdManager

    suspend fun discover(timeoutMs: Long = 4000): Pair<String, Int>? =
        withTimeoutOrNull(timeoutMs) {
            val result = CompletableDeferred<Pair<String, Int>?>()
            val listener = object : NsdManager.DiscoveryListener {
                override fun onDiscoveryStarted(serviceType: String) {}
                override fun onServiceFound(info: NsdServiceInfo) {
                    @Suppress("DEPRECATION")
                    nsd.resolveService(info, object : NsdManager.ResolveListener {
                        override fun onResolveFailed(s: NsdServiceInfo, errorCode: Int) {}
                        override fun onServiceResolved(s: NsdServiceInfo) {
                            @Suppress("DEPRECATION")
                            val host = s.host?.hostAddress
                            if (host != null && !result.isCompleted) result.complete(host to s.port)
                        }
                    })
                }
                override fun onServiceLost(info: NsdServiceInfo) {}
                override fun onDiscoveryStopped(serviceType: String) {}
                override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                    if (!result.isCompleted) result.complete(null)
                }
                override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {}
            }
            try {
                nsd.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, listener)
                result.await()
            } finally {
                runCatching { nsd.stopServiceDiscovery(listener) }
            }
        }

    private companion object {
        const val SERVICE_TYPE = "_squire._tcp."
    }
}
