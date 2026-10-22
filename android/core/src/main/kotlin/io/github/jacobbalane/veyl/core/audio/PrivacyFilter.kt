package io.github.jacobbalane.veyl.core.audio

import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

internal class Biquad(rate: Int, cutoff: Int, q: Double, highpass: Boolean = true) {
    private val omega = 2 * PI * cutoff / rate
    private val c = cos(omega)
    private val alpha = sin(omega) / (2 * q)
    private val a0 = 1 + alpha
    private val b0 = ((if (highpass) 1 + c else 1 - c) / (2 * a0)).toFloat()
    private val b1 = ((if (highpass) -(1 + c) else 1 - c) / a0).toFloat()
    private val a1 = (-2 * c / a0).toFloat()
    private val a2 = ((1 - alpha) / a0).toFloat()
    private var z1 = 0f
    private var z2 = 0f

    fun step(input: Float): Float {
        val output = b0 * input + z1
        z1 = b1 * input - a1 * output + z2
        z2 = b0 * input - a2 * output
        return output
    }

    fun reset() {
        z1 = 0f
        z2 = 0f
    }
}

class PrivacyFilter(rate: Int, cutoff: Int = 8000, private val channels: Int = 1) {
    init {
        require(rate in listOf(44100, 48000))
        require(cutoff in listOf(6000, 7000, 8000))
        require(channels in 1..4)
    }

    private val filters =
        Array(channels) {
            arrayOf(0.50980, 0.60134, 0.89998, 2.56292).map { Biquad(rate, cutoff, it) }
        }

    fun process(samples: FloatArray) {
        require(samples.size % channels == 0) { "Incomplete audio frame." }
        for (index in samples.indices) {
            var sample = samples[index]
            for (stage in filters[index % channels]) sample = stage.step(sample)
            samples[index] = sample
        }
    }

    fun reset() {
        filters.forEach { stages -> stages.forEach { it.reset() } }
    }
}
