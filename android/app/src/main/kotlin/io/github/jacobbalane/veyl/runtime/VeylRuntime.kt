package io.github.jacobbalane.veyl.runtime

import android.content.Context
import android.os.SystemClock
import io.github.jacobbalane.veyl.core.audio.BlockStats
import io.github.jacobbalane.veyl.core.link.AudioBlock
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class StreamLevel(
    val blocks: Long = 0,
    val rms: Float = -120f,
    val highRms: Float = -120f,
    val channels: Int = 0,
)

data class CaptureInfo(
    val rate: Int,
    val micSource: String,
    val micChannels: Int,
    val effectsOff: List<String>,
    val effectsOn: List<String> = emptyList(),
    val microphones: List<MicMetadata> = emptyList(),
)

data class MicMetadata(
    val index: Int,
    val type: String,
    val positionM: List<Float>?,
    val channels: List<Int>,
)

data class CaptureSnapshot(
    val state: String = "idle",
    val source: String = "capture",
    val reason: String? = null,
    val route: String = "unknown",
    val info: CaptureInfo? = null,
    val reference: StreamLevel = StreamLevel(),
    val microphone: StreamLevel = StreamLevel(),
    val startedMs: Long = 0,
    val micSilenced: Boolean = false,
)

/** Process-wide state survives activity rotation. Only metadata reaches the UI. */
class VeylRuntime(@Suppress("UNUSED_PARAMETER") context: Context) {
    private val mutable = MutableStateFlow(CaptureSnapshot())
    val capture = mutable.asStateFlow()
    private val generation = AtomicLong()
    private val counts = Array(2) { AtomicLong() }
    private val lastPublish = Array(2) { AtomicLong() }
    @Volatile var pcConnected = false

    fun starting(source: String): Long {
        val id = generation.incrementAndGet()
        counts.forEach { it.set(0) }
        lastPublish.forEach { it.set(0) }
        mutable.value =
            CaptureSnapshot(
                route = mutable.value.route,
                state = "starting",
                source = source,
                startedMs = SystemClock.elapsedRealtime(),
            )
        return id
    }

    fun running(id: Long, info: CaptureInfo) {
        if (generation.get() == id) mutable.update { it.copy(state = "running", info = info) }
    }

    fun block(id: Long, block: AudioBlock) {
        if (generation.get() != id) return
        // This is the M2 null link: validate the real wire payload and discard filtered samples.
        block.encode()
        val count = counts[block.stream].incrementAndGet()
        val now = SystemClock.elapsedRealtime()
        if (now - lastPublish[block.stream].get() >= 200) {
            lastPublish[block.stream].set(now)
            val level =
                StreamLevel(
                    count,
                    block.preRmsDbfs,
                    BlockStats.measure(block.samples).rmsDbfs,
                    block.channels,
                )
            mutable.update {
                if (block.stream == 0) it.copy(reference = level) else it.copy(microphone = level)
            }
        }
    }

    fun stopped(id: Long, reason: String) {
        if (generation.get() == id)
            mutable.update {
                it.copy(
                    state =
                        if (reason in listOf("mic_error", "capture_error", "permission")) "error"
                        else "stopped",
                    reason = reason,
                )
            }
    }

    fun route(route: String) {
        mutable.update { it.copy(route = route) }
    }

    fun silenced(value: Boolean) {
        mutable.update { it.copy(micSilenced = value) }
    }
}
