package io.github.jacobbalane.veyl.core.audio

import kotlin.math.*
import kotlin.random.Random
import kotlin.test.*
import org.junit.Test

class FftSoundCheckTest {
    @Test
    fun fftMatchesAToneAndRoundTripsWithParsevalScaling() {
        val real =
            DoubleArray(2048) { sin(2 * PI * 37 * it / 2048) + 0.2 * cos(2 * PI * 101 * it / 2048) }
        val original = real.copyOf()
        val imag = DoubleArray(real.size)
        Fft.transform(real, imag)
        assertTrue(abs(imag[37] + 1024) < 1e-8)
        assertTrue(abs(real[101] - 204.8) < 1e-8)
        assertTrue(
            abs(
                original.sumOf { it * it } -
                    real.indices.sumOf { real[it] * real[it] + imag[it] * imag[it] } / real.size,
            ) < 1e-7,
        )
        Fft.transform(real, imag, inverse = true)
        assertTrue(
            real.indices.all { abs(real[it] - original[it]) < 1e-10 && abs(imag[it]) < 1e-10 },
        )
        assertFails { Fft.transform(DoubleArray(3), DoubleArray(3)) }
    }

    @Test
    fun soundCheckRecoversPositiveAndNegativeDelayAndRejectsIndependentSound() {
        for (rate in listOf(44100, 48000)) {
            val random = Random(7)
            val reference = FloatArray(rate * 3 / 2) { random.nextFloat() * 0.5f - 0.25f }
            for (lag in listOf(-173, 389)) {
                val microphone =
                    FloatArray(reference.size) {
                        (reference.getOrElse(it - lag) { 0f } * 0.6f) +
                            (random.nextFloat() - 0.5f) * 0.01f
                    }
                val result = SoundCheck.analyze(reference, microphone, rate)
                assertEquals(lag, result.lagFrames)
                assertTrue(result.meanCoherence > 0.99)
                assertTrue(result.score > 0.99)
            }
            val unrelated = FloatArray(reference.size) { random.nextFloat() * 0.5f - 0.25f }
            val noisy =
                FloatArray(reference.size) { reference[it] + (random.nextFloat() - 0.5f) * 0.3f }
            val expected = 0.5 * 0.5 / (0.5 * 0.5 + 0.3 * 0.3)
            assertTrue(
                abs(SoundCheck.analyze(reference, noisy, rate).meanCoherence - expected) < 0.04,
            )
            assertTrue(SoundCheck.analyze(reference, unrelated, rate).score < 0.1)
            assertEquals(0.0, SoundCheck.analyze(reference, FloatArray(reference.size), rate).score)
            val dull = reference.copyOf()
            val low = Biquad(rate, 3000, sqrt(0.5), false)
            for (i in dull.indices) dull[i] =
                low.step(dull[i]) * 0.001f + (random.nextFloat() - 0.5f) * 0.01f
            assertTrue(SoundCheck.analyze(reference, dull, rate).meanCoherence < 0.2)
        }
    }

    @Test
    fun tiesPreferFewerEnabledEffectsThenStereo() {
        val result = SoundCheckResult(0, 0.8, 1.0, 0.8)
        val mono = SoundCheckCandidate("mic", 1, 0, result)
        val stereo = SoundCheckCandidate("voice_recognition", 2, 0, result)
        val processed = SoundCheckCandidate("unprocessed", 2, 1, result)
        assertEquals(stereo, SoundCheck.best(listOf(processed, mono, stereo)))
        assertEquals(mono, SoundCheck.best(listOf(processed, mono)))
        assertNull(SoundCheck.best(emptyList()))
    }
}
