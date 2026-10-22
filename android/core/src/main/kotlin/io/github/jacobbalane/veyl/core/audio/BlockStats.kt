package io.github.jacobbalane.veyl.core.audio

import kotlin.math.abs
import kotlin.math.log10

class BlockStats(val rmsDbfs: Float, val peak: Float) {
    companion object {
        fun measure(samples: ShortArray): BlockStats {
            require(samples.isNotEmpty())
            var power = 0.0
            var peak = 0f
            for (raw in samples) {
                val sample = raw / 32768f
                power += sample.toDouble() * sample
                peak = maxOf(peak, abs(sample))
            }
            return BlockStats(
                if (power == 0.0) -120f else (10 * log10(power / samples.size)).toFloat(),
                peak,
            )
        }
    }
}
