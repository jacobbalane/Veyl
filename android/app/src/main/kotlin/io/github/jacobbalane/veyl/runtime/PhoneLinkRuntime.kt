package io.github.jacobbalane.veyl.runtime

import android.Manifest
import android.app.AppOpsManager
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioManager
import android.os.*
import androidx.core.content.ContextCompat
import io.github.jacobbalane.veyl.BuildConfig
import io.github.jacobbalane.veyl.capture.Notifications
import io.github.jacobbalane.veyl.core.link.*
import io.github.jacobbalane.veyl.core.util.Base64Url
import io.github.jacobbalane.veyl.core.util.Utf8
import io.github.jacobbalane.veyl.link.*
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch
import kotlinx.serialization.json.*

data class PhoneMeter(val score: Int, val label: String, val tip: String)

data class PhoneLinkState(
    val pcName: String? = null,
    val address: String? = null,
    val permission: Boolean = true,
    val pairing: PairingPrompt? = null,
    val pcs: List<SavedPc> = emptyList(),
    val meter: PhoneMeter? = null,
    /** Pokes the connected PC reported; each one flashes the listening screen. */
    val pokes: Int = 0,
    val pairedName: String? = null,
    val error: String? = null,
)

/** Owns platform networking; urgent audio callers only offer an already filtered block. */
class PhoneLinkRuntime(private val context: Context, private val runtime: VeylRuntime) {
    private val control = Executors.newSingleThreadExecutor { task ->
        Thread(task, "Veyl-network-control").apply { isDaemon = true }
    }
    private val mutable = MutableStateFlow(PhoneLinkState(permission = permissionGranted()))
    val state = mutable.asStateFlow()
    private val foregroundActivities = AtomicInteger()
    private val pairingForeground = AtomicBoolean(false)
    private val service = AtomicBoolean(false)
    private val store = PairingStore.android(context)
    private val limiter = PairingRateLimit()
    private val connected = AtomicReference<LinkSession?>()
    private val descriptor = AtomicReference(describe(runtime.capture.value))
    private var network = LocalNetwork(null, null)
    private val advertiser = NsdAdvertiser(context, runtime::noteLink)
    private val networkInfo =
        NetworkInfo(context) { next ->
            control.execute {
                if (network != next) {
                    connected.get()?.close()
                    network = next
                    reconcile()
                }
            }
        }
    private var watching = false
    private var server: LinkServer? = null
    private var previousName: String? = null
    private var lastCpu = Process.getElapsedCpuTime()
    private var lastCpuAt = SystemClock.elapsedRealtime()
    private var lastStartRequest = -5000L
    private val startGate = StartRequestGate(SystemClock::elapsedRealtime)
    private val permissionWatcher = AppOpsManager.OnOpChangedListener { _, name ->
        if (name == null || name == context.packageName) refreshPermissions()
    }
    private val callbacks =
        object : PhoneLinkCallbacks {
            override fun canPair() = pairingForeground.get()

            override fun ready(): JsonObject {
                captureChanged()
                val settings = runtime.settings.state.value
                return Messages.objectOf(
                    "ready",
                    "device" to
                        buildJsonObject {
                            put("phone_name", settings.phoneName)
                            put("manufacturer", Build.MANUFACTURER)
                            put("model", Build.MODEL)
                            put("android_sdk", Build.VERSION.SDK_INT)
                            put("app_version", BuildConfig.VERSION_NAME)
                        },
                    "capture" to JsonObject(descriptor.get().state - "t"),
                )
            }

            override fun capture(): LinkCapture {
                captureChanged()
                return descriptor.get()
            }

            override fun stats(queueMs: Int, dropped: Long): JsonObject {
                val now = SystemClock.elapsedRealtime()
                val cpu = Process.getElapsedCpuTime()
                val percent =
                    ((cpu - lastCpu).toDouble() / maxOf(1, now - lastCpuAt) * 100 /
                            maxOf(1, Runtime.getRuntime().availableProcessors()))
                        .coerceIn(0.0, 100.0)
                lastCpu = cpu
                lastCpuAt = now
                val audio = context.getSystemService(AudioManager::class.java)
                val battery = context.getSystemService(BatteryManager::class.java)
                val thermal =
                    if (Build.VERSION.SDK_INT >= 29)
                        context.getSystemService(PowerManager::class.java).currentThermalStatus
                    else 0
                return Messages.objectOf(
                    "stats",
                    "queue_ms" to JsonPrimitive(queueMs),
                    "dropped_blocks" to JsonPrimitive(dropped),
                    "media_volume" to
                        JsonPrimitive(
                            audio.getStreamVolume(AudioManager.STREAM_MUSIC).toDouble() /
                                maxOf(1, audio.getStreamMaxVolume(AudioManager.STREAM_MUSIC)),
                        ),
                    "battery_pct" to
                        JsonPrimitive(
                            battery
                                .getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY)
                                .coerceIn(0, 100),
                        ),
                    "charging" to JsonPrimitive(battery.isCharging),
                    "thermal" to
                        JsonPrimitive(
                            listOf(
                                    "none",
                                    "light",
                                    "moderate",
                                    "severe",
                                    "critical",
                                    "emergency",
                                    "shutdown",
                                )
                                .getOrElse(thermal) { "none" },
                        ),
                    "cpu_pct" to JsonPrimitive(percent),
                )
            }

            override fun pairing(prompt: PairingPrompt?) {
                mutable.update { it.copy(pairing = prompt) }
            }

            override fun paired(peer: Peer) {
                runtime.noteLink("pairing_complete")
                mutable.update { it.copy(pairedName = peer.name, error = null) }
                control.execute { reloadPeers() }
            }

            override fun connected(session: LinkSession, peer: Peer) {
                connected.set(session)
                runtime.pcConnected = true
                captureChanged()
                mutable.update {
                    it.copy(
                        pcName = peer.name,
                        pairedName = null,
                        error = null,
                        meter = null,
                        pokes = 0,
                    )
                }
                runtime.noteLink("link_connected")
                if (service.get()) Notifications.updateConnected(context, peer.name)
                control.execute { reloadPeers() }
            }

            override fun closed(session: LinkSession) {
                if (connected.compareAndSet(session, null)) {
                    runtime.pcConnected = false
                    mutable.update { it.copy(pcName = null, meter = null, pokes = 0) }
                    runtime.noteLink("link_disconnected")
                    if (service.get()) Notifications.updateConnected(context, null)
                    startGate.revoke(session)
                }
            }

            override fun meter(value: JsonObject) {
                mutable.update {
                    it.copy(
                        meter =
                            PhoneMeter(
                                value.unsigned("score", 255u).toInt().coerceIn(0, 100),
                                value.text("label"),
                                value.text("tip"),
                            ),
                    )
                }
            }

            override fun gesture(value: JsonObject) {
                if (runCatching { value.text("kind") }.getOrNull() == "poke")
                    mutable.update { it.copy(pokes = it.pokes + 1) }
            }

            override fun requestStart() {
                val now = SystemClock.elapsedRealtime()
                if (
                    now - lastStartRequest >= 5000 &&
                        runtime.capture.value.state !in listOf("starting", "running")
                ) {
                    lastStartRequest = now
                    state.value.pcName?.let { name ->
                        connected.get()?.let { owner ->
                            Notifications.requestStart(context, name, startGate.issue(owner))
                        }
                    }
                }
            }

            override fun note(reason: String) {
                runtime.noteLink(reason)
                if (reason == "pairing_failed") mutable.update { it.copy(error = reason) }
            }
        }

    init {
        if (Build.VERSION.SDK_INT >= 37)
            AppOpsManager.permissionToOp(Manifest.permission.ACCESS_LOCAL_NETWORK)?.let { operation
                ->
                context
                    .getSystemService(AppOpsManager::class.java)
                    .startWatchingMode(operation, context.packageName, permissionWatcher)
            }
        runtime.scope.launch {
            runtime.settings.state.collect { settings ->
                control.execute {
                    captureChanged()
                    if (previousName != null && previousName != settings.phoneName)
                        connected
                            .get()
                            ?.sendControl(
                                Messages.objectOf(
                                    "phone_info",
                                    "phone_name" to JsonPrimitive(settings.phoneName),
                                ),
                            )
                    previousName = settings.phoneName
                    reconcile()
                }
            }
        }
        control.execute { reloadPeers() }
    }

    fun activityStarted() {
        foregroundActivities.incrementAndGet()
        control.execute { reconcile() }
    }

    fun activityStopped() {
        foregroundActivities.updateAndGet { maxOf(0, it - 1) }
        pairingForeground.set(false)
        control.execute { reconcile() }
    }

    fun pairingForeground(value: Boolean) {
        pairingForeground.set(value)
    }

    fun serviceRunning(value: Boolean) {
        service.set(value)
        control.execute { reconcile() }
    }

    fun refreshPermissions() {
        control.execute { reconcile() }
    }

    fun consumeStartToken(token: String?): Boolean = startGate.consume(token, connected.get())

    fun confirm(attempt: Long, matches: Boolean) {
        control.execute { server?.current()?.confirm(attempt, matches) }
    }

    fun forget(id: String) {
        control.execute {
            try {
                val bytes = Base64Url.decode(id)
                store.forget(bytes)
                if (connected.get()?.peerId?.contentEquals(bytes) == true)
                    connected.get()?.disconnect("forgotten")
                reloadPeers()
            } catch (_: Exception) {
                mutable.update { it.copy(error = "pair_store_write_failed") }
                runtime.noteLink("pair_store_write_failed")
            }
        }
    }

    fun audio(id: Long, block: AudioBlock) {
        val session = connected.get() ?: return
        var capture = descriptor.get()
        if (capture.runId != id || capture.config == null) {
            captureChanged()
            capture = descriptor.get()
        }
        if (capture.runId == id) session.audio(id, block)
    }

    @Synchronized
    fun captureChanged() {
        val next = describe(runtime.capture.value)
        descriptor.set(next)
        connected.get()?.updateCapture(next)
    }

    private fun permissionGranted(): Boolean =
        Build.VERSION.SDK_INT < 37 ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_LOCAL_NETWORK) ==
                PackageManager.PERMISSION_GRANTED

    private fun reconcile() {
        val permission = permissionGranted()
        mutable.update { it.copy(permission = permission) }
        val enabled =
            (foregroundActivities.get() > 0 || service.get()) &&
                permission &&
                runtime.settings.state.value.phoneId.isNotEmpty()
        if (!enabled) {
            advertiser.update(null)
            server?.close()
            server = null
            if (watching) {
                networkInfo.close()
                watching = false
                network = LocalNetwork(null, null)
            }
            mutable.update { it.copy(address = null) }
            return
        }
        if (!watching) {
            networkInfo.start()
            watching = true
        }
        val settings = runtime.settings.state.value
        if (server == null) {
            val next =
                LinkServer(
                    { socket ->
                        LinkSession(
                            socket,
                            Identity(
                                Base64Url.decode(runtime.settings.state.value.phoneId),
                                runtime.settings.state.value.phoneName,
                                Utf8.prefix(Build.MODEL),
                                "veyl-android/${BuildConfig.VERSION_NAME}",
                            ),
                            store,
                            limiter,
                            SystemClock::elapsedRealtime,
                            callbacks,
                        )
                    },
                    runtime::noteLink,
                )
            try {
                next.start()
                server = next
            } catch (_: Exception) {
                runtime.noteLink("link_bind_failed")
                return
            }
        }
        val port = server!!.port
        mutable.update { it.copy(address = network.ipv4?.let { ip -> "$ip:$port" }) }
        advertiser.update(
            network.ipv4?.let {
                Advertisement(
                    settings.phoneId,
                    settings.phoneName,
                    Build.MODEL,
                    port,
                    network.network,
                )
            },
        )
    }

    private fun reloadPeers() {
        try {
            mutable.update { it.copy(pcs = store.list()) }
        } catch (_: Exception) {
            mutable.update { it.copy(error = "pair_store_read_failed") }
            runtime.noteLink("pair_store_read_failed")
        }
    }

    private fun describe(snapshot: CaptureSnapshot): LinkCapture {
        val settings = runtime.settings.state.value
        val capture =
            Messages.objectOf(
                "capture_state",
                "state" to JsonPrimitive(snapshot.state),
                "source" to
                    JsonPrimitive(
                        if (snapshot.state == "idle") settings.source else snapshot.source,
                    ),
                "route" to JsonPrimitive(snapshot.route),
                "reason" to
                    (if (snapshot.micSilenced) JsonPrimitive("mic_silenced")
                    else snapshot.reason?.let(::JsonPrimitive) ?: JsonNull),
            )
        val info = snapshot.info
        val config =
            if (snapshot.state != "running" || info == null) null
            else
                Messages.objectOf(
                    "audio_config",
                    "run_id" to JsonPrimitive(snapshot.status.runId),
                    "source" to JsonPrimitive(snapshot.source),
                    "sample_rate" to JsonPrimitive(info.rate),
                    "frames_per_block" to JsonPrimitive(info.rate / 100),
                    "ref_channels" to JsonPrimitive(2),
                    "mic_channels" to JsonPrimitive(info.micChannels),
                    "mic_source" to JsonPrimitive(info.micSource),
                    "effects_off" to JsonArray(info.effectsOff.map(::JsonPrimitive)),
                    "hp_cutoff_hz" to JsonPrimitive(info.cutoff),
                    "mics" to
                        JsonArray(
                            info.microphones.map { mic ->
                                buildJsonObject {
                                    put("index", mic.index)
                                    put("type", mic.type)
                                    put(
                                        "position_m",
                                        mic.positionM?.let { JsonArray(it.map(::JsonPrimitive)) }
                                            ?: JsonNull,
                                    )
                                    put("channels", JsonArray(mic.channels.map(::JsonPrimitive)))
                                }
                            },
                        ),
                    "sound_check" to
                        (info.soundCheck?.let { check ->
                            buildJsonObject {
                                put("mic_source", check.source)
                                put("mic_channels", check.channels)
                                put("score", check.score)
                                put("mean_coherence", check.meanCoherence)
                            }
                        } ?: JsonNull),
                )
        return LinkCapture(snapshot.status.runId, config, capture)
    }
}
