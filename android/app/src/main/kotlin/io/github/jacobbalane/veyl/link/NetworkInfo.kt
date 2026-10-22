package io.github.jacobbalane.veyl.link

import android.content.Context
import android.net.*
import java.net.Inet4Address
import java.util.concurrent.ConcurrentHashMap

data class LocalNetwork(val ipv4: String?, val network: Network?)

class NetworkInfo(context: Context, private val changed: (LocalNetwork) -> Unit) : AutoCloseable {
    private val manager = context.getSystemService(ConnectivityManager::class.java)
    private val candidates = ConcurrentHashMap<Network, LinkProperties>()
    private val callback =
        object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                manager.getLinkProperties(network)?.let { candidates[network] = it }
                refresh()
            }

            override fun onLinkPropertiesChanged(network: Network, properties: LinkProperties) {
                candidates[network] = properties
                refresh()
            }

            override fun onLost(network: Network) {
                candidates.remove(network)
                refresh()
            }
        }

    fun start() {
        manager.registerNetworkCallback(
            NetworkRequest.Builder()
                .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
                .addTransportType(NetworkCapabilities.TRANSPORT_ETHERNET)
                .build(),
            callback,
        )
    }

    private fun refresh() {
        val selected =
            candidates.entries
                .sortedBy { if (it.key == manager.activeNetwork) 0 else 1 }
                .firstNotNullOfOrNull { entry ->
                    entry.value.linkAddresses
                        .map { it.address }
                        .filterIsInstance<Inet4Address>()
                        .firstOrNull { !it.isLoopbackAddress && !it.isAnyLocalAddress }
                        ?.let { LocalNetwork(it.hostAddress, entry.key) }
                } ?: LocalNetwork(null, null)
        changed(selected)
    }

    override fun close() {
        manager.unregisterNetworkCallback(callback)
        candidates.clear()
    }
}
