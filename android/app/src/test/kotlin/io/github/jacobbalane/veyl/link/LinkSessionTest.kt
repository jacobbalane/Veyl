package io.github.jacobbalane.veyl.link

import io.github.jacobbalane.veyl.core.audio.BlockPacker
import io.github.jacobbalane.veyl.core.link.*
import io.github.jacobbalane.veyl.core.util.Base64Url
import io.github.jacobbalane.veyl.runtime.*
import java.net.*
import java.util.concurrent.*
import kotlin.math.PI
import kotlin.math.sin
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

class LinkSessionTest {
    private class MemoryPeers : PeerStore, AutoCloseable {
        private val keys = ConcurrentHashMap<String, ByteArray>()

        override fun list() =
            keys.keys.map { SavedPc(it, "Desk", "2026-10-22T00:00:00Z", "2026-10-22T00:00:00Z") }

        override fun find(id: ByteArray) =
            keys[Base64Url.encode(id)]?.let {
                PairedPeer(Peer(id.copyOf(), "Desk", ""), Secret(it))
            }

        override fun remember(peer: PairedPeer) {
            keys.put(Base64Url.encode(peer.peer.id), peer.link.copyForProtectedStore())?.fill(0)
        }

        override fun seen(id: ByteArray) {}

        override fun forget(id: ByteArray) {
            keys.remove(Base64Url.encode(id))?.fill(0)
        }

        override fun close() {
            keys.values.forEach { it.fill(0) }
            keys.clear()
        }
    }

    private class Callbacks : PhoneLinkCallbacks {
        var foreground = true
        val prompts = LinkedBlockingQueue<PairingPrompt>()
        val connected = CountDownLatch(1)
        val paired = CountDownLatch(1)

        override fun canPair() = foreground

        override fun ready() =
            Messages.objectOf(
                "ready",
                "device" to
                    buildJsonObject {
                        put("phone_name", "Test phone")
                        put("manufacturer", "Test")
                        put("model", "Test")
                        put("android_sdk", 37)
                        put("app_version", "0.0.1")
                    },
                "capture" to capture().state.let { JsonObject(it - "t") },
            )

        override fun capture() =
            LinkCapture(
                7,
                Messages.objectOf(
                    "audio_config",
                    "run_id" to JsonPrimitive(7),
                    "source" to JsonPrimitive("test_sound"),
                    "sample_rate" to JsonPrimitive(48000),
                    "frames_per_block" to JsonPrimitive(480),
                    "ref_channels" to JsonPrimitive(2),
                    "mic_channels" to JsonPrimitive(1),
                    "mic_source" to JsonPrimitive("unprocessed"),
                    "effects_off" to JsonArray(emptyList()),
                    "hp_cutoff_hz" to JsonPrimitive(8000),
                    "mics" to JsonArray(emptyList()),
                    "sound_check" to JsonNull,
                ),
                Messages.objectOf(
                    "capture_state",
                    "state" to JsonPrimitive("running"),
                    "source" to JsonPrimitive("test_sound"),
                    "route" to JsonPrimitive("speaker"),
                    "reason" to JsonNull,
                ),
            )

        override fun stats(queueMs: Int, dropped: Long) =
            Messages.objectOf(
                "stats",
                "queue_ms" to JsonPrimitive(queueMs),
                "dropped_blocks" to JsonPrimitive(dropped),
                "media_volume" to JsonPrimitive(0.5),
                "battery_pct" to JsonPrimitive(50),
                "charging" to JsonPrimitive(false),
                "thermal" to JsonPrimitive("none"),
                "cpu_pct" to JsonPrimitive(1.0),
            )

        override fun pairing(prompt: PairingPrompt?) {
            if (prompt != null) prompts.offer(prompt)
        }

        override fun paired(peer: Peer) {
            paired.countDown()
        }

        override fun connected(session: LinkSession, peer: Peer) {
            connected.countDown()
        }
    }

    private class Connection(store: PeerStore, val callbacks: Callbacks) : AutoCloseable {
        private val listener = ServerSocket(0, 1, InetAddress.getLoopbackAddress())
        private val origin = System.nanoTime()
        private val accepted = CountDownLatch(1)
        lateinit var phone: LinkSession
        private val worker = Thread {
            val socket = listener.accept()
            phone =
                LinkSession(
                    socket,
                    Identity(ByteArray(16) { 9 }, "Test phone", "Test", "veyl-android/0.0.1"),
                    store,
                    PairingRateLimit(),
                    { (System.nanoTime() - origin) / 1_000_000 },
                    callbacks,
                )
            accepted.countDown()
            phone.run()
        }
            .apply {
                isDaemon = true
                start()
            }
        val pc =
            Socket(InetAddress.getLoopbackAddress(), listener.localPort).apply {
                soTimeout = 3000
                tcpNoDelay = true
            }

        init {
            check(accepted.await(2, TimeUnit.SECONDS))
        }

        fun write(frame: Frame) = Frames.write(pc.getOutputStream(), frame)

        fun read() = Frames.read(pc.getInputStream())

        override fun close() {
            pc.close()
            phone.close()
            listener.close()
            worker.join(2000)
            assertFalse(worker.isAlive)
        }
    }

    private fun pair(connection: Connection, id: ByteArray): Secret {
        Ephemeral.generate().use { ephemeral ->
            val hello =
                Messages.handshake(
                    Messages.objectOf(
                        "pair_hello",
                        "v" to JsonPrimitive(1),
                        "pc_id" to binary(id),
                        "pc_name" to JsonPrimitive("Desk"),
                        "app" to JsonPrimitive("veyl-desktop/0.0.1"),
                        "commit" to binary(Crypto.commit(ephemeral.public, ephemeral.nonce)),
                    ),
                )
            connection.write(hello)
            val reply = connection.read()
            val info = Messages.parseHandshake(reply)
            val reveal =
                Messages.handshake(
                    Messages.objectOf(
                        "pair_reveal",
                        "e_pub" to binary(ephemeral.public),
                        "nonce" to binary(ephemeral.nonce),
                    ),
                )
            connection.write(reveal)
            ephemeral.shared(info.binary("e_pub", 32)).use { shared ->
                PairKeys.derive(shared, hello.body, reply.body, reveal.body).use { keys ->
                    val prompt =
                        connection.callbacks.prompts.poll(2, TimeUnit.SECONDS)
                            ?: error("No pairing prompt.")
                    assertTrue(keys.sas() == prompt.code)
                    connection.write(
                        Messages.handshake(
                            Messages.objectOf(
                                "pair_confirm",
                                "mac" to binary(keys.confirmation(true)),
                            ),
                        ),
                    )
                    connection.phone.confirm(prompt.attempt + 1, true)
                    assertNull(connection.callbacks.prompts.poll(70, TimeUnit.MILLISECONDS))
                    connection.phone.confirm(true)
                    val confirmation = Messages.parseHandshake(connection.read())
                    assertTrue(
                        Crypto.matches(keys.confirmation(false), confirmation.binary("mac", 32)),
                    )
                    assertTrue(connection.callbacks.paired.await(2, TimeUnit.SECONDS))
                    val copy = keys.link.copyForProtectedStore()
                    return try {
                        Secret(copy)
                    } finally {
                        copy.fill(0)
                    }
                }
            }
        }
    }

    private fun open(connection: Connection, id: ByteArray, link: Secret): SealedChannel {
        Ephemeral.generate().use { ephemeral ->
            val hello =
                Messages.handshake(
                    Messages.objectOf(
                        "hello",
                        "v" to JsonPrimitive(1),
                        "pc_id" to binary(id),
                        "app" to JsonPrimitive("veyl-desktop/0.0.1"),
                        "e_pub" to binary(ephemeral.public),
                        "nonce" to binary(ephemeral.nonce),
                    ),
                )
            connection.write(hello)
            val reply = connection.read()
            val info = Messages.parseHandshake(reply)
            val channel =
                ephemeral.shared(info.binary("e_pub", 32)).use { shared ->
                    SessionKeys.derive(shared, link, hello.body, reply.body).use {
                        it.channel(true)
                    }
                }
            val ready = channel.open(connection.read())
            assertEquals("ready", Messages.parseControl(ready.payload).text("t"))
            connection.write(
                channel.seal(
                    Frames.CONTROL,
                    Messages.objectBytes(
                        Messages.objectOf(
                            "ready",
                            "pc" to
                                buildJsonObject {
                                    put("pc_name", "Desk")
                                    put("app_version", "0.0.1")
                                },
                            "want" to buildJsonObject { put("meter", true) },
                        ),
                    ),
                ),
            )
            assertTrue(connection.callbacks.connected.await(2, TimeUnit.SECONDS))
            assertEquals(
                "audio_config",
                Messages.parseControl(channel.open(connection.read()).payload).text("t"),
            )
            assertEquals(
                "capture_state",
                Messages.parseControl(channel.open(connection.read()).payload).text("t"),
            )
            return channel
        }
    }

    @Test
    fun realTcpPairsRekeysStreamsBothFilteredChannelsAndAnswersPing() {
        MemoryPeers().use { peers ->
            val id = ByteArray(16) { 3 }
            val link = Connection(peers, Callbacks()).use { pair(it, id) }
            link.use {
                Connection(peers, Callbacks()).use { connection ->
                    open(connection, id, link).use { channel ->
                        val packers = arrayOf(BlockPacker(0, 48000, 2), BlockPacker(1, 48000, 1))
                        for (index in 0 until 20) for (stream in 0..1) {
                            val channels = if (stream == 0) 2 else 1
                            val pcm =
                                ShortArray(480 * channels) {
                                    (sin(2 * PI * 12000 * (index * 480 + it / channels) / 48000) *
                                            6000)
                                        .toInt()
                                        .toShort()
                                }
                            connection.phone.audio(
                                7,
                                packers[stream].pack(pcm, index * 480L, index * 10_000_000L),
                            )
                        }
                        connection.write(channel.seal(Frames.PING, PingPong.encodePing(123456)))
                        val count = IntArray(2)
                        var pong = false
                        while (count.sum() < 40 || !pong) {
                            val packet = channel.open(connection.read())
                            if (packet.type == Frames.AUDIO) {
                                val block =
                                    AudioBlock.decode(
                                        byteArrayOf(Frames.AUDIO.toByte()) + packet.payload,
                                    )
                                assertEquals(count[block.stream].toLong(), block.seq)
                                assertEquals(48000, block.sampleRate)
                                count[block.stream]++
                            } else if (packet.type == Frames.PONG) {
                                assertEquals(123456L, PingPong.pong(packet.payload).first)
                                pong = true
                            }
                        }
                        assertEquals(20, count[0])
                        assertEquals(20, count[1])
                        connection.phone.disconnect("forgotten")
                        var bye = false
                        while (!bye) {
                            val packet = channel.open(connection.read())
                            if (packet.type == Frames.CONTROL)
                                bye = Messages.parseControl(packet.payload).text("t") == "bye"
                        }
                    }
                }
            }
        }
    }

    @Test
    fun backgroundPairingIsRefusedAndAnUnknownPcCannotStartASession() {
        MemoryPeers().use { peers ->
            Connection(peers, Callbacks().apply { foreground = false }).use { connection ->
                val hello =
                    Messages.handshake(
                        Messages.objectOf(
                            "pair_hello",
                            "v" to JsonPrimitive(1),
                            "pc_id" to binary(ByteArray(16)),
                            "pc_name" to JsonPrimitive("Desk"),
                            "app" to JsonPrimitive("test"),
                            "commit" to binary(ByteArray(32)),
                        ),
                    )
                connection.write(hello)
                assertEquals(
                    "not_in_foreground",
                    Messages.parseHandshake(connection.read()).text("reason"),
                )
                assertTrue(peers.list().isEmpty())
            }
            Connection(peers, Callbacks()).use { connection ->
                Ephemeral.generate().use { ephemeral ->
                    connection.write(
                        Messages.handshake(
                            Messages.objectOf(
                                "hello",
                                "v" to JsonPrimitive(1),
                                "pc_id" to binary(ByteArray(16)),
                                "app" to JsonPrimitive("test"),
                                "e_pub" to binary(ephemeral.public),
                                "nonce" to binary(ephemeral.nonce),
                            ),
                        ),
                    )
                    assertEquals(
                        "not_paired",
                        Messages.parseHandshake(connection.read()).text("code"),
                    )
                }
            }
        }
    }

    @Test
    fun stoppedAndReplacedRunsCannotBeRevivedByAnOldAudioCallback() {
        MemoryPeers().use { peers ->
            val id = ByteArray(16) { 4 }
            val link = Secret(ByteArray(32) { 8 })
            link.use {
                peers.remember(PairedPeer(Peer(id, "Desk", ""), link))
                Connection(peers, Callbacks()).use { connection ->
                    open(connection, id, link).use { channel ->
                        val old = BlockPacker(0, 48000, 2).pack(ShortArray(960), 0, 0)
                        val stopped =
                            JsonObject(
                                connection.callbacks.capture().state +
                                    ("state" to JsonPrimitive("stopped")),
                            )
                        connection.phone.updateCapture(LinkCapture(7, null, stopped))
                        connection.phone.audio(7, old)
                        assertEquals(
                            "stopped",
                            Messages.parseControl(channel.open(connection.read()).payload)
                                .text("state"),
                        )
                        val original = connection.callbacks.capture()
                        val nextConfig =
                            JsonObject(
                                original.config!! +
                                    mapOf(
                                        "run_id" to JsonPrimitive(8),
                                        "sample_rate" to JsonPrimitive(44100),
                                        "frames_per_block" to JsonPrimitive(441),
                                    ),
                            )
                        connection.phone.updateCapture(LinkCapture(8, nextConfig, original.state))
                        connection.phone.audio(7, old)
                        connection.phone.audio(
                            8,
                            BlockPacker(0, 44100, 2).pack(ShortArray(882), 0, 0),
                        )
                        val config = Messages.parseControl(channel.open(connection.read()).payload)
                        assertEquals(8uL, config.unsigned("run_id"))
                        assertEquals(
                            "capture_state",
                            Messages.parseControl(channel.open(connection.read()).payload)
                                .text("t"),
                        )
                        val packet = channel.open(connection.read())
                        assertEquals(Frames.AUDIO, packet.type)
                        assertEquals(
                            44100,
                            AudioBlock.decode(byteArrayOf(Frames.AUDIO.toByte()) + packet.payload)
                                .sampleRate,
                        )
                    }
                }
            }
        }
    }

    @Test
    fun aChangedAuthenticationTagClosesTheTcpSession() {
        MemoryPeers().use { peers ->
            val id = ByteArray(16) { 5 }
            Secret(ByteArray(32) { 9 }).use { link ->
                peers.remember(PairedPeer(Peer(id, "Desk", ""), link))
                Connection(peers, Callbacks()).use { connection ->
                    open(connection, id, link).use { channel ->
                        val frame = channel.seal(Frames.PING, PingPong.encodePing(1))
                        frame.body[frame.body.lastIndex] =
                            (frame.body.last().toInt() xor 1).toByte()
                        connection.write(frame)
                        val ended =
                            try {
                                connection.pc.getInputStream().read() == -1
                            } catch (_: SocketException) {
                                true
                            }
                        assertTrue(ended)
                    }
                }
            }
        }
    }
}
