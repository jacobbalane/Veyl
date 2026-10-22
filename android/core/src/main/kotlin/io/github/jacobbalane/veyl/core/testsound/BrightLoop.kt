package io.github.jacobbalane.veyl.core.testsound

import io.github.jacobbalane.veyl.core.audio.Biquad
import kotlin.math.*

/** Deterministic sixteen-second music/noise loop; generated, never a recorded asset. */
class BrightLoop(val rate: Int, seed: Long = 0x5EED5EED5EED5EEDL) {
    val samples: FloatArray

    init {
        require(rate in listOf(44100, 48000) && seed != 0L)
        val count = rate * 16
        var random = seed
        val raw =
            FloatArray(count * 2) {
                random = random xor (random ushr 12)
                random = random xor (random shl 25)
                random = random xor (random ushr 27)
                ((random * 0x2545F4914F6CDD1DL ushr 40).toFloat() / 8388608f) - 1f
            }
        val hats = Array(2) { Array(2) { Biquad(rate, 7000, sqrt(0.5)) } }
        val snare = Biquad(rate, 1000, sqrt(0.5))
        val shimmer =
            Array(2) {
                arrayOf(Biquad(rate, 9000, sqrt(0.5)), Biquad(rate, 18000, sqrt(0.5), false))
            }
        for (frame in 0 until count) {
            snare.step(raw[frame * 2])
            for (ch in 0..1) {
                hats[ch][1].step(hats[ch][0].step(raw[frame * 2 + ch]))
                shimmer[ch][1].step(shimmer[ch][0].step(raw[frame * 2 + ch]))
            }
        }
        val notes =
            arrayOf(
                intArrayOf(60, 64, 67),
                intArrayOf(57, 60, 64),
                intArrayOf(53, 57, 60),
                intArrayOf(55, 59, 62),
            )
        val waves =
            notes
                .flatMap { it.toList() }
                .distinct()
                .associateWith { note ->
                    val hz = round(440 * 2.0.pow((note - 69) / 12.0) * 16) / 16
                    hz to
                        FloatArray(8192) { index ->
                            (1..(5000 / hz).toInt())
                                .sumOf { harmonic ->
                                    sin(2 * PI * harmonic * index / 8192) / harmonic
                                }
                                .toFloat()
                        }
                }
        fun pad(t: Double, chord: Int): Double =
            notes[chord].sumOf { note ->
                val (hz, wave) = waves.getValue(note)
                val phase = (hz * t % 1) * wave.size
                val index = phase.toInt()
                val fraction = phase - index
                wave[index] * (1 - fraction) + wave[(index + 1) % wave.size] * fraction
            } / 3
        samples = FloatArray(count * 2)
        for (frame in 0 until count) {
            val t = frame.toDouble() / rate
            val kickAge = t % 1
            val sweep = minOf(kickAge, 0.12)
            val kick =
                db(-8.0) *
                    exp(-kickAge / 0.060) *
                    sin(
                        2 *
                            PI *
                            (110 * sweep - (65 / 0.24) * sweep * sweep + 45 * (kickAge - sweep)),
                    )
            val snareAge = (t + 0.5) % 1
            val snareValue =
                db(-12.0) *
                    (snare.step(raw[frame * 2]) * exp(-snareAge / 0.070) +
                        0.3 * sin(2 * PI * 190 * snareAge) * exp(-snareAge / 0.040))
            val envelope =
                db(-16.0) * exp(-(t % 0.25) / 0.025) + db(-18.0) * exp(-((t + 0.25) % 2) / 0.120)
            val chord = (t / 4).toInt()
            val fade = minOf((t % 4) / 0.1, 1.0)
            val padValue = 0.19 * (pad(t, chord) * fade + pad(t, (chord + 3) % 4) * (1 - fade))
            for (ch in 0..1) {
                val input = raw[frame * 2 + ch]
                val hat = hats[ch][1].step(hats[ch][0].step(input))
                val bright = shimmer[ch][1].step(shimmer[ch][0].step(input))
                val pan = sqrt(if (((t / 0.25).toInt() + ch) % 2 == 0) 0.65 else 0.35)
                val mix =
                    kick +
                        snareValue +
                        padValue +
                        hat * envelope * pan +
                        bright * db(-36 + 3 * sin(2 * PI * 0.25 * t)) * 2.4
                samples[frame * 2 + ch] = (tanh(1.2 * mix) / tanh(1.2)).toFloat()
            }
        }
        val gain = db(-1.0) / samples.maxOf { abs(it) }
        for (i in samples.indices) samples[i] = (samples[i] * gain).toFloat()
    }

    fun pcm(firstFrame: Long, frames: Int): ShortArray =
        ShortArray(frames * 2) { index ->
            (samples[((firstFrame % (rate * 16)) * 2 + index).rem(samples.size).toInt()] * 32768f)
                .roundToInt()
                .coerceIn(-32768, 32767)
                .toShort()
        }

    private fun db(value: Double) = 10.0.pow(value / 20)
}
