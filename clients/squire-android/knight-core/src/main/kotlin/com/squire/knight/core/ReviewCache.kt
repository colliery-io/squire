package com.squire.knight.core

/**
 * Caches the last successful `GET /household-review` blob so the parent can triage while the Keep
 * is unreachable (offline-first render; REQ-K1 / REQ-K10 / NFR-1).
 */
interface ReviewCache {
    fun save(raw: String)
    fun load(): String?
}

/** In-memory [ReviewCache] holding the most recently cached blob. */
class InMemoryReviewCache : ReviewCache {
    private var cached: String? = null

    override fun save(raw: String) {
        cached = raw
    }

    override fun load(): String? = cached
}
