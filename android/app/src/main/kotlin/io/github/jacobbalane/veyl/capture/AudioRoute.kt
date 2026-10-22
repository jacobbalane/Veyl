package io.github.jacobbalane.veyl.capture

import android.media.*
import android.os.Build
import android.os.Handler
import android.os.Looper

class AudioRoute(private val manager: AudioManager, private val changed: (String) -> Unit) :
    AutoCloseable {
    private val callback =
        object : AudioDeviceCallback() {
            override fun onAudioDevicesAdded(addedDevices: Array<out AudioDeviceInfo>) {
                refresh()
            }

            override fun onAudioDevicesRemoved(removedDevices: Array<out AudioDeviceInfo>) {
                refresh()
            }
        }

    fun start() {
        manager.registerAudioDeviceCallback(callback, Handler(Looper.getMainLooper()))
        refresh()
    }

    private fun refresh() {
        val route =
            try {
                if (Build.VERSION.SDK_INT >= 33) {
                    val devices =
                        manager.getAudioDevicesForAttributes(
                            AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).build(),
                        )
                    classifyActive(devices.map { it.type }.toSet())
                } else
                    classify(
                        manager
                            .getDevices(AudioManager.GET_DEVICES_OUTPUTS)
                            .map { it.type }
                            .toSet(),
                    )
            } catch (_: RuntimeException) {
                "unknown"
            }
        changed(route)
    }

    override fun close() {
        manager.unregisterAudioDeviceCallback(callback)
    }

    companion object {
        fun classifyActive(types: Set<Int>): String =
            if (types.isEmpty()) "unknown"
            else if (
                types.all {
                    it == AudioDeviceInfo.TYPE_BUILTIN_SPEAKER ||
                        it == AudioDeviceInfo.TYPE_BUILTIN_SPEAKER_SAFE
                }
            )
                "speaker"
            else "external"

        fun classify(types: Set<Int>): String =
            if (
                types.any {
                    it in
                        setOf(
                            AudioDeviceInfo.TYPE_BLUETOOTH_A2DP,
                            AudioDeviceInfo.TYPE_BLE_HEADSET,
                            AudioDeviceInfo.TYPE_BLE_SPEAKER,
                            AudioDeviceInfo.TYPE_WIRED_HEADSET,
                            AudioDeviceInfo.TYPE_WIRED_HEADPHONES,
                            AudioDeviceInfo.TYPE_USB_HEADSET,
                            AudioDeviceInfo.TYPE_USB_DEVICE,
                            AudioDeviceInfo.TYPE_HEARING_AID,
                            AudioDeviceInfo.TYPE_LINE_ANALOG,
                            AudioDeviceInfo.TYPE_LINE_DIGITAL,
                            AudioDeviceInfo.TYPE_HDMI,
                            AudioDeviceInfo.TYPE_HDMI_ARC,
                            AudioDeviceInfo.TYPE_HDMI_EARC,
                            AudioDeviceInfo.TYPE_BLUETOOTH_SCO,
                            AudioDeviceInfo.TYPE_BLE_BROADCAST,
                        )
                }
            )
                "external"
            else "speaker"
    }
}
