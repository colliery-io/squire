package com.squire.core

/**
 * Caches the last successful `GET /state` blob so the phone can render while the
 * computer is unreachable (offline-first render; NFR-1/6).
 */
interface StateCache {
    fun save(raw: String)
    fun load(): String?
}

/** In-memory [StateCache] holding the most recently cached blob. */
class InMemoryStateCache : StateCache {
    private var cached: String? = null

    override fun save(raw: String) {
        cached = raw
    }

    override fun load(): String? = cached
}
