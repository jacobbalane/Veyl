package io.github.jacobbalane.veyl.capture

import android.annotation.SuppressLint
import android.media.*
import android.media.audiofx.*
import android.media.projection.MediaProjection
import android.os.Build
import io.github.jacobbalane.veyl.runtime.CaptureInfo
import io.github.jacobbalane.veyl.runtime.MicMetadata
import java.io.IOException
import java.util.concurrent.atomic.AtomicBoolean

class CaptureFailure(val reason: String) : RuntimeException(reason)

data class MicChoice(val source: Int, val channels: Int, val name: String)

class RecorderHandle(
    val record: AudioRecord,
    val choice: MicChoice?,
    val effectsOff: List<String>,
    val effectsOn: List<String>,
    private val effects: List<AudioEffect>,
) : AutoCloseable {
    private val closed = AtomicBoolean(false)

    override fun close() {
        if (!closed.compareAndSet(false, true)) return
        try {
            if (record.recordingState == AudioRecord.RECORDSTATE_RECORDING) record.stop()
        } catch (_: IllegalStateException) {} finally {
            effects.forEach { runCatching { it.release() } }
            record.release()
        }
    }
}

object Recorders {
    fun choices(manager: AudioManager): List<MicChoice> = buildList {
        if (manager.getProperty(AudioManager.PROPERTY_SUPPORT_AUDIO_SOURCE_UNPROCESSED) == "true") {
            add(MicChoice(MediaRecorder.AudioSource.UNPROCESSED, 2, "unprocessed"))
            add(MicChoice(MediaRecorder.AudioSource.UNPROCESSED, 1, "unprocessed"))
        }
        add(MicChoice(MediaRecorder.AudioSource.VOICE_RECOGNITION, 2, "voice_recognition"))
        add(MicChoice(MediaRecorder.AudioSource.VOICE_RECOGNITION, 1, "voice_recognition"))
        add(MicChoice(MediaRecorder.AudioSource.CAMCORDER, 2, "camcorder"))
        add(MicChoice(MediaRecorder.AudioSource.MIC, 2, "mic"))
        add(MicChoice(MediaRecorder.AudioSource.MIC, 1, "mic"))
    }

    private fun format(rate: Int, channels: Int): AudioFormat =
        AudioFormat.Builder()
            .setSampleRate(rate)
            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
            .setChannelMask(
                if (channels == 2) AudioFormat.CHANNEL_IN_STEREO else AudioFormat.CHANNEL_IN_MONO,
            )
            .build()

    private fun buffer(rate: Int, channels: Int): Int {
        val minimum =
            AudioRecord.getMinBufferSize(
                rate,
                if (channels == 2) AudioFormat.CHANNEL_IN_STEREO else AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_16BIT,
            )
        check(minimum > 0)
        return maxOf(minimum * 2, rate * channels * 2 / 5)
    }

    @SuppressLint("MissingPermission")
    fun mic(rate: Int, choice: MicChoice): RecorderHandle {
        val record =
            AudioRecord.Builder()
                .setAudioSource(choice.source)
                .setAudioFormat(format(rate, choice.channels))
                .setBufferSizeInBytes(buffer(rate, choice.channels))
                .build()
        if (record.state != AudioRecord.STATE_INITIALIZED) {
            record.release()
            throw CaptureFailure("mic_error")
        }
        val effects = mutableListOf<AudioEffect>()
        val off = mutableListOf<String>()
        val on = mutableListOf<String>()
        val factories: List<Pair<String, () -> AudioEffect?>> =
            listOf(
                "ns" to
                    {
                        if (NoiseSuppressor.isAvailable())
                            NoiseSuppressor.create(record.audioSessionId)
                        else null
                    },
                "agc" to
                    {
                        if (AutomaticGainControl.isAvailable())
                            AutomaticGainControl.create(record.audioSessionId)
                        else null
                    },
                "aec" to
                    {
                        if (AcousticEchoCanceler.isAvailable())
                            AcousticEchoCanceler.create(record.audioSessionId)
                        else null
                    },
            )
        for ((name, factory) in factories) {
            try {
                val effect = factory() ?: continue
                effects.add(effect)
                if (effect.enabled) {
                    effect.enabled = false
                    if (!effect.enabled) off.add(name)
                }
                if (effect.enabled) on.add(name)
            } catch (_: RuntimeException) {
                on.add(name)
            }
        }
        return RecorderHandle(record, choice, off, on, effects)
    }

    @SuppressLint("MissingPermission")
    fun playback(rate: Int, projection: MediaProjection): RecorderHandle {
        if (Build.VERSION.SDK_INT < 29) throw CaptureFailure("capture_error")
        val config =
            AudioPlaybackCaptureConfiguration.Builder(projection)
                .addMatchingUsage(AudioAttributes.USAGE_MEDIA)
                .addMatchingUsage(AudioAttributes.USAGE_GAME)
                .addMatchingUsage(AudioAttributes.USAGE_UNKNOWN)
                .build()
        val record =
            AudioRecord.Builder()
                .setAudioFormat(format(rate, 2))
                .setBufferSizeInBytes(buffer(rate, 2))
                .setAudioPlaybackCaptureConfig(config)
                .build()
        if (record.state != AudioRecord.STATE_INITIALIZED) {
            record.release()
            throw CaptureFailure("capture_error")
        }
        return RecorderHandle(record, null, emptyList(), emptyList(), emptyList())
    }

    fun start(
        manager: AudioManager,
        projection: MediaProjection?,
        preferred: MicChoice? = null,
    ): Pair<RecorderHandle?, RecorderHandle> {
        for ((rate, choice) in order(choices(manager), preferred)) {
            var mic: RecorderHandle? = null
            var playback: RecorderHandle? = null
            try {
                mic = Recorders.mic(rate, choice)
                playback = projection?.let { Recorders.playback(rate, it) }
                mic.record.startRecording()
                playback?.record?.startRecording()
                check(mic.record.recordingState == AudioRecord.RECORDSTATE_RECORDING)
                check(
                    playback == null ||
                        playback.record.recordingState == AudioRecord.RECORDSTATE_RECORDING,
                )
                return playback to mic
            } catch (failure: SecurityException) {
                playback?.close()
                mic?.close()
                throw failure
            } catch (_: RuntimeException) {
                playback?.close()
                mic?.close()
            }
        }
        throw CaptureFailure(if (projection == null) "mic_error" else "capture_error")
    }

    fun order(available: List<MicChoice>, preferred: MicChoice?): List<Pair<Int, MicChoice>> =
        (listOfNotNull(preferred).flatMap { choice -> listOf(48000 to choice, 44100 to choice) } +
                listOf(48000, 44100).flatMap { rate -> available.map { rate to it } })
            .distinct()

    fun info(mic: RecorderHandle): CaptureInfo =
        CaptureInfo(
            mic.record.sampleRate,
            mic.choice!!.name,
            mic.record.channelCount,
            mic.effectsOff,
            mic.effectsOn,
            microphones(mic.record),
        )

    private fun microphones(record: AudioRecord): List<MicMetadata> {
        if (Build.VERSION.SDK_INT < 28) return emptyList()
        return try {
            record.activeMicrophones.mapIndexed { index, mic ->
                val position = mic.position
                val values = listOf(position.x, position.y, position.z)
                val unknown = MicrophoneInfo.POSITION_UNKNOWN
                MicMetadata(
                    index,
                    if (mic.type == AudioDeviceInfo.TYPE_BUILTIN_MIC) "builtin" else "external",
                    values.takeUnless {
                        it.any { value -> !value.isFinite() } ||
                            (position.x == unknown.x &&
                                position.y == unknown.y &&
                                position.z == unknown.z)
                    },
                    mic.channelMapping.map { it.first },
                )
            }
        } catch (_: IOException) {
            emptyList()
        } catch (_: RuntimeException) {
            emptyList()
        }
    }
}
