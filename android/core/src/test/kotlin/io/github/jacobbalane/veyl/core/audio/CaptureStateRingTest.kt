package io.github.jacobbalane.veyl.core.audio

import kotlin.test.*
import org.junit.Test

class CaptureStateRingTest {
    @Test
    fun staleAndLateEventsCannotReviveCaptureOrHideAStopReason() {
        val start = CaptureReducer.start(CaptureStatus(), 1, "capture")
        val running = CaptureReducer.running(start, 1)
        assertEquals("starting", start.state)
        assertEquals("running", running.state)
        assertTrue(CaptureReducer.silenced(running, 1, true).silenced)
        val stopped = CaptureReducer.stopped(running, 1, "screen_locked")
        assertEquals(stopped, CaptureReducer.running(stopped, 1))
        assertEquals(stopped, CaptureReducer.silenced(stopped, 1, true))
        assertEquals(stopped, CaptureReducer.stopped(stopped, 1, "user"))
        val next = CaptureReducer.start(stopped, 2, "test_sound")
        assertEquals(next, CaptureReducer.stopped(next, 1, "mic_error"))
        assertEquals(next, CaptureReducer.running(next, 1))
        assertEquals("error", CaptureReducer.stopped(next, 2, "permission").state)
        assertFails { CaptureReducer.start(next, 2, "capture") }
    }

    @Test
    fun pcmRingDropsNewInputOnOverflowAndRetainsOrderAcrossWraparound() {
        val ring = SpscRing<Int>(2)
        assertNull(ring.poll())
        assertTrue(ring.offer(1))
        assertTrue(ring.offer(2))
        assertFalse(ring.offer(3))
        assertEquals(1, ring.poll())
        assertTrue(ring.offer(4))
        assertEquals(2, ring.poll())
        assertEquals(4, ring.poll())
        assertNull(ring.poll())
    }

    @Test
    fun pcmRingPublishesAllSamplesBetweenRealProducerAndConsumerThreads() {
        val ring = SpscRing<Int>(17)
        val result = ArrayList<Int>()
        val producer = Thread { repeat(100000) { while (!ring.offer(it)) Thread.yield() } }
        val consumer = Thread {
            while (result.size < 100000) {
                val value = ring.poll()
                if (value != null) result.add(value) else Thread.yield()
            }
        }
        producer.isDaemon = true
        consumer.isDaemon = true
        producer.start()
        consumer.start()
        producer.join(5000)
        consumer.join(5000)
        assertFalse(producer.isAlive)
        assertFalse(consumer.isAlive)
        assertEquals((0 until 100000).toList(), result)
    }

    @Test
    fun stereoConversionDuplicatesMonoAndKeepsAllSurroundChannelsWithoutClipping() {
        assertContentEquals(shortArrayOf(7, 7, -8, -8), StereoPcm.mix(shortArrayOf(7, -8), 1))
        assertContentEquals(shortArrayOf(7, -8), StereoPcm.mix(shortArrayOf(7, -8), 2))
        assertContentEquals(
            shortArrayOf(100, 0),
            StereoPcm.mix(shortArrayOf(300, 0, 0, 0, 0, 0), 6),
        )
        assertContentEquals(
            shortArrayOf(100, 100),
            StereoPcm.mix(shortArrayOf(0, 0, 600, 0, 0, 0), 6),
        )
        assertTrue(
            StereoPcm.mix(ShortArray(8) { Short.MAX_VALUE }, 8).all { it == Short.MAX_VALUE },
        )
    }
}
