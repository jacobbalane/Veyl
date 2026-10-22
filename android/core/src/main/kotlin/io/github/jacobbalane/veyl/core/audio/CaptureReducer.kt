package io.github.jacobbalane.veyl.core.audio

data class CaptureStatus(
    val runId: Long = 0,
    val source: String = "capture",
    val state: String = "idle",
    val reason: String? = null,
    val silenced: Boolean = false,
)

/** Delayed reader callbacks cannot restart a stopped run or overwrite a newer run. */
object CaptureReducer {
    fun start(previous: CaptureStatus, id: Long, source: String): CaptureStatus {
        require(id > previous.runId && source in listOf("capture", "player", "test_sound"))
        return CaptureStatus(id, source, "starting")
    }

    fun running(previous: CaptureStatus, id: Long): CaptureStatus =
        if (previous.runId == id && previous.state == "starting") previous.copy(state = "running")
        else previous

    fun stopped(previous: CaptureStatus, id: Long, reason: String): CaptureStatus =
        if (previous.runId != id || previous.state in listOf("stopped", "error")) previous
        else
            previous.copy(
                state =
                    if (reason in listOf("mic_error", "capture_error", "permission")) "error"
                    else "stopped",
                reason = reason,
                silenced = false,
            )

    fun silenced(previous: CaptureStatus, id: Long, value: Boolean): CaptureStatus =
        if (previous.runId == id && previous.state == "running") previous.copy(silenced = value)
        else previous
}
