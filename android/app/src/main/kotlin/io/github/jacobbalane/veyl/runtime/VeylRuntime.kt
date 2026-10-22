package io.github.jacobbalane.veyl.runtime

import android.content.Context
import android.os.Build
import android.os.SystemClock
import io.github.jacobbalane.veyl.BuildConfig
import io.github.jacobbalane.veyl.core.audio.BlockStats
import io.github.jacobbalane.veyl.core.audio.CaptureReducer
import io.github.jacobbalane.veyl.core.audio.CaptureStatus
import io.github.jacobbalane.veyl.core.link.AudioBlock
import io.github.jacobbalane.veyl.core.testsound.BrightLoop
import java.time.Instant
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class StreamLevel(
    val blocks: Long = 0,
    val rms: Float = -120f,
    val highRms: Float = -120f,
    val channels: Int = 0,
)

data class PlayerSnapshot(
    val playing: Boolean = false,
    val index: Int = 0,
    val count: Int = 0,
    val shuffle: Boolean = false,
)

data class CheckSnapshot(
    val state: String = "idle",
    val current: Int = 0,
    val total: Int = 0,
    val result: CheckedMic? = null,
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
    val status: CaptureStatus = CaptureStatus(),
    val route: String = "unknown",
    val info: CaptureInfo? = null,
    val reference: StreamLevel = StreamLevel(),
    val microphone: StreamLevel = StreamLevel(),
    val startedMs: Long = 0,
) {
    val state
        get() = status.state

    val source
        get() = status.source

    val reason
        get() = status.reason

    val micSilenced
        get() = status.silenced
}

/** Process-wide state survives activity rotation. Only metadata reaches the UI. */
class VeylRuntime(context: Context) {
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    val settings = Settings(context, scope)
    private val loops = ConcurrentHashMap<Int, BrightLoop>()

    fun testLoop(rate: Int): BrightLoop = loops.computeIfAbsent(rate) { BrightLoop(it) }

    private val playerMutable = MutableStateFlow(PlayerSnapshot())
    val player = playerMutable.asStateFlow()

    fun updatePlayer(value: PlayerSnapshot) {
        playerMutable.value = value
    }

    private val checkMutable = MutableStateFlow(CheckSnapshot())
    val check = checkMutable.asStateFlow()

    fun updateCheck(value: CheckSnapshot) {
        checkMutable.value = value
    }

    @Volatile var hasProjection = false
    private val stoppingMutable = MutableStateFlow(false)
    val serviceStopping = stoppingMutable.asStateFlow()

    fun serviceStopping(value: Boolean) {
        stoppingMutable.value = value
    }

    private val mutable = MutableStateFlow(CaptureSnapshot())
    val capture = mutable.asStateFlow()
    private val generation = AtomicLong()
    private val counts = Array(2) { AtomicLong() }
    private val lastPublish = Array(2) { AtomicLong() }
    @Volatile var pcConnected = false
    private val changes = ArrayDeque<String>()

    @Synchronized
    private fun event(value: String) {
        changes.addLast("${Instant.now()} $value")
        while (changes.size > 20) changes.removeFirst()
    }

    @Synchronized
    fun diagnostics(): String {
        val current = capture.value
        val info = current.info
        return "Veyl ${BuildConfig.VERSION_NAME}; Android ${Build.VERSION.RELEASE} (${Build.VERSION.SDK_INT}); ${Build.MANUFACTURER} ${Build.MODEL}\n" +
            "state=${current.state}; source=${current.source}; reason=${current.reason}; route=${current.route}; hp_cutoff_hz=${settings.state.value.cutoff}\n" +
            "sample_rate=${info?.rate}; mic_source=${info?.micSource}; mic_channels=${info?.micChannels}; effects_off=${info?.effectsOff}; effects_on=${info?.effectsOn}\n" +
            changes.joinToString("\n")
    }

    fun starting(source: String): Long {
        val id = generation.incrementAndGet()
        event("starting source=$source run=$id")
        counts.forEach { it.set(0) }
        lastPublish.forEach { it.set(0) }
        mutable.value =
            CaptureSnapshot(
                route = mutable.value.route,
                status = CaptureReducer.start(mutable.value.status, id, source),
                startedMs = SystemClock.elapsedRealtime(),
            )
        return id
    }

    fun running(id: Long, info: CaptureInfo) {
        event(
            "running run=$id rate=${info.rate} mic=${info.micSource} channels=${info.micChannels}",
        )
        if (generation.get() == id)
            mutable.update { it.copy(status = CaptureReducer.running(it.status, id), info = info) }
    }

    fun block(id: Long, block: AudioBlock) {
        if (generation.get() != id || mutable.value.state != "running") return
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
                if (it.status.runId != id || it.state != "running") it
                else if (block.stream == 0) it.copy(reference = level)
                else it.copy(microphone = level)
            }
        }
    }

    fun stopped(id: Long, reason: String) {
        event("stopped run=$id reason=$reason")
        if (generation.get() == id)
            mutable.update {
                it.copy(
                    status = CaptureReducer.stopped(it.status, id, reason),
                )
            }
    }

    fun route(route: String) {
        if (mutable.value.route != route) event("route=$route")
        mutable.update { it.copy(route = route) }
    }

    fun silenced(id: Long, value: Boolean) {
        if (mutable.value.micSilenced != value && mutable.value.status.runId == id)
            event("mic_silenced=$value")
        mutable.update { it.copy(status = CaptureReducer.silenced(it.status, id, value)) }
    }
}
