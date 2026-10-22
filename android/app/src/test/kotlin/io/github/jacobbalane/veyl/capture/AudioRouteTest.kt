package io.github.jacobbalane.veyl.capture

import android.media.AudioDeviceInfo
import org.junit.Assert.assertEquals
import org.junit.Test

class AudioRouteTest {
    @Test
    fun olderAndroidTreatsAnyAvailableExternalOutputAsExternal() {
        assertEquals("speaker", AudioRoute.classify(setOf(AudioDeviceInfo.TYPE_BUILTIN_SPEAKER)))
        assertEquals("speaker", AudioRoute.classify(emptySet()))
        for (type in
            listOf(
                AudioDeviceInfo.TYPE_BLUETOOTH_A2DP,
                AudioDeviceInfo.TYPE_WIRED_HEADPHONES,
                AudioDeviceInfo.TYPE_USB_DEVICE,
                AudioDeviceInfo.TYPE_BLE_HEADSET,
                AudioDeviceInfo.TYPE_HEARING_AID,
                AudioDeviceInfo.TYPE_LINE_ANALOG,
                AudioDeviceInfo.TYPE_HDMI,
            )) {
            assertEquals(
                "external",
                AudioRoute.classify(setOf(AudioDeviceInfo.TYPE_BUILTIN_SPEAKER, type)),
            )
        }
    }
}
