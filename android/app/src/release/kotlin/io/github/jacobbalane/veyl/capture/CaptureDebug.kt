package io.github.jacobbalane.veyl.capture

import android.content.Context

/** Release has neither a raw-audio observer nor a phone recording implementation. */
object CaptureDebug {
    fun tap(@Suppress("UNUSED_PARAMETER") context: Context): CaptureTap? = null
}
