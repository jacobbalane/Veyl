package io.github.jacobbalane.veyl.capture

import androidx.media3.common.C
import androidx.media3.common.audio.AudioProcessor
import androidx.media3.common.audio.SonicAudioProcessor
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.audio.TeeAudioProcessor
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.sin
import org.junit.Assert.*
import org.junit.Test

@androidx.annotation.OptIn(UnstableApi::class)
class PlayerProcessorsTest {
    @Test
    fun theRealPlayerProcessorsTurn44100MonoInto48000StereoAndTeeTheExactOutput() {
        val mix = VeylPlayer.StereoProcessor()
        val resample = SonicAudioProcessor().apply { setOutputSampleRateHz(48000) }
        var teed = ByteArray(0)
        var format: Triple<Int, Int, Int>? = null
        val tee =
            TeeAudioProcessor(
                object : TeeAudioProcessor.AudioBufferSink {
                    override fun flush(sampleRateHz: Int, channelCount: Int, encoding: Int) {
                        format = Triple(sampleRateHz, channelCount, encoding)
                    }

                    override fun handleBuffer(buffer: ByteBuffer) {
                        val bytes = ByteArray(buffer.remaining())
                        buffer.get(bytes)
                        teed += bytes
                    }
                },
            )
        val original = AudioProcessor.AudioFormat(44100, 1, C.ENCODING_PCM_16BIT)
        val stereo = mix.configure(original)
        val converted = resample.configure(stereo)
        tee.configure(converted)
        val metadata = AudioProcessor.StreamMetadata.DEFAULT
        mix.flush(metadata)
        resample.flush(metadata)
        tee.flush(metadata)
        val input = ByteBuffer.allocateDirect(44100 * 2).order(ByteOrder.nativeOrder())
        repeat(44100) {
            input.putShort((sin(2 * PI * 12000 * it / 44100) * 12000).toInt().toShort())
        }
        input.flip()
        mix.queueInput(input)
        resample.queueInput(mix.output)
        resample.queueEndOfStream()
        var output = ByteArray(0)
        while (!resample.isEnded) {
            val chunk = resample.output
            if (!chunk.hasRemaining()) continue
            tee.queueInput(chunk)
            val played = tee.output
            val bytes = ByteArray(played.remaining())
            played.get(bytes)
            output += bytes
        }
        assertEquals(Triple(48000, 2, C.ENCODING_PCM_16BIT), format)
        assertTrue(abs(output.size / 4 - 48000) <= 2)
        assertArrayEquals(output, teed)
        val pcm = ByteBuffer.wrap(output).order(ByteOrder.nativeOrder())
        var previous = 0
        var crossings = 0
        var peak = 0
        while (pcm.hasRemaining()) {
            val sample = pcm.short.toInt()
            assertEquals(sample.toShort(), pcm.short)
            if (previous <= 0 && sample > 0) crossings++
            previous = sample
            peak = maxOf(peak, abs(sample))
        }
        assertTrue(peak > 3000)
        assertTrue(abs(crossings - 12000) < 50)
        mix.reset()
        resample.reset()
        tee.reset()
    }
}
