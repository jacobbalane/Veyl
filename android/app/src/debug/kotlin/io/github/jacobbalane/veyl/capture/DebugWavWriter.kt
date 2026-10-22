package io.github.jacobbalane.veyl.capture

import java.io.File
import java.io.RandomAccessFile

/** The sole phone recording writer. Compiled only in the explicitly requested M2 debug Lab. */
class DebugWavWriter(file: File, private val rate: Int, private val channels: Int) : AutoCloseable {
    private val output = RandomAccessFile(file, "rw")
    private var samples = 0
    private var closed = false
    val complete: Boolean
        get() = samples == rate * channels * 10

    init {
        require(rate in listOf(44100, 48000) && channels in 1..2)
        output.setLength(0)
        output.write(ByteArray(44))
    }

    fun append(pcm: ShortArray) {
        check(!closed)
        val count = minOf(pcm.size, rate * channels * 10 - samples)
        val bytes = ByteArray(count * 2)
        for (i in 0 until count) {
            bytes[i * 2] = pcm[i].toByte()
            bytes[i * 2 + 1] = (pcm[i].toInt() shr 8).toByte()
        }
        output.write(bytes)
        samples += count
    }

    override fun close() {
        if (closed) return
        closed = true
        try {
            output.seek(0)
            output.writeBytes("RIFF")
            little(36 + samples * 2, 4)
            output.writeBytes("WAVEfmt ")
            little(16, 4)
            little(1, 2)
            little(channels, 2)
            little(rate, 4)
            little(rate * channels * 2, 4)
            little(channels * 2, 2)
            little(16, 2)
            output.writeBytes("data")
            little(samples * 2, 4)
        } finally {
            output.close()
        }
    }

    private fun little(value: Int, size: Int) {
        repeat(size) { output.write(value ushr (it * 8) and 255) }
    }
}
