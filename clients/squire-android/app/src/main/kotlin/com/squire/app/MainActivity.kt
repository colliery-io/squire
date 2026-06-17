package com.squire.app

import android.app.Activity
import android.os.Bundle
import android.widget.TextView
import com.squire.core.InMemoryOutbox
import com.squire.core.InMemoryStateCache

/**
 * Skeleton entry point. The full Compose UI + Room land in a later task; this only
 * proves the module configures and compiles against android-34 while linking :core.
 */
class MainActivity : Activity() {
    private val outbox = InMemoryOutbox()
    private val stateCache = InMemoryStateCache()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val view = TextView(this).apply {
            text = "Squire — ${outbox.pending().size} pending; cache=${stateCache.load()}"
        }
        setContentView(view)
    }
}
