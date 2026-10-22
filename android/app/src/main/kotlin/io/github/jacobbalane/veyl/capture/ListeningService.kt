package io.github.jacobbalane.veyl.capture

import android.app.*
import android.content.*
import android.content.pm.ServiceInfo
import android.media.AudioManager
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.net.wifi.WifiManager
import android.os.*
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import io.github.jacobbalane.veyl.BuildConfig
import io.github.jacobbalane.veyl.VeylApp
import io.github.jacobbalane.veyl.core.audio.NoPcDeadline
import io.github.jacobbalane.veyl.runtime.VeylRuntime
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

class ListeningService : Service() {
    private lateinit var runtime: VeylRuntime
    private val control = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())
    private val stopping = AtomicBoolean(false)
    private var run: CaptureRun? = null
    private var projection: MediaProjection? = null
    private var wake: PowerManager.WakeLock? = null
    private var wifi: WifiManager.WifiLock? = null
    private var route: AudioRoute? = null
    private var runId = 0L
    private var launched = false
    private var lab = false
    private lateinit var deadline: NoPcDeadline
    private var lockRefreshedMs = 0L
    private val screenOff =
        object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) {
                stop("screen_locked")
            }
        }
    private val projectionCallback =
        object : MediaProjection.Callback() {
            override fun onStop() {
                val locked =
                    getSystemService(KeyguardManager::class.java).isKeyguardLocked ||
                        !getSystemService(PowerManager::class.java).isInteractive
                stop(if (locked) "screen_locked" else "projection_stopped")
            }
        }
    private var thermal: PowerManager.OnThermalStatusChangedListener? = null
    private val liveness =
        object : Runnable {
            override fun run() {
                if (stopping.get()) return
                val now = SystemClock.elapsedRealtime()
                if (deadline.expired(now, runtime.pcConnected)) {
                    stop("no_pc_timeout")
                    return
                }
                control.execute {
                    if (!stopping.get() && now - lockRefreshedMs >= 300_000) {
                        wake?.acquire(660_000)
                        lockRefreshedMs = now
                    }
                }
                main.postDelayed(this, 1000)
            }
        }

    override fun onCreate() {
        super.onCreate()
        runtime = (application as VeylApp).runtime
        Notifications.create(this)
        ContextCompat.registerReceiver(
            this,
            screenOff,
            IntentFilter(Intent.ACTION_SCREEN_OFF),
            ContextCompat.RECEIVER_NOT_EXPORTED,
        )
        route =
            AudioRoute(getSystemService(AudioManager::class.java), runtime::route).also {
                it.start()
            }
        if (Build.VERSION.SDK_INT >= 29) {
            thermal = PowerManager.OnThermalStatusChangedListener {
                if (it >= PowerManager.THERMAL_STATUS_CRITICAL) stop("thermal")
            }
            getSystemService(PowerManager::class.java)
                .addThermalStatusListener(mainExecutor, thermal!!)
        }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == STOP) {
            stop("user")
            return START_NOT_STICKY
        }
        if (intent == null || launched) return START_NOT_STICKY
        launched = true
        lab = BuildConfig.DEBUG && intent.getBooleanExtra("lab", false)
        runId = runtime.starting("capture")
        deadline = NoPcDeadline(SystemClock.elapsedRealtime(), lab)
        try {
            val types =
                if (Build.VERSION.SDK_INT >= 30)
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION or
                        ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE
                else if (Build.VERSION.SDK_INT >= 29)
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION
                else 0
            ServiceCompat.startForeground(
                this,
                Notifications.LISTENING_ID,
                Notifications.listening(this),
                types,
            )
        } catch (_: RuntimeException) {
            stop("permission")
            return START_NOT_STICKY
        }
        val data = consentData(intent)
        control.execute {
            try {
                if (stopping.get()) return@execute
                if (
                    Build.VERSION.SDK_INT < 29 ||
                        data == null ||
                        intent.getIntExtra("result", Activity.RESULT_CANCELED) != Activity.RESULT_OK
                )
                    throw CaptureFailure("capture_error")
                val next =
                    getSystemService(MediaProjectionManager::class.java)
                        .getMediaProjection(Activity.RESULT_OK, data)
                        ?: throw CaptureFailure("capture_error")
                projection = next
                next.registerCallback(projectionCallback, main)
                acquireLocks()
                val capture = CaptureRun(runtime, runId, CaptureDebug.tap(this), ::stop)
                run = capture
                capture.start(getSystemService(AudioManager::class.java), next)
                main.post(liveness)
            } catch (_: SecurityException) {
                stop("permission")
            } catch (failure: CaptureFailure) {
                stop(failure.reason)
            } catch (_: RuntimeException) {
                stop("capture_error")
            }
        }
        return START_NOT_STICKY
    }

    @Suppress("DEPRECATION")
    private fun consentData(intent: Intent): Intent? =
        if (Build.VERSION.SDK_INT >= 33) intent.getParcelableExtra("consent", Intent::class.java)
        else intent.getParcelableExtra("consent")

    @Suppress("DEPRECATION")
    private fun acquireLocks() {
        wake =
            getSystemService(PowerManager::class.java)
                .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "veyl:listening")
                .also {
                    it.setReferenceCounted(false)
                    it.acquire(660_000)
                }
        lockRefreshedMs = SystemClock.elapsedRealtime()
        val manager = applicationContext.getSystemService(WifiManager::class.java)
        wifi =
            manager
                .createWifiLock(
                    if (Build.VERSION.SDK_INT >= 29) WifiManager.WIFI_MODE_FULL_LOW_LATENCY
                    else WifiManager.WIFI_MODE_FULL_HIGH_PERF,
                    "veyl:link",
                )
                .also { it.acquire() }
    }

    private fun cleanup() {
        runCatching { run?.close() }
        run = null
        runCatching { projection?.unregisterCallback(projectionCallback) }
        runCatching { projection?.stop() }
        projection = null
        runCatching { wake?.let { if (it.isHeld) it.release() } }
        wake = null
        runCatching { wifi?.let { if (it.isHeld) it.release() } }
        wifi = null
    }

    private fun stop(reason: String) {
        if (!stopping.compareAndSet(false, true)) return
        main.removeCallbacks(liveness)
        if (control.isShutdown) return
        control.execute {
            cleanup()
            runtime.stopped(runId, reason)
            main.post {
                if (reason != "user") Notifications.alert(this, reason)
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
            }
        }
    }

    override fun onDestroy() {
        stop("user")
        main.removeCallbacks(liveness)
        route?.close()
        unregisterReceiver(screenOff)
        if (Build.VERSION.SDK_INT >= 29)
            thermal?.let {
                getSystemService(PowerManager::class.java).removeThermalStatusListener(it)
            }
        control.shutdown()
        super.onDestroy()
    }

    companion object {
        const val STOP = "io.github.jacobbalane.veyl.STOP"

        fun start(context: Context, result: Int, data: Intent, laboratory: Boolean = false) {
            ContextCompat.startForegroundService(
                context,
                Intent(context, ListeningService::class.java)
                    .setAction("capture")
                    .putExtra("result", result)
                    .putExtra("consent", data)
                    .putExtra("lab", laboratory),
            )
        }

        fun stop(context: Context) {
            context.startService(Intent(context, ListeningService::class.java).setAction(STOP))
        }
    }
}
