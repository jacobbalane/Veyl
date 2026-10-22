package io.github.jacobbalane.veyl.core.audio

import kotlin.math.*

data class SoundCheckResult(
    val lagFrames: Int,
    val meanCoherence: Double,
    val coverage: Double,
    val score: Double,
)

data class SoundCheckCandidate(
    val source: String,
    val channels: Int,
    val effectsOn: Int,
    val result: SoundCheckResult,
)

object SoundCheck {
    fun analyze(reference: FloatArray, microphone: FloatArray, rate: Int): SoundCheckResult {
        require(
            rate in listOf(44100, 48000) &&
                reference.all { it.isFinite() } &&
                microphone.all { it.isFinite() },
        )
        if (minOf(reference.size, microphone.size) < 3072) return SoundCheckResult(0, 0.0, 0.0, 0.0)
        var size = 2
        while (size < reference.size + microphone.size) size = size shl 1
        val rr = DoubleArray(size) { reference.getOrElse(it) { 0f }.toDouble() }
        val ri = DoubleArray(size)
        val mr = DoubleArray(size) { microphone.getOrElse(it) { 0f }.toDouble() }
        val mi = DoubleArray(size)
        Fft.transform(rr, ri)
        Fft.transform(mr, mi)
        for (bin in 0 until size) {
            val hz = minOf(bin, size - bin).toDouble() * rate / size
            val real = rr[bin] * mr[bin] + ri[bin] * mi[bin]
            val imaginary = rr[bin] * mi[bin] - ri[bin] * mr[bin]
            rr[bin] = if (hz in 8000.0..20000.0) real else 0.0
            ri[bin] = if (hz in 8000.0..20000.0) imaginary else 0.0
        }
        Fft.transform(rr, ri, inverse = true)
        val limit = minOf(rate / 2, minOf(reference.size, microphone.size) - 3072)
        // Both recordings start together. Search half a second in either direction for
        // acoustic/driver delay.
        val lag = (-limit..limit).maxBy { abs(rr[if (it >= 0) it else size + it]) }
        val refStart = maxOf(0, -lag)
        val micStart = maxOf(0, lag)
        val length = minOf(reference.size - refStart, microphone.size - micStart)
        val n = 2048
        val xx = DoubleArray(n / 2 + 1)
        val yy = DoubleArray(xx.size)
        val xyR = DoubleArray(xx.size)
        val xyI = DoubleArray(xx.size)
        val xr = DoubleArray(n)
        val xi = DoubleArray(n)
        val yr = DoubleArray(n)
        val yi = DoubleArray(n)
        val window = DoubleArray(n) { 0.5 - 0.5 * cos(2 * PI * it / (n - 1)) }
        for (offset in 0..length - n step n / 2) {
            for (i in 0 until n) {
                xr[i] = reference[refStart + offset + i] * window[i]
                yr[i] = microphone[micStart + offset + i] * window[i]
            }
            xi.fill(0.0)
            yi.fill(0.0)
            Fft.transform(xr, xi)
            Fft.transform(yr, yi)
            for (bin in xx.indices) {
                xx[bin] += xr[bin] * xr[bin] + xi[bin] * xi[bin]
                yy[bin] += yr[bin] * yr[bin] + yi[bin] * yi[bin]
                xyR[bin] += xr[bin] * yr[bin] + xi[bin] * yi[bin]
                xyI[bin] += xi[bin] * yr[bin] - xr[bin] * yi[bin]
            }
        }
        val bins = ceil(8000.0 * n / rate).toInt()..floor(20000.0 * n / rate).toInt()
        var mean = 0.0
        var covered = 0
        for (bin in bins) {
            val power = xx[bin] * yy[bin]
            val coherence =
                if (power <= 1e-20) 0.0
                else ((xyR[bin] * xyR[bin] + xyI[bin] * xyI[bin]) / power).coerceIn(0.0, 1.0)
            mean += coherence
            if (coherence >= 0.5) covered++
        }
        mean /= bins.count()
        val coverage = covered.toDouble() / bins.count()
        return SoundCheckResult(lag, mean, coverage, mean * coverage)
    }

    fun best(candidates: List<SoundCheckCandidate>): SoundCheckCandidate? =
        candidates.reduceOrNull { best, next ->
            if (
                next.result.score > best.result.score + 1e-6 ||
                    (abs(next.result.score - best.result.score) <= 1e-6 &&
                        (next.effectsOn < best.effectsOn ||
                            (next.effectsOn == best.effectsOn && next.channels > best.channels)))
            )
                next
            else best
        }
}
