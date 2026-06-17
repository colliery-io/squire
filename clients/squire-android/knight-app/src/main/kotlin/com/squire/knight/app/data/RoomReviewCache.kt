package com.squire.knight.app.data

import com.squire.knight.app.data.db.CachedReviewEntity
import com.squire.knight.app.data.db.ReviewCacheDao
import com.squire.knight.core.ReviewCache

/**
 * Durable [ReviewCache] backed by Room. Persists the last `GET /household-review` JSON blob in a
 * single-row table so the parent can triage offline across app/process restarts (REQ-K1/K10).
 */
class RoomReviewCache(private val dao: ReviewCacheDao) : ReviewCache {
    override fun save(raw: String) {
        dao.save(CachedReviewEntity(id = 0, json = raw))
    }

    override fun load(): String? = dao.load()
}
