package io.github.jacobbalane.veyl.capture

import android.media.AudioManager
import android.media.AudioRecord
import android.media.AudioTimestamp
import android.media.projection.MediaProjection
import android.os.Process
import android.os.SystemClock
import io.github.jacobbalane.veyl.core.audio.BlockPacker
import io.github.jacobbalane.veyl.core.audio.TimestampTracker
import io.github.jacobbalane.veyl.runtime.VeylRuntime
import java.util.concurrent.atomic.AtomicBoolean

interface CaptureTap {
    fun block(stream: Int, rate: Int, channels: Int, samples: ShortArray)

    fun close()
}

class CaptureRun(
    private val runtime: VeylRuntime,
    private val id: Long,
    private val tap: CaptureTap?,
    private val error: (String) -> Unit,
) : AutoCloseable {
    private val active = AtomicBoolean(false)
    private val handles = mutableListOf<RecorderHandle>()
    private val readers = mutableListOf<Thread>()

    fun start(manager: AudioManager, projection: MediaProjection?) {
        val (playback, mic) = Recorders.start(manager, projection)
        playback?.let(handles::add)
        handles.add(mic)
        active.set(true)
        runtime.running(id, Recorders.info(mic))
        playback?.let { launch(0, it.record) }
        launch(1, mic.record)
    }

    private fun launch(stream: Int, record: AudioRecord) {
        val thread =
            Thread(
                {
                    val rate = record.sampleRate
                    val channels = record.channelCount
                    val raw = ShortArray(rate / 100 * channels)
                    val packer = BlockPacker(stream, rate, channels)
                    val timestamp = AudioTimestamp()
                    val clock = TimestampTracker(rate)
                    var firstFrame = 0L
                    var silenceStarted: Long? = null
                    try {
                        Process.setThreadPriority(Process.THREAD_PRIORITY_URGENT_AUDIO)
                        while (active.get()) {
                            var used = 0
                            while (used < raw.size && active.get()) {
                                val read =
                                    record.read(
                                        raw,
                                        used,
                                        raw.size - used,
                                        AudioRecord.READ_BLOCKING,
                                    )
                                if (read < 0)
                                    throw CaptureFailure(
                                        if (stream == 0) "capture_error" else "mic_error",
                                    )
                                if (read == 0) {
                                    Thread.sleep(1)
                                    continue
                                }
                                used += read
                            }
                            if (!active.get()) break
                            if (
                                record.getTimestamp(timestamp, AudioTimestamp.TIMEBASE_MONOTONIC) ==
                                    AudioRecord.SUCCESS
                            )
                                clock.update(timestamp.framePosition, timestamp.nanoTime)
                            tap?.block(stream, rate, channels, raw)
                            runtime.block(id, packer.pack(raw, firstFrame, clock.at(firstFrame)))
                            firstFrame += rate / 100
                            if (stream == 1) {
                                if (raw.all { it == 0.toShort() }) {
                                    if (silenceStarted == null)
                                        silenceStarted = SystemClock.elapsedRealtime()
                                    runtime.silenced(
                                        SystemClock.elapsedRealtime() - silenceStarted >= 2000,
                                    )
                                } else {
                                    silenceStarted = null
                                    runtime.silenced(false)
                                }
                            }
                        }
                    } catch (_: SecurityException) {
                        if (active.get()) error("permission")
                    } catch (_: RuntimeException) {
                        if (active.get()) error(if (stream == 0) "capture_error" else "mic_error")
                    } catch (_: InterruptedException) {
                        Thread.currentThread().interrupt()
                    }
                },
                "Veyl-audio-$stream",
            )
        readers.add(thread)
        thread.start()
    }

    override fun close() {
        active.set(false)
        // Stop normally unblocks reads; bound the join in case a vendor recorder stalls.
        handles.forEach { runCatching { it.record.stop() } }
        try {
            readers.forEach { it.join(1500) }
        } finally {
            handles.forEach { runCatching { it.close() } }
            tap?.close()
            handles.clear()
            readers.clear()
        }
    }
}
