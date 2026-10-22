package io.github.jacobbalane.veyl.core.audio

import io.github.jacobbalane.veyl.core.testsound.BrightLoop
import kotlin.math.*
import kotlin.test.*
import org.junit.Test

class BrightLoopTest {
    @Test
    fun deterministicStereoLoopHasSensingEnergyAndALimitedPeakAtBothRates() {
        for (rate in listOf(44100, 48000)) {
            val loop = BrightLoop(rate)
            val repeated = BrightLoop(rate)
            assertTrue(loop.samples.contentEquals(repeated.samples))
            assertEquals(rate * 16 * 2, loop.samples.size)
            assertTrue(loop.samples.all { it.isFinite() && abs(it) <= 10.0.pow(-1.0 / 20) + 1e-6 })
            assertTrue(
                loop.samples.indices.step(2).any { loop.samples[it] != loop.samples[it + 1] },
            )
            assertTrue(loop.pcm(0, 480).contentEquals(loop.pcm(rate * 16L, 480)))
            val acrossBoundary = loop.pcm(rate * 16L - 1, 2)
            assertEquals(loop.pcm(0, 1)[0], acrossBoundary[2])
            var total = 0.0
            var high = 0.0
            for (start in 0..rate * 16 - 4096 step 4096) for (ch in 0..1) {
                val real =
                    DoubleArray(4096) {
                        loop.samples[(start + it) * 2 + ch] * (0.5 - 0.5 * cos(2 * PI * it / 4096))
                    }
                val imaginary = DoubleArray(4096)
                Fft.transform(real, imaginary)
                for (bin in 1..2048) {
                    val power = real[bin] * real[bin] + imaginary[bin] * imaginary[bin]
                    total += power
                    if (bin.toDouble() * rate / 4096 in 8000.0..20000.0) high += power
                }
            }
            val ratio = 10 * log10(high / total)
            assertTrue(ratio in -20.0..-8.0, "$rate Hz sensing band: $ratio dB")
        }
        assertFails { BrightLoop(16000) }
        assertFails { BrightLoop(48000, 0) }
    }
}
