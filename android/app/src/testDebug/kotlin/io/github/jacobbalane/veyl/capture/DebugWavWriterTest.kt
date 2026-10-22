package io.github.jacobbalane.veyl.capture

import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class DebugWavWriterTest {
    @get:Rule val temporary = TemporaryFolder()

    @Test
    fun recordingIsCappedAtTenSecondsAndHasALittleEndianPcmHeader() {
        for (rate in listOf(44100, 48000)) for (channels in 1..2) {
            val file = temporary.newFile()
            DebugWavWriter(file, rate, channels).use { writer ->
                repeat(1100) {
                    writer.append(
                        ShortArray(rate / 100 * channels) { if (it % 2 == 0) 0x1234 else -123 },
                    )
                }
                assertTrue(writer.complete)
            }
            val bytes = file.readBytes()
            val header = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
            assertEquals("RIFF", String(bytes, 0, 4, Charsets.US_ASCII))
            assertEquals("WAVE", String(bytes, 8, 4, Charsets.US_ASCII))
            assertEquals(36 + rate * channels * 20, header.getInt(4))
            assertEquals(channels.toShort(), header.getShort(22))
            assertEquals(rate, header.getInt(24))
            assertEquals(16.toShort(), header.getShort(34))
            assertEquals(rate * channels * 20, header.getInt(40))
            assertEquals(44 + rate * channels * 20, bytes.size)
            assertEquals(0x34.toByte(), bytes[44])
            assertEquals(0x12.toByte(), bytes[45])
            assertEquals((-123).toShort(), header.getShort(46))
        }
    }

    @Test
    fun stoppingEarlyFinalizesTheActualLengthAndCloseIsIdempotent() {
        val file = temporary.newFile()
        val writer = DebugWavWriter(file, 48000, 1)
        writer.append(shortArrayOf(1, 2, 3))
        assertFalse(writer.complete)
        writer.close()
        writer.close()
        assertEquals(50L, file.length())
        assertEquals(6, ByteBuffer.wrap(file.readBytes()).order(ByteOrder.LITTLE_ENDIAN).getInt(40))
    }
}
