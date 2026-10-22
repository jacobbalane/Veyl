package io.github.jacobbalane.veyl.capture

import android.content.Context
import android.media.AudioManager
import android.media.AudioRecord
import android.os.SystemClock
import io.github.jacobbalane.veyl.core.audio.SoundCheck
import io.github.jacobbalane.veyl.core.audio.SoundCheckCandidate
import io.github.jacobbalane.veyl.core.testsound.BrightLoop
import io.github.jacobbalane.veyl.runtime.CheckedMic
import java.util.concurrent.atomic.AtomicBoolean

/** Temporary raw samples remain in memory and never enter a link or recording writer. */
class SoundCheckRunner(
    private val context: Context,
    private val cancelled: () -> Boolean,
    private val loop: (Int) -> BrightLoop,
) {
    fun run(progress: (Int, Int) -> Unit): CheckedMic? {
        val manager = context.getSystemService(AudioManager::class.java)
        val choices = Recorders.choices(manager)
        val results = mutableListOf<Pair<SoundCheckCandidate, Int>>()
        var player: TestSoundPlayer? = null
        val playerFailed = AtomicBoolean(false)
        try {
            for ((index, choice) in choices.withIndex()) {
                if (cancelled()) return null
                progress(index + 1, choices.size)
                var mic: RecorderHandle? = null
                try {
                    for (rate in listOf(48000, 44100)) {
                        if (cancelled()) return null
                        try {
                            mic = Recorders.mic(rate, choice)
                            mic.record.startRecording()
                            check(mic.record.recordingState == AudioRecord.RECORDSTATE_RECORDING)
                            break
                        } catch (failure: SecurityException) {
                            mic?.close()
                            mic = null
                            throw failure
                        } catch (_: RuntimeException) {
                            mic?.close()
                            mic = null
                        }
                    }
                    val record = mic ?: continue
                    val rate = record.record.sampleRate
                    if (player?.loop?.rate != rate) {
                        player?.close()
                        player = null
                        playerFailed.set(false)
                        val next =
                            TestSoundPlayer(
                                context,
                                loop(rate),
                                { _, _, _ -> },
                                { playerFailed.set(true) },
                            )
                        player = next
                        next.start()
                    }
                    // Discard startup buffers so this sample and the presentation-head reference
                    // begin together.
                    val discard = ShortArray(rate / 10 * choice.channels)
                    repeat(3) {
                        record.record.read(discard, 0, discard.size, AudioRecord.READ_NON_BLOCKING)
                    }
                    val firstFrame = player.presentationFrame
                    val raw = ShortArray(rate * 3 / 2 * choice.channels)
                    var used = 0
                    val until = SystemClock.elapsedRealtime() + 4000
                    while (
                        used < raw.size &&
                            !cancelled() &&
                            !playerFailed.get() &&
                            SystemClock.elapsedRealtime() < until
                    ) {
                        val read =
                            record.record.read(
                                raw,
                                used,
                                raw.size - used,
                                AudioRecord.READ_NON_BLOCKING,
                            )
                        if (read < 0) throw CaptureFailure("mic_error")
                        if (read == 0) Thread.sleep(5) else used += read
                    }
                    if (cancelled()) return null
                    if (used != raw.size || playerFailed.get()) continue
                    val ref = player.loop.pcm(firstFrame, rate * 3 / 2)
                    val refMono =
                        FloatArray(ref.size / 2) {
                            (ref[it * 2].toInt() + ref[it * 2 + 1]) / 65536f
                        }
                    val micMono =
                        FloatArray(raw.size / choice.channels) { frame ->
                            (0 until choice.channels).sumOf {
                                raw[frame * choice.channels + it].toInt()
                            } / (32768f * choice.channels)
                        }
                    results.add(
                        SoundCheckCandidate(
                            choice.name,
                            choice.channels,
                            record.effectsOn.size,
                            SoundCheck.analyze(refMono, micMono, rate),
                        ) to rate,
                    )
                } catch (failure: SecurityException) {
                    throw failure
                } catch (_: RuntimeException) {
                    /* Try the remaining configurations if this recorder fails. */
                } finally {
                    mic?.close()
                }
            }
        } finally {
            player?.close()
        }
        val best = SoundCheck.best(results.map { it.first }) ?: return null
        return CheckedMic(
            best.source,
            best.channels,
            results.first { it.first == best }.second,
            best.result.score,
            best.result.meanCoherence,
        )
    }
}
