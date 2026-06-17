package com.squire.app.data

import com.squire.app.data.db.CacheDao
import com.squire.app.data.db.CachedStateEntity
import com.squire.core.StateCache

/**
 * Durable [StateCache] backed by Room. Persists the last `GET /state` JSON blob in a
 * single-row table so the phone can render offline across app/process restarts (REQ-SY1).
 */
class RoomStateCache(private val dao: CacheDao) : StateCache {
    override fun save(raw: String) {
        dao.save(CachedStateEntity(id = 0, json = raw))
    }

    override fun load(): String? = dao.load()
}
