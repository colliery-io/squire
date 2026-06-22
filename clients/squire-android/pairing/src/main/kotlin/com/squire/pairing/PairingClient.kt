package com.squire.pairing

import com.squire.sdk.api.ControlApi
import com.squire.sdk.model.PairReq
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Exchanges a one-time pairing code for a durable [Session] via the api's unauthenticated
 * `POST /pair` (ADR SQUIRE-A-0010 / SQUIRE-T-0044), using the generated [ControlApi]. Throws on a
 * bad/expired/used code (the api returns 401) or an unreachable host — the caller surfaces that.
 */
object PairingClient {
    suspend fun pair(target: PairTarget): Session = withContext(Dispatchers.IO) {
        val control = ControlApi(basePath = "http://${target.host}:${target.port}")
        val resp = control.pair(PairReq(code = target.code, household = target.household))
        Session(
            host = target.host,
            port = target.port,
            household = resp.household,
            token = resp.token,
            user = resp.user,
            role = resp.role.value,
            displayName = resp.displayName.orEmpty(),
        )
    }
}
