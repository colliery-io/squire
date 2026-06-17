package com.squire.app.data.db

import android.content.Context
import androidx.room.Dao
import androidx.room.Database
import androidx.room.Entity
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.PrimaryKey
import androidx.room.Query
import androidx.room.Room
import androidx.room.RoomDatabase

/**
 * Single-row table holding the last `GET /state` JSON blob. The fixed [id]`=0`
 * means an upsert (INSERT OR REPLACE) always targets the same row.
 */
@Entity(tableName = "cached_state")
data class CachedStateEntity(
    @PrimaryKey val id: Int = 0,
    val json: String,
)

/**
 * One pending offline submission, keyed by the phone-minted [id] (claimId for claims,
 * requestId for redemptions). [kind] discriminates the payload; [payloadJson] is the
 * serialized `SubmitClaimReq`/`RequestRedemptionReq`.
 */
@Entity(tableName = "outbox")
data class OutboxEntity(
    @PrimaryKey val id: Long,
    val kind: String,
    val payloadJson: String,
)

@Dao
interface CacheDao {
    @Query("SELECT json FROM cached_state WHERE id = 0")
    fun load(): String?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    fun save(e: CachedStateEntity)
}

@Dao
interface OutboxDao {
    /** Idempotent by primary-key id: re-inserting a known id is a no-op. */
    @Insert(onConflict = OnConflictStrategy.IGNORE)
    fun insert(e: OutboxEntity)

    @Query("SELECT * FROM outbox")
    fun all(): List<OutboxEntity>

    @Query("DELETE FROM outbox WHERE id IN (:ids)")
    fun deleteByIds(ids: List<Long>)
}

@Database(
    entities = [CachedStateEntity::class, OutboxEntity::class],
    version = 1,
    exportSchema = false,
)
abstract class SquireDb : RoomDatabase() {
    abstract fun cacheDao(): CacheDao
    abstract fun outboxDao(): OutboxDao

    companion object {
        /**
         * Builds the durable Squire database. `allowMainThreadQueries()` is used because the
         * `:core` `Outbox`/`StateCache` ports are synchronous (non-suspend); callers in
         * `PlayerStore` already run on background dispatchers via the adapter/sync, so this is
         * safe at this app's scale. `fallbackToDestructiveMigration()` is fine for a v1 schema.
         */
        fun build(context: Context): SquireDb =
            Room.databaseBuilder(
                context.applicationContext,
                SquireDb::class.java,
                "squire.db",
            )
                .fallbackToDestructiveMigration()
                .allowMainThreadQueries()
                .build()
    }
}
