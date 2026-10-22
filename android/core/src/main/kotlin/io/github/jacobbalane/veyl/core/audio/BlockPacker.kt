package io.github.jacobbalane.veyl.core.audio

import io.github.jacobbalane.veyl.core.link.AudioBlock
import kotlin.math.roundToInt

/** One packer per stream and capture run; filter state never crosses between streams. */
class BlockPacker(
    private val stream: Int,
    private val rate: Int,
    private val channels: Int,
    cutoff: Int = 8000,
) {
    private val filter = PrivacyFilter(rate, cutoff, channels)
    private var seq = 0L

    fun pack(raw: ShortArray, firstFrame: Long, captureNs: Long, flags: Int = 0): AudioBlock {
        require(raw.size == rate / 100 * channels) { "Expected a ten-millisecond audio block." }
        val stats = BlockStats.measure(raw)
        val samples = FloatArray(raw.size) { raw[it] / 32768f }
        filter.process(samples)
        val pcm =
            ShortArray(samples.size) {
                (samples[it] * 32768f).roundToInt().coerceIn(-32768, 32767).toShort()
            }
        return AudioBlock(
                stream,
                channels,
                rate,
                seq,
                firstFrame,
                captureNs,
                rate / 100,
                flags or if (stats.peak >= 0.99f) 2 else 0,
                stats.rmsDbfs,
                stats.peak,
                pcm,
            )
            .also { seq = (seq + 1) and 0xffffffffL }
    }
}

class TimestampTracker(private val rate: Int) {
    private var position: Long? = null
    private var time = Long.MIN_VALUE

    fun update(frame: Long, nanoseconds: Long) {
        position = frame
        time = nanoseconds
    }

    fun at(frame: Long): Long =
        position?.let { time + (frame - it) * 1_000_000_000L / rate } ?: Long.MIN_VALUE
}
