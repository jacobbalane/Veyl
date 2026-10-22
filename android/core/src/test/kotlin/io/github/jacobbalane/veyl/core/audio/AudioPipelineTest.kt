package io.github.jacobbalane.veyl.core.audio

import kotlin.math.*
import kotlin.test.*
import org.junit.Test

class AudioPipelineTest {
    private fun response(rate: Int, hz: Double): Double {
        val samples = FloatArray(rate) { sin(2 * PI * hz * it / rate).toFloat() }
        PrivacyFilter(rate).process(samples)
        val power = samples.drop(rate / 2).sumOf { it.toDouble() * it } / (rate / 2)
        return 10 * log10(power / 0.5)
    }

    @Test
    fun privacyResponseMatchesTheRustSpecificationAtBothRates() {
        for (rate in listOf(44100, 48000)) {
            assertTrue(abs(response(rate, 8000.0) + 3) < 0.3)
            assertTrue(response(rate, 4000.0) <= -40)
            assertTrue(response(rate, 2000.0) <= -70)
            for (hz in listOf(11000.0, 16000.0, 20000.0)) assertTrue(abs(response(rate, hz)) < 0.3)
        }
    }

    @Test
    fun filterIsStableKeepsChannelsSeparateAndResets() {
        val filter = PrivacyFilter(48000, 8000, 2)
        val impulse = FloatArray(96000)
        impulse[0] = 1f
        filter.process(impulse)
        assertTrue(impulse.all { it.isFinite() })
        assertTrue(impulse.indices.filter { it % 2 == 1 }.all { impulse[it] == 0f })
        filter.reset()
        val again = FloatArray(96000)
        again[0] = 1f
        filter.process(again)
        assertTrue(impulse.contentEquals(again))
        assertFails { PrivacyFilter(16000) }
        assertFails { PrivacyFilter(48000, 1000) }
        assertFails { PrivacyFilter(48000, channels = 0) }
    }

    @Test
    fun statsAreMeasuredBeforeFilteringAndFrameCountersArePreserved() {
        assertEquals(-120f, BlockStats.measure(ShortArray(10)).rmsDbfs)
        val raw = ShortArray(480) { Short.MIN_VALUE }
        val packer = BlockPacker(1, 48000, 1)
        val block = packer.pack(raw, 1000, -123)
        assertEquals(0f, block.preRmsDbfs)
        assertEquals(1f, block.prePeak)
        assertEquals(2, block.flags and 2)
        assertEquals(1000L, block.firstFrame)
        assertEquals(-123L, block.captureNs)
        assertEquals(0L, block.seq)
        assertEquals(1L, packer.pack(raw, 1480, 5).seq)
        assertTrue(block.samples.any { it != Short.MIN_VALUE })
        assertEquals(
            1000L,
            io.github.jacobbalane.veyl.core.link.AudioBlock.decode(block.encode()).firstFrame,
        )
    }

    @Test
    fun timestampsExtrapolateFromTheLastValidRecorderClock() {
        val tracker = TimestampTracker(48000)
        assertEquals(Long.MIN_VALUE, tracker.at(0))
        tracker.update(480, 1_000_000_000)
        assertEquals(990_000_000L, tracker.at(0))
        assertEquals(1_010_000_000L, tracker.at(960))
    }
}
