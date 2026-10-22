package io.github.jacobbalane.veyl.link

import android.content.Context
import android.net.Network
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.ext.SdkExtensions
import io.github.jacobbalane.veyl.core.util.Base64Url
import io.github.jacobbalane.veyl.core.util.Utf8

data class Advertisement(
    val id: String,
    val name: String,
    val model: String,
    val port: Int,
    val network: Network?,
)

/** All registrations and acknowledgements are serialized on the main looper. */
class NsdAdvertiser(context: Context, private val note: (String) -> Unit) {
    private val manager = context.getSystemService(NsdManager::class.java)
    private val wifi = context.applicationContext.getSystemService(WifiManager::class.java)
    private val main = Handler(Looper.getMainLooper())
    private var desired: Advertisement? = null
    private var current: Advertisement? = null
    private var listener: NsdManager.RegistrationListener? = null
    private var unregistering = false
    private var multicast: WifiManager.MulticastLock? = null
    private val retry = Runnable { reconcile() }

    fun update(value: Advertisement?) {
        main.post {
            desired = value
            reconcile()
        }
    }

    private fun reconcile() {
        main.removeCallbacks(retry)
        val registered = listener
        if (registered != null) {
            if (current != desired && !unregistering) {
                unregistering = true
                try {
                    manager.unregisterService(registered)
                } catch (_: IllegalArgumentException) {
                    listener = null
                    unregistering = false
                    current = null
                    reconcile()
                }
            }
            if (desired == null) releaseMulticast()
            return
        }
        val target =
            desired
                ?: run {
                    releaseMulticast()
                    return
                }
        if (
            Build.VERSION.SDK_INT < 33 ||
                SdkExtensions.getExtensionVersion(Build.VERSION_CODES.TIRAMISU) < 7
        ) {
            if (multicast == null)
                multicast =
                    wifi.createMulticastLock("veyl:discovery").apply { setReferenceCounted(false) }
            if (multicast?.isHeld == false) multicast?.acquire()
        }
        val shortId =
            Base64Url.decode(target.id).take(4).joinToString("") {
                "%02x".format(java.util.Locale.ROOT, it.toInt() and 255)
            }
        val service =
            NsdServiceInfo().apply {
                serviceName = "Veyl ${Utf8.prefix(target.name)} ${shortId.take(4)}"
                serviceType = "_veyl._tcp"
                port = target.port
                setAttribute("v", "1")
                setAttribute("id", shortId)
                setAttribute("name", Utf8.prefix(target.name))
                setAttribute("model", Utf8.prefix(target.model))
                if (Build.VERSION.SDK_INT >= 33) network = target.network
            }
        val next =
            object : NsdManager.RegistrationListener {
                override fun onServiceRegistered(info: NsdServiceInfo) {
                    main.post { if (listener === this) reconcile() }
                }

                override fun onRegistrationFailed(info: NsdServiceInfo, error: Int) {
                    main.post {
                        if (listener === this) {
                            listener = null
                            current = null
                            unregistering = false
                            note("nsd_registration_failed")
                            releaseMulticast()
                            if (desired != null) main.postDelayed(retry, 2000)
                        }
                    }
                }

                override fun onServiceUnregistered(info: NsdServiceInfo) {
                    main.post {
                        if (listener === this) {
                            listener = null
                            current = null
                            unregistering = false
                            reconcile()
                        }
                    }
                }

                override fun onUnregistrationFailed(info: NsdServiceInfo, error: Int) {
                    main.post {
                        if (listener === this) {
                            unregistering = false
                            note("nsd_unregister_failed")
                            if (desired == null) releaseMulticast()
                            main.postDelayed(retry, 2000)
                        }
                    }
                }
            }
        current = target
        listener = next
        try {
            manager.registerService(service, NsdManager.PROTOCOL_DNS_SD, next)
        } catch (_: RuntimeException) {
            listener = null
            current = null
            releaseMulticast()
            note("nsd_registration_failed")
            if (desired != null) main.postDelayed(retry, 2000)
        }
    }

    private fun releaseMulticast() {
        multicast?.let { if (it.isHeld) runCatching { it.release() } }
        multicast = null
    }
}
