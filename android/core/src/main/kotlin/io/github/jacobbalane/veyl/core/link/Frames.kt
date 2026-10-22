package io.github.jacobbalane.veyl.core.link

import java.io.ByteArrayInputStream
import java.io.DataInputStream
import java.io.InputStream
import java.io.OutputStream
import java.nio.ByteBuffer

class Frame(val kind: Int, val body: ByteArray) {
    override fun toString(): String = "Frame(kind=$kind, bytes=${body.size})"
}

object Frames {
    const val HANDSHAKE = 1
    const val SEALED = 2
    const val CONTROL = 0x10
    const val AUDIO = 0x20
    const val IMU = 0x21
    const val PING = 0x30
    const val PONG = 0x31
    const val MAX_FRAME = 1_048_576
    const val MAX_HANDSHAKE = 8192
    const val MAX_CONTROL = 65_536

    private fun validate(kind: Int, size: Int) {
        require(kind == HANDSHAKE || kind == SEALED) { "Invalid frame kind." }
        require(size in 0 until MAX_FRAME) { "Invalid frame size." }
        require(kind != HANDSHAKE || size <= MAX_HANDSHAKE) { "Handshake size limit." }
    }

    fun encode(frame: Frame): ByteArray {
        validate(frame.kind, frame.body.size)
        return ByteBuffer.allocate(5 + frame.body.size)
            .putInt(frame.body.size + 1)
            .put(frame.kind.toByte())
            .put(frame.body)
            .array()
    }

    fun read(input: InputStream): Frame {
        val reader = DataInputStream(input)
        val length = reader.readInt()
        require(length in 1..MAX_FRAME) { "Invalid frame length." }
        val kind = reader.readUnsignedByte()
        validate(kind, length - 1)
        val body = ByteArray(length - 1)
        reader.readFully(body)
        return Frame(kind, body)
    }

    fun decode(wire: ByteArray): Frame {
        val input = ByteArrayInputStream(wire)
        val frame = read(input)
        require(input.available() == 0) { "Trailing frame bytes." }
        return frame
    }

    fun write(output: OutputStream, frame: Frame) {
        output.write(encode(frame))
        output.flush()
    }
}
