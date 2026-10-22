package io.github.jacobbalane.veyl.core.link

import kotlin.test.*
import org.junit.Test

class WriterPriorityTest {
    @Test
    fun controlAndPongOvertakeAnExistingAudioBacklogWithoutDroppingMessages() {
        val queue = WriterQueue()
        repeat(100) {
            queue.pushAudio(
                AudioBlock(
                    0,
                    1,
                    48000,
                    it.toLong(),
                    it * 480L,
                    0,
                    480,
                    0,
                    -30f,
                    0.1f,
                    ShortArray(480),
                ),
            )
        }
        queue.pushMessage(Frames.CONTROL, "{\"t\":\"request_start\"}".toByteArray())
        queue.pushMessage(Frames.PONG, PingPong.encodePong(1, 2))
        assertEquals(1000, queue.audioQueueMs)
        assertEquals(Frames.CONTROL, (queue.pop() as WriterQueue.Outbound.Message).type)
        assertEquals(Frames.PONG, (queue.pop() as WriterQueue.Outbound.Message).type)
        queue.pushMessage(Frames.CONTROL, "{\"t\":\"request_start\"}".toByteArray())
        queue.clearAudio()
        assertEquals(1, queue.size)
        assertEquals(0, queue.audioQueueMs)
        queue.clear()
        assertNull(queue.pop())
        repeat(256) { queue.pushMessage(Frames.PING, PingPong.encodePing(it.toLong())) }
        assertFails { queue.pushMessage(Frames.PING, PingPong.encodePing(300)) }
    }
}
