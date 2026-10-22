package io.github.jacobbalane.veyl.link

import io.github.jacobbalane.veyl.core.link.*
import io.github.jacobbalane.veyl.runtime.*
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.*
import org.junit.Test

class LinkServerTest {
    @Test
    fun occupiedPortFallsBackBusyPeerGetsAnErrorAndCloseReleasesTheListener() {
        val accepted = CountDownLatch(1)
        val closed = CountDownLatch(1)
        val store =
            object : PeerStore {
                override fun list() = emptyList<SavedPc>()

                override fun find(id: ByteArray): PairedPeer? = null

                override fun remember(peer: PairedPeer) = error("Unexpected pairing.")

                override fun seen(id: ByteArray) {}

                override fun forget(id: ByteArray) {}
            }
        val callbacks =
            object : PhoneLinkCallbacks {
                override fun canPair() = false

                override fun ready() = JsonObject(emptyMap())

                override fun capture() = LinkCapture(0, null, JsonObject(emptyMap()))

                override fun stats(queueMs: Int, dropped: Long) = JsonObject(emptyMap())

                override fun closed(session: LinkSession) {
                    closed.countDown()
                }
            }
        // Reserve an ephemeral port rather than interfering with an installed app's default port.
        ServerSocket(0, 1, InetAddress.getByName("0.0.0.0")).use { occupied ->
            val server =
                LinkServer(
                    { socket ->
                        LinkSession(
                                socket,
                                Identity(ByteArray(16) { 1 }, "Test phone", "Test", "test"),
                                store,
                                PairingRateLimit(),
                                { System.nanoTime() / 1_000_000 },
                                callbacks,
                            )
                            .also { accepted.countDown() }
                    },
                    {},
                    occupied.localPort,
                )
            server.use {
                server.start()
                val port = server.port
                assertTrue(port > 0)
                assertNotEquals(occupied.localPort, port)
                Socket("127.0.0.1", port).use { first ->
                    first.soTimeout = 3000
                    assertTrue(accepted.await(2, TimeUnit.SECONDS))
                    Socket("127.0.0.1", port).use { second ->
                        second.soTimeout = 3000
                        Frames.write(second.getOutputStream(), Messages.error("internal"))
                        val response = Messages.parseHandshake(Frames.read(second.getInputStream()))
                        assertEquals("busy", response.text("code"))
                        assertEquals(-1, second.getInputStream().read())
                    }
                    server.close()
                    assertEquals(-1, first.getInputStream().read())
                    assertTrue(closed.await(2, TimeUnit.SECONDS))
                    assertEquals(0, server.port)
                    ServerSocket(port, 1, InetAddress.getByName("0.0.0.0")).use { rebound ->
                        assertEquals(port, rebound.localPort)
                    }
                }
            }
        }
    }
}
