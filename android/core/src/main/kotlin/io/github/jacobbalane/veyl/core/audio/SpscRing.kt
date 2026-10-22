package io.github.jacobbalane.veyl.core.audio

import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.atomic.AtomicReferenceArray

/** One producer and one consumer. A full ring drops new input instead of blocking playback. */
class SpscRing<T : Any>(private val capacity: Int) {
    init {
        require(capacity > 0)
    }

    private val slots = AtomicReferenceArray<T>(capacity)
    private val written = AtomicLong()
    private val read = AtomicLong()

    fun offer(item: T): Boolean {
        val index = written.get()
        if (index - read.get() >= capacity) return false
        slots.set((index % capacity).toInt(), item)
        written.lazySet(index + 1)
        return true
    }

    fun poll(): T? {
        val index = read.get()
        if (index == written.get()) return null
        val item = slots.getAndSet((index % capacity).toInt(), null)
        read.lazySet(index + 1)
        return item
    }
}

object StereoPcm {
    fun mix(input: ShortArray, channels: Int): ShortArray {
        require(channels in 1..8 && input.size % channels == 0)
        return ShortArray(input.size / channels * 2) { index ->
            val start = index / 2 * channels
            when (channels) {
                1 -> input[start]
                2 -> input[start + index % 2]
                else -> {
                    // Keep the front pair distinct and share the other channels without clipping.
                    var shared = 0
                    for (ch in 2 until channels) shared += input[start + ch]
                    ((2 * input[start + index % 2] + shared) / channels).toShort()
                }
            }
        }
    }
}
