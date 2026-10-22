package io.github.jacobbalane.veyl.capture

import android.media.AudioDeviceInfo
import org.junit.Assert.assertEquals
import org.junit.Test

class AudioRouteTest {
    @Test
    fun newerAndroidUsesTheActualMediaRouteAndReportsAnUnknownEmptyRoute() {
        assertEquals("unknown", AudioRoute.classifyActive(emptySet()))
        assertEquals(
            "speaker",
            AudioRoute.classifyActive(setOf(AudioDeviceInfo.TYPE_BUILTIN_SPEAKER_SAFE)),
        )
        assertEquals(
            "external",
            AudioRoute.classifyActive(setOf(AudioDeviceInfo.TYPE_BLUETOOTH_A2DP)),
        )
        assertEquals(
            "external",
            AudioRoute.classifyActive(
                setOf(AudioDeviceInfo.TYPE_BUILTIN_SPEAKER, AudioDeviceInfo.TYPE_WIRED_HEADPHONES),
            ),
        )
    }

    @Test
    fun aCheckedMicIsTriedAtBothRatesBeforeOtherSourcesAndDefaultOrderKeepsTheSharedRate() {
        val first = MicChoice(9, 2, "voice_recognition")
        val second = MicChoice(1, 1, "mic")
        assertEquals(
            listOf(48000 to first, 48000 to second, 44100 to first, 44100 to second),
            Recorders.order(listOf(first, second), null),
        )
        assertEquals(
            listOf(48000 to second, 44100 to second, 48000 to first, 44100 to first),
            Recorders.order(listOf(first, second), second),
        )
    }

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
