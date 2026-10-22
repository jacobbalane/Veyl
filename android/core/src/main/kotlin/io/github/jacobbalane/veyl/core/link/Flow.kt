package io.github.jacobbalane.veyl.core.link

/** Audio is the only traffic that may be dropped. The next queued block marks the gap. */
class WriterQueue {
    sealed interface Outbound {
        class Audio(val block: AudioBlock) : Outbound

        class Message(val type: Int, val payload: ByteArray) : Outbound
    }

    private val queue = ArrayDeque<Outbound>()
    val size: Int
        get() = queue.size

    fun pushAudio(block: AudioBlock): Boolean {
        block.encode()
        val dropped = queue.count { it is Outbound.Audio && it.block.stream == block.stream } >= 100
        if (dropped) {
            queue.removeAt(
                queue.indexOfFirst { it is Outbound.Audio && it.block.stream == block.stream },
            )
            val next =
                queue.firstOrNull { it is Outbound.Audio && it.block.stream == block.stream }
                    as? Outbound.Audio
            (next?.block ?: block).let { it.flags = it.flags or 1 }
        }
        queue.addLast(Outbound.Audio(block))
        return dropped
    }

    fun pushMessage(type: Int, payload: ByteArray) {
        require(type in listOf(Frames.CONTROL, Frames.PING, Frames.PONG)) {
            "Not a priority message."
        }
        Messages.validatePayload(type, payload)
        queue.addLast(Outbound.Message(type, payload.copyOf()))
    }

    fun pop(): Outbound? = if (queue.isEmpty()) null else queue.removeFirst()
}

class PhoneLiveness(nowMs: Long) {
    private var lastReceived = nowMs

    fun received(nowMs: Long) {
        lastReceived = nowMs
    }

    fun shouldClose(nowMs: Long): Boolean = nowMs - lastReceived >= 10_000
}
