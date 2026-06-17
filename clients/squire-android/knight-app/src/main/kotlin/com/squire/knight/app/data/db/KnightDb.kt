package com.squire.knight.app.data.db

import android.content.Context
import androidx.room.Dao
import androidx.room.Database
import androidx.room.Entity
import androidx.room.Index
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.PrimaryKey
import androidx.room.Query
import androidx.room.Room
import androidx.room.RoomDatabase

/**
 * Single-row table holding the last `GET /household-review` JSON blob. The fixed [id]`=0`
 * means an upsert (INSERT OR REPLACE) always targets the same row (REQ-K1 offline render).
 */
@Entity(tableName = "cached_review")
data class CachedReviewEntity(
    @PrimaryKey val id: Int = 0,
    val json: String,
)

/**
 * One pending privileged command, keyed by the `:knight-core` composite [key] (`kind:id`).
 * [seq] gives durable insertion order; the unique index on [key] makes `INSERT OR IGNORE`
 * idempotent (REQ-K8). [payloadJson] is the serialized `:sdk` request DTO.
 */
@Entity(tableName = "knight_outbox", indices = [Index(value = ["key"], unique = true)])
data class PrivilegedOutboxEntity(
    @PrimaryKey(autoGenerate = true) val seq: Long = 0,
    val key: String,
    val kind: String,
    val payloadJson: String,
)

@Dao
interface ReviewCacheDao {
    @Query("SELECT json FROM cached_review WHERE id = 0")
    fun load(): String?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    fun save(e: CachedReviewEntity)
}

@Dao
interface PrivilegedOutboxDao {
    /** Idempotent by the unique `key` index: re-inserting a known command key is a no-op. */
    @Insert(onConflict = OnConflictStrategy.IGNORE)
    fun insert(e: PrivilegedOutboxEntity)

    @Query("SELECT * FROM knight_outbox ORDER BY seq")
    fun all(): List<PrivilegedOutboxEntity>

    @Query("DELETE FROM knight_outbox WHERE key IN (:keys)")
    fun deleteByKeys(keys: List<String>)
}

@Database(
    entities = [CachedReviewEntity::class, PrivilegedOutboxEntity::class],
    version = 1,
    exportSchema = false,
)
abstract class KnightDb : RoomDatabase() {
    abstract fun reviewCacheDao(): ReviewCacheDao
    abstract fun outboxDao(): PrivilegedOutboxDao

    companion object {
        /**
         * Builds the durable Knight database (`knight.db`, separate from the Squire app's
         * `squire.db`). `allowMainThreadQueries()` is acceptable because the `:knight-core` ports
         * are synchronous and their callers already run on background dispatchers via the adapter;
         * `fallbackToDestructiveMigration()` is fine for a v1 schema.
         */
        fun build(context: Context): KnightDb =
            Room.databaseBuilder(
                context.applicationContext,
                KnightDb::class.java,
                "knight.db",
            )
                .fallbackToDestructiveMigration()
                .allowMainThreadQueries()
                .build()
    }
}
