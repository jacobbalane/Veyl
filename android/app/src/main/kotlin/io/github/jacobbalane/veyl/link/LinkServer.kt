package io.github.jacobbalane.veyl.link

import io.github.jacobbalane.veyl.core.link.Frames
import io.github.jacobbalane.veyl.core.link.Messages
import java.io.IOException
import java.net.*
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

class LinkServer(
    private val create: (Socket) -> LinkSession,
    private val note: (String) -> Unit,
    private val preferredPort: Int = 47820,
) : AutoCloseable {
    private val active = AtomicBoolean(false)
    private val session = AtomicReference<LinkSession?>()
    private var socket: ServerSocket? = null
    private var listener: Thread? = null
    val port: Int
        get() = socket?.localPort?.takeIf { it > 0 } ?: 0

    fun current(): LinkSession? = session.get()

    fun start() {
        if (!active.compareAndSet(false, true)) return
        val bound =
            try {
                bind(preferredPort)
            } catch (_: IOException) {
                try {
                    bind(0)
                } catch (failure: IOException) {
                    active.set(false)
                    throw failure
                }
            }
        socket = bound
        listener =
            Thread(
                    {
                        while (active.get()) {
                            try {
                                val peer = bound.accept()
                                if (!active.get() || peer.inetAddress !is Inet4Address) {
                                    peer.close()
                                    continue
                                }
                                if (session.get() != null) {
                                    rejectBusy(peer)
                                    continue
                                }
                                val next =
                                    try {
                                        create(peer)
                                    } catch (_: Exception) {
                                        peer.close()
                                        continue
                                    }
                                if (!active.get()) {
                                    next.close()
                                    continue
                                }
                                if (!session.compareAndSet(null, next)) {
                                    next.close()
                                    continue
                                }
                                if (!active.get()) {
                                    session.compareAndSet(next, null)
                                    next.close()
                                    continue
                                }
                                Thread(
                                        {
                                            try {
                                                next.run()
                                            } finally {
                                                session.compareAndSet(next, null)
                                            }
                                        },
                                        "Veyl-link-session",
                                    )
                                    .apply {
                                        isDaemon = true
                                        start()
                                    }
                            } catch (_: IOException) {
                                if (active.get()) {
                                    note("accept_failed")
                                    Thread.sleep(100)
                                }
                            }
                        }
                    },
                    "Veyl-link-accept",
                )
                .apply {
                    isDaemon = true
                    start()
                }
    }

    private fun bind(port: Int): ServerSocket =
        ServerSocket().apply {
            try {
                reuseAddress = true
                bind(InetSocketAddress(InetAddress.getByName("0.0.0.0"), port))
            } catch (failure: IOException) {
                close()
                throw failure
            }
        }

    private fun rejectBusy(peer: Socket) {
        peer.use {
            runCatching {
                it.tcpNoDelay = true
                it.soTimeout = 200
                Frames.write(it.getOutputStream(), Messages.error("busy"))
                it.shutdownOutput()
                Frames.read(it.getInputStream())
            }
        }
    }

    override fun close() {
        active.set(false)
        runCatching { socket?.close() }
        socket = null
        session.getAndSet(null)?.close()
        listener?.join(1000)
        listener = null
    }
}
