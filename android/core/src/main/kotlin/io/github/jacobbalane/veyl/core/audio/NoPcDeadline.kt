package io.github.jacobbalane.veyl.core.audio

/** Production stops after ten minutes without a PC; only the explicit debug Lab is exempt. */
class NoPcDeadline(startedMs: Long, private val debugLab: Boolean = false) {
    private var lastConnectedMs = startedMs

    fun expired(nowMs: Long, pcConnected: Boolean): Boolean {
        if (pcConnected) lastConnectedMs = nowMs
        return !debugLab && nowMs - lastConnectedMs >= 600_000
    }
}
