package io.github.jacobbalane.veyl.capture

import android.content.Context
import android.media.*
import android.os.Handler
import android.os.Looper
import android.os.Process
import io.github.jacobbalane.veyl.core.audio.TimestampTracker
import io.github.jacobbalane.veyl.core.testsound.BrightLoop
import java.util.concurrent.atomic.AtomicBoolean

class TestSoundPlayer(
    context: Context,
    val loop: BrightLoop,
    private val reference: (ShortArray, Long, Long) -> Unit,
    private val error: (String) -> Unit,
) : AutoCloseable {
    private val active = AtomicBoolean(false)
    private val closed = AtomicBoolean(false)
    private val manager = context.getSystemService(AudioManager::class.java)
    private val attributes =
        AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA)
            .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
            .build()
    private val track =
        AudioTrack.Builder()
            .setAudioAttributes(attributes)
            .setAudioFormat(
                AudioFormat.Builder()
                    .setSampleRate(loop.rate)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                    .build(),
            )
            .setBufferSizeInBytes(
                maxOf(
                    AudioTrack.getMinBufferSize(
                        loop.rate,
                        AudioFormat.CHANNEL_OUT_STEREO,
                        AudioFormat.ENCODING_PCM_16BIT,
                    ),
                    loop.rate * 4 / 10,
                ),
            )
            .setTransferMode(AudioTrack.MODE_STREAM)
            .build()
    private var writer: Thread? = null
    private val focus =
        AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
            .setAudioAttributes(attributes)
            .setOnAudioFocusChangeListener(
                { change ->
                    if (!closed.get())
                        when (change) {
                            AudioManager.AUDIOFOCUS_LOSS,
                            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> error("capture_error")
                            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK ->
                                runCatching { track.setVolume(0.2f) }
                            AudioManager.AUDIOFOCUS_GAIN -> runCatching { track.setVolume(1f) }
                        }
                },
                Handler(Looper.getMainLooper()),
            )
            .build()
    val presentationFrame: Long
        get() = track.playbackHeadPosition.toLong() and 0xffffffffL

    fun start() {
        check(track.state == AudioTrack.STATE_INITIALIZED)
        if (manager.requestAudioFocus(focus) != AudioManager.AUDIOFOCUS_REQUEST_GRANTED)
            throw CaptureFailure("capture_error")
        active.set(true)
        track.play()
        writer =
            Thread(
                    {
                        try {
                            Process.setThreadPriority(Process.THREAD_PRIORITY_URGENT_AUDIO)
                            var firstFrame = 0L
                            val clock = TimestampTracker(loop.rate)
                            val timestamp = AudioTimestamp()
                            while (active.get()) {
                                val pcm = loop.pcm(firstFrame, loop.rate / 100)
                                var written = 0
                                while (written < pcm.size && active.get()) {
                                    val count =
                                        track.write(
                                            pcm,
                                            written,
                                            pcm.size - written,
                                            AudioTrack.WRITE_BLOCKING,
                                        )
                                    if (count < 0) throw CaptureFailure("capture_error")
                                    if (count == 0) Thread.sleep(1) else written += count
                                }
                                if (!active.get()) break
                                if (track.getTimestamp(timestamp))
                                    clock.update(timestamp.framePosition, timestamp.nanoTime)
                                reference(pcm, firstFrame, clock.at(firstFrame))
                                firstFrame += loop.rate / 100
                            }
                        } catch (_: RuntimeException) {
                            if (active.get()) error("capture_error")
                        } catch (_: InterruptedException) {
                            Thread.currentThread().interrupt()
                        }
                    },
                    "Veyl-test-sound",
                )
                .also { it.start() }
    }

    override fun close() {
        if (!closed.compareAndSet(false, true)) return
        active.set(false)
        runCatching {
            track.pause()
            track.flush()
            track.stop()
        }
        try {
            writer?.join(1500)
        } finally {
            track.release()
            manager.abandonAudioFocusRequest(focus)
        }
    }
}
