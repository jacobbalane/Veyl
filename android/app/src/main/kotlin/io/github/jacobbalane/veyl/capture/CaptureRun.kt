package io.github.jacobbalane.veyl.capture

import android.content.Context
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
    private val cutoff: Int = 8000,
    private val preferredMic: MicChoice? = null,
) : AutoCloseable {
    private val active = AtomicBoolean(false)
    private val handles = mutableListOf<RecorderHandle>()
    private val readers = mutableListOf<Thread>()
    private var testSound: TestSoundPlayer? = null
    private var player: VeylPlayer? = null

    fun start(
        context: Context,
        manager: AudioManager,
        projection: MediaProjection?,
        source: String = "capture",
        items: List<String> = emptyList(),
    ) {
        if (source == "capture" && projection == null) throw CaptureFailure("capture_error")
        val (playback, mic) =
            Recorders.start(manager, if (source == "capture") projection else null, preferredMic)
        playback?.let(handles::add)
        handles.add(mic)
        active.set(true)
        playback?.let { launch(0, it.record) }
        launch(1, mic.record)
        val rate = mic.record.sampleRate
        val packer = BlockPacker(0, rate, 2, cutoff)
        fun reference(pcm: ShortArray, first: Long, time: Long, flags: Int) {
            if (!active.get()) return
            tap?.block(0, rate, 2, pcm)
            runtime.block(id, packer.pack(pcm, first, time, flags))
        }
        when (source) {
            "test_sound" -> {
                val sound =
                    TestSoundPlayer(
                        context,
                        runtime.testLoop(rate),
                        { pcm, first, time -> reference(pcm, first, time, 12) },
                        error,
                    )
                testSound = sound
                sound.start()
            }
            "player" -> {
                val music =
                    VeylPlayer(
                        context,
                        rate,
                        items,
                        { pcm, first, time, gap ->
                            reference(pcm, first, time, 8 or if (gap) 1 else 0)
                        },
                        runtime::updatePlayer,
                        error,
                    )
                player = music
                music.start()
            }
        }
        runtime.running(id, Recorders.info(mic))
    }

    fun command(action: String) {
        player?.command(action)
    }

    private fun launch(stream: Int, record: AudioRecord) {
        val thread =
            Thread(
                {
                    val rate = record.sampleRate
                    val channels = record.channelCount
                    val raw = ShortArray(rate / 100 * channels)
                    val packer = BlockPacker(stream, rate, channels, cutoff)
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
                                        id,
                                        SystemClock.elapsedRealtime() - silenceStarted >= 2000,
                                    )
                                } else {
                                    silenceStarted = null
                                    runtime.silenced(id, false)
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
        runCatching { testSound?.close() }
        testSound = null
        runCatching { player?.close() }
        player = null
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
