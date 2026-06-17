package com.squire.knight.core

/**
 * An idempotent, order-preserving queue of pending privileged commands, keyed by each command's
 * composite [KnightCommand.key]. Re-enqueueing a known key is a no-op (REQ-K8: idempotent outbox).
 */
interface PrivilegedOutbox {
    /** Add a command. If its [KnightCommand.key] is already present, this is a no-op. */
    fun enqueue(command: KnightCommand)

    /** Pending commands, in insertion order. */
    fun pending(): List<KnightCommand>

    /** Remove commands whose keys are now resolved (acked, already-done, or dropped as poison). */
    fun markResolved(keys: Set<String>)
}

/** In-memory [PrivilegedOutbox] backed by an insertion-ordered map keyed by [KnightCommand.key]. */
class InMemoryPrivilegedOutbox : PrivilegedOutbox {
    private val items = LinkedHashMap<String, KnightCommand>()

    override fun enqueue(command: KnightCommand) {
        items.putIfAbsent(command.key, command)
    }

    override fun pending(): List<KnightCommand> = items.values.toList()

    override fun markResolved(keys: Set<String>) {
        keys.forEach { items.remove(it) }
    }
}
