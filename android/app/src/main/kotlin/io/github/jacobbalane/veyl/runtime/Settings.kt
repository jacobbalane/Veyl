package io.github.jacobbalane.veyl.runtime

import android.content.Context
import android.os.Build
import androidx.datastore.preferences.core.*
import androidx.datastore.preferences.preferencesDataStore
import io.github.jacobbalane.veyl.core.util.Base64Url
import io.github.jacobbalane.veyl.core.util.Utf8
import java.io.IOException
import java.security.SecureRandom
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject

private val Context.veylSettings by preferencesDataStore("settings")

data class CheckedMic(
    val source: String,
    val channels: Int,
    val rate: Int,
    val score: Double,
    val meanCoherence: Double,
)

data class PhoneSettings(
    val phoneId: String = "",
    val phoneName: String = Utf8.prefix(Build.MODEL).ifEmpty { "Veyl" },
    val cutoff: Int = 8000,
    val dim: Boolean = true,
    val pip: Boolean = true,
    val onboarded: Boolean = false,
    val explained: Boolean = false,
    val source: String = if (Build.VERSION.SDK_INT >= 29) "capture" else "test_sound",
    val playerItems: List<String> = emptyList(),
    val checkedMic: CheckedMic? = null,
)

class Settings(context: Context, private val scope: CoroutineScope) {
    private val store = context.veylSettings
    private val data =
        store.data.catch { if (it is IOException) emit(emptyPreferences()) else throw it }
    val state = data.map(::decode).stateIn(scope, SharingStarted.Eagerly, PhoneSettings())

    init {
        scope.launch {
            store.edit {
                if (it[PHONE_ID] == null)
                    it[PHONE_ID] =
                        Base64Url.encode(
                            ByteArray(16).also { bytes -> SecureRandom().nextBytes(bytes) },
                        )
            }
        }
    }

    suspend fun read(): PhoneSettings = decode(data.first())

    fun cutoff(value: Int) {
        require(value in listOf(6000, 7000, 8000))
        edit { it[CUTOFF] = value }
    }

    fun dim(value: Boolean) = edit { it[DIM] = value }

    fun pip(value: Boolean) = edit { it[PIP] = value }

    fun onboard() = edit { it[ONBOARDED] = true }

    fun explained() = edit { it[EXPLAINED] = true }

    fun name(value: String) = edit {
        it[PHONE_NAME] = Utf8.prefix(value).ifEmpty { Utf8.prefix(Build.MODEL).ifEmpty { "Veyl" } }
    }

    fun source(value: String) {
        require(value in listOf("capture", "player", "test_sound"))
        edit { it[SOURCE] = value }
    }

    fun playerItems(values: List<String>) = edit {
        it[PLAYER_ITEMS] = JSONArray(values.distinct()).toString()
    }

    suspend fun checked(mic: CheckedMic) {
        store.edit {
            it[SOUND_CHECK] =
                JSONObject()
                    .put("fingerprint", Build.FINGERPRINT)
                    .put("mic_source", mic.source)
                    .put("mic_channels", mic.channels)
                    .put("rate", mic.rate)
                    .put("score", mic.score)
                    .put("mean_coherence", mic.meanCoherence)
                    .toString()
        }
    }

    private fun edit(change: (MutablePreferences) -> Unit) {
        scope.launch { store.edit { change(it) } }
    }

    private fun decode(p: Preferences): PhoneSettings {
        val checked = runCatching {
            JSONObject(p[SOUND_CHECK] ?: "{}")
                .takeIf { it.optString("fingerprint") == Build.FINGERPRINT }
                ?.let {
                    CheckedMic(
                        it.getString("mic_source"),
                        it.getInt("mic_channels"),
                        it.getInt("rate"),
                        it.getDouble("score"),
                        it.getDouble("mean_coherence"),
                    )
                }
        }
            .getOrNull()
            ?.takeIf {
                it.source in listOf("unprocessed", "voice_recognition", "camcorder", "mic") &&
                    it.channels in 1..2 &&
                    it.rate in listOf(44100, 48000) &&
                    it.score in 0.0..1.0 &&
                    it.meanCoherence in 0.0..1.0
            }
        val items = runCatching {
            val array = JSONArray(p[PLAYER_ITEMS] ?: "[]")
            List(array.length()) { array.getString(it) }
        }
            .getOrDefault(emptyList())
        return PhoneSettings(
            p[PHONE_ID] ?: "",
            Utf8.prefix(p[PHONE_NAME] ?: Build.MODEL).ifEmpty { "Veyl" },
            p[CUTOFF]?.takeIf { it in listOf(6000, 7000, 8000) } ?: 8000,
            p[DIM] ?: true,
            p[PIP] ?: true,
            p[ONBOARDED] ?: false,
            p[EXPLAINED] ?: false,
            p[SOURCE]?.takeIf {
                it in listOf("capture", "player", "test_sound") &&
                    (it != "capture" || Build.VERSION.SDK_INT >= 29)
            } ?: if (Build.VERSION.SDK_INT >= 29) "capture" else "test_sound",
            items,
            checked,
        )
    }

    private companion object {
        val PHONE_ID = stringPreferencesKey("phone_id")
        val PHONE_NAME = stringPreferencesKey("phone_name")
        val CUTOFF = intPreferencesKey("hp_cutoff_hz")
        val DIM = booleanPreferencesKey("dim_screen")
        val PIP = booleanPreferencesKey("pip")
        val ONBOARDED = booleanPreferencesKey("onboarding_done")
        val EXPLAINED = booleanPreferencesKey("explainer_shown")
        val SOURCE = stringPreferencesKey("source")
        val PLAYER_ITEMS = stringPreferencesKey("player_items")
        val SOUND_CHECK = stringPreferencesKey("sound_check")
    }
}
