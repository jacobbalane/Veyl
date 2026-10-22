package io.github.jacobbalane.veyl.capture

import android.content.Context
import java.io.File
import java.io.IOException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Explicit, bounded diagnostic recording; never constructed by release builds. */
object CaptureDebug {
    private val mutable = MutableStateFlow("idle")
    val recording = mutable.asStateFlow()
    private var session: Session? = null

    @Synchronized
    fun tap(context: Context): CaptureTap {
        session?.close()
        return Session(File(context.filesDir, "capture-lab")).also { session = it }
    }

    @Synchronized
    fun record() {
        session?.request()
    }

    @Synchronized
    fun delete(context: Context) {
        session?.cancel()
        listOf("reference.wav", "microphone.wav").forEach {
            File(context.filesDir, "capture-lab/$it").delete()
        }
        mutable.value = "idle"
    }

    private class Session(private val directory: File) : CaptureTap {
        private var requested = false
        private var closed = false
        private val writers = arrayOfNulls<DebugWavWriter>(2)
        private val done = BooleanArray(2)

        @Synchronized
        fun request() {
            if (closed || requested) return
            cancel()
            directory.mkdirs()
            done.fill(false)
            requested = true
            mutable.value = "recording"
        }

        @Synchronized
        override fun block(stream: Int, rate: Int, channels: Int, samples: ShortArray) {
            if (!requested || closed || done[stream]) return
            try {
                val writer =
                    writers[stream]
                        ?: DebugWavWriter(
                                File(
                                    directory,
                                    if (stream == 0) "reference.wav" else "microphone.wav",
                                ),
                                rate,
                                channels,
                            )
                            .also { writers[stream] = it }
                writer.append(samples)
                if (writer.complete) {
                    writer.close()
                    writers[stream] = null
                    done[stream] = true
                }
                if (done.all { it }) {
                    requested = false
                    mutable.value = "saved"
                }
            } catch (_: IOException) {
                cancel()
                mutable.value = "error"
            }
        }

        @Synchronized
        fun cancel() {
            requested = false
            writers.forEachIndexed { index, writer ->
                try {
                    writer?.close()
                } catch (_: IOException) {} finally {
                    writers[index] = null
                }
            }
        }

        @Synchronized
        override fun close() {
            cancel()
            closed = true
            if (mutable.value == "recording") mutable.value = "idle"
        }
    }
}
