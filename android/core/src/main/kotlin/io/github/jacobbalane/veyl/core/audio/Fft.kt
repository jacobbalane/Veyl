package io.github.jacobbalane.veyl.core.audio

import kotlin.math.*

/** In-place radix-2 transform. Inverse includes 1/N scaling. No platform or native dependency. */
object Fft {
    fun transform(real: DoubleArray, imaginary: DoubleArray, inverse: Boolean = false) {
        val n = real.size
        require(n >= 2 && n and (n - 1) == 0 && imaginary.size == n)
        var j = 0
        for (i in 1 until n) {
            var bit = n shr 1
            while (j and bit != 0) {
                j = j xor bit
                bit = bit shr 1
            }
            j = j xor bit
            if (i < j) {
                var value = real[i]
                real[i] = real[j]
                real[j] = value
                value = imaginary[i]
                imaginary[i] = imaginary[j]
                imaginary[j] = value
            }
        }
        var length = 2
        while (length <= n) {
            val angle = (if (inverse) 2 else -2) * PI / length
            val stepR = cos(angle)
            val stepI = sin(angle)
            for (start in 0 until n step length) {
                var wr = 1.0
                var wi = 0.0
                for (offset in 0 until length / 2) {
                    val a = start + offset
                    val b = a + length / 2
                    val br = real[b] * wr - imaginary[b] * wi
                    val bi = real[b] * wi + imaginary[b] * wr
                    real[b] = real[a] - br
                    imaginary[b] = imaginary[a] - bi
                    real[a] += br
                    imaginary[a] += bi
                    val next = wr * stepR - wi * stepI
                    wi = wr * stepI + wi * stepR
                    wr = next
                }
            }
            if (length == n) break
            length = length shl 1
        }
        if (inverse)
            for (i in 0 until n) {
                real[i] /= n
                imaginary[i] /= n
            }
    }
}
