package io.github.jacobbalane.veyl.capture

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.Process
import androidx.core.net.toUri
import androidx.media3.common.*
import androidx.media3.common.audio.*
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.*
import androidx.media3.exoplayer.audio.*
import io.github.jacobbalane.veyl.core.audio.SpscRing
import io.github.jacobbalane.veyl.core.audio.StereoPcm
import io.github.jacobbalane.veyl.runtime.PlayerSnapshot
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

@androidx.annotation.OptIn(UnstableApi::class)
class VeylPlayer(
    private val context: Context,
    private val rate: Int,
    private val items: List<String>,
    private val reference: (ShortArray, Long, Long, Boolean) -> Unit,
    private val changed: (PlayerSnapshot) -> Unit,
    private val error: (String) -> Unit,
) : AutoCloseable {
    private data class Chunk(
        val pcm: ShortArray,
        val firstFrame: Long,
        val time: Long,
        val epoch: Long,
    )

    private val ring = SpscRing<Chunk>(32)
    private val active = AtomicBoolean(false)
    private val main = Handler(Looper.getMainLooper())
    private var consumer: Thread? = null
    private var player: ExoPlayer? = null // Accessed only on the main looper.
    private var nextFrame = 0L // Accessed only by the player's audio sink.
    private val epoch = AtomicLong()
    private val sink =
        object : TeeAudioProcessor.AudioBufferSink {
            override fun flush(sampleRateHz: Int, channelCount: Int, encoding: Int) {
                epoch.incrementAndGet()
                if (sampleRateHz != rate || channelCount != 2 || encoding != C.ENCODING_PCM_16BIT)
                    error("capture_error")
            }

            override fun handleBuffer(buffer: ByteBuffer) {
                val bytes = buffer.duplicate().order(ByteOrder.LITTLE_ENDIAN)
                if (bytes.remaining() % 4 != 0) {
                    error("capture_error")
                    return
                }
                val pcm = ShortArray(bytes.remaining() / 2) { bytes.short }
                ring.offer(Chunk(pcm, nextFrame, System.nanoTime(), epoch.get()))
                nextFrame += pcm.size / 2
            }
        }

    fun start() {
        require(items.isNotEmpty() && items.all { it.toUri().scheme == "content" })
        active.set(true)
        consumer =
            Thread(
                    {
                        try {
                            Process.setThreadPriority(Process.THREAD_PRIORITY_URGENT_AUDIO)
                            val block = ShortArray(rate / 100 * 2)
                            var used = 0
                            var first = 0L
                            var stamp = 0L
                            var expected = 0L
                            var gap = false
                            var seenEpoch = epoch.get()
                            while (active.get()) {
                                val currentEpoch = epoch.get()
                                if (currentEpoch != seenEpoch) {
                                    used = 0
                                    gap = true
                                    seenEpoch = currentEpoch
                                }
                                val chunk = ring.poll()
                                if (chunk == null) {
                                    Thread.sleep(1)
                                    continue
                                }
                                if (chunk.epoch != seenEpoch) continue
                                if (chunk.firstFrame != expected) {
                                    used = 0
                                    gap = true
                                }
                                var offset = 0
                                while (offset < chunk.pcm.size && active.get()) {
                                    if (used == 0) {
                                        first = chunk.firstFrame + offset / 2
                                        stamp = chunk.time + offset / 2 * 1_000_000_000L / rate
                                    }
                                    val count = minOf(block.size - used, chunk.pcm.size - offset)
                                    chunk.pcm.copyInto(block, used, offset, offset + count)
                                    used += count
                                    offset += count
                                    if (used == block.size) {
                                        reference(block, first, stamp, gap)
                                        used = 0
                                        gap = false
                                    }
                                }
                                expected = chunk.firstFrame + chunk.pcm.size / 2
                            }
                        } catch (_: RuntimeException) {
                            if (active.get()) error("capture_error")
                        } catch (_: InterruptedException) {
                            Thread.currentThread().interrupt()
                        }
                    },
                    "Veyl-player-reference",
                )
                .also { it.start() }
        val ready = CountDownLatch(1)
        val failed = AtomicBoolean(false)
        main.post {
            try {
                if (!active.get()) return@post
                val renderers =
                    object : DefaultRenderersFactory(context) {
                        override fun buildAudioSink(
                            context: Context,
                            enableFloatOutput: Boolean,
                            enableAudioOutputPlaybackParams: Boolean,
                        ): AudioSink =
                            DefaultAudioSink.Builder(context)
                                .setEnableFloatOutput(false)
                                .setEnableAudioOutputPlaybackParameters(false)
                                .setAudioProcessors(
                                    arrayOf(
                                        StereoProcessor(),
                                        SonicAudioProcessor().apply { setOutputSampleRateHz(rate) },
                                        TeeAudioProcessor(sink),
                                    ),
                                )
                                .build()
                    }
                val p = ExoPlayer.Builder(context, renderers).build()
                player = p
                p.setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(C.USAGE_MEDIA)
                        .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
                        .build(),
                    true,
                )
                p.addListener(
                    object : Player.Listener {
                        override fun onEvents(player: Player, events: Player.Events) {
                            changed(
                                PlayerSnapshot(
                                    player.isPlaying,
                                    player.currentMediaItemIndex,
                                    player.mediaItemCount,
                                    player.shuffleModeEnabled,
                                ),
                            )
                        }

                        override fun onPlayerError(failure: PlaybackException) {
                            error("capture_error")
                        }
                    },
                )
                p.setMediaItems(items.map { MediaItem.fromUri(it.toUri()) })
                p.repeatMode = Player.REPEAT_MODE_ALL
                p.prepare()
                p.play()
            } catch (_: RuntimeException) {
                failed.set(true)
                error("capture_error")
            } finally {
                ready.countDown()
            }
        }
        if (!ready.await(5, TimeUnit.SECONDS) || failed.get()) throw CaptureFailure("capture_error")
    }

    fun command(action: String) {
        main.post {
            player?.let { p ->
                when (action) {
                    "play" -> p.play()
                    "pause" -> p.pause()
                    "previous" -> p.seekToPreviousMediaItem()
                    "next" -> p.seekToNextMediaItem()
                    "shuffle" -> p.shuffleModeEnabled = !p.shuffleModeEnabled
                }
            }
        }
    }

    override fun close() {
        active.set(false)
        val released = CountDownLatch(1)
        main.post {
            try {
                player?.release()
                player = null
                changed(PlayerSnapshot())
            } finally {
                released.countDown()
            }
        }
        try {
            consumer?.join(1500)
        } finally {
            released.await(3, TimeUnit.SECONDS)
        }
    }

    internal class StereoProcessor : BaseAudioProcessor() {
        override fun onConfigure(
            inputAudioFormat: AudioProcessor.AudioFormat,
        ): AudioProcessor.AudioFormat {
            if (
                inputAudioFormat.encoding != C.ENCODING_PCM_16BIT ||
                    inputAudioFormat.channelCount !in 1..8
            )
                throw AudioProcessor.UnhandledAudioFormatException(inputAudioFormat)
            return AudioProcessor.AudioFormat(inputAudioFormat.sampleRate, 2, C.ENCODING_PCM_16BIT)
        }

        override fun queueInput(inputBuffer: ByteBuffer) {
            val pcm = ShortArray(inputBuffer.remaining() / 2) { inputBuffer.short }
            val stereo = StereoPcm.mix(pcm, inputAudioFormat.channelCount)
            val output = replaceOutputBuffer(stereo.size * 2).order(ByteOrder.nativeOrder())
            stereo.forEach { output.putShort(it) }
            output.flip()
        }
    }
}
