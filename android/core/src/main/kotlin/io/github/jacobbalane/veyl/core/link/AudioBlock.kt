package io.github.jacobbalane.veyl.core.link

import java.nio.ByteBuffer
import java.nio.ByteOrder

class AudioBlock(
    val stream: Int,
    val channels: Int,
    val sampleRate: Int,
    val seq: Long,
    val firstFrame: Long,
    val captureNs: Long,
    val frames: Int,
    var flags: Int,
    val preRmsDbfs: Float,
    val prePeak: Float,
    val samples: ShortArray,
) {
    private fun validate() {
        require(stream in 0..1 && channels in 1..4 && sampleRate in listOf(44100, 48000)) {
            "Invalid AUDIO format."
        }
        require(seq in 0..0xffffffffL && frames in 1..4096 && flags and 15 == flags) {
            "Invalid AUDIO header."
        }
        require(preRmsDbfs.isFinite() && prePeak.isFinite() && samples.size == frames * channels) {
            "Invalid AUDIO samples."
        }
    }

    /** Includes the plaintext type byte, so offsets match the specification. */
    fun encode(): ByteArray {
        validate()
        return ByteBuffer.allocate(40 + samples.size * 2)
            .order(ByteOrder.LITTLE_ENDIAN)
            .apply {
                put(Frames.AUDIO.toByte())
                put(stream.toByte())
                put(channels.toByte())
                put(1)
                putInt(sampleRate)
                putInt(seq.toInt())
                putLong(firstFrame)
                putLong(captureNs)
                putShort(frames.toShort())
                putShort(flags.toShort())
                putFloat(preRmsDbfs)
                putFloat(prePeak)
                samples.forEach(::putShort)
            }
            .array()
    }

    companion object {
        fun decode(bytes: ByteArray): AudioBlock {
            require(bytes.size >= 40) { "Invalid AUDIO length." }
            val buffer = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
            require(buffer.get().toInt() and 255 == Frames.AUDIO)
            val stream = buffer.get().toInt() and 255
            val channels = buffer.get().toInt() and 255
            require(buffer.get().toInt() == 1) { "Invalid AUDIO format." }
            val rate = buffer.int
            val seq = buffer.int.toLong() and 0xffffffffL
            val first = buffer.long
            val capture = buffer.long
            val frames = buffer.short.toInt() and 65535
            val flags = buffer.short.toInt() and 65535
            val rms = buffer.float
            val peak = buffer.float
            require(
                channels in 1..4 && frames in 1..4096 && bytes.size == 40 + frames * channels * 2,
            ) {
                "Invalid AUDIO length."
            }
            val samples = ShortArray(frames * channels) { buffer.short }
            return AudioBlock(
                    stream,
                    channels,
                    rate,
                    seq,
                    first,
                    capture,
                    frames,
                    flags,
                    rms,
                    peak,
                    samples,
                )
                .also { it.validate() }
        }
    }
}

object PingPong {
    fun encodePing(pcNs: Long): ByteArray =
        ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putLong(pcNs).array()

    fun ping(bytes: ByteArray): Long {
        require(bytes.size == 8)
        return ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN).long
    }

    fun encodePong(pcNs: Long, phoneNs: Long): ByteArray =
        ByteBuffer.allocate(16)
            .order(ByteOrder.LITTLE_ENDIAN)
            .putLong(pcNs)
            .putLong(phoneNs)
            .array()

    fun pong(bytes: ByteArray): Pair<Long, Long> {
        require(bytes.size == 16)
        val buffer = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
        return buffer.long to buffer.long
    }
}
