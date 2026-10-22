package io.github.jacobbalane.veyl.core.link

import java.nio.ByteBuffer
import kotlin.test.*
import kotlinx.serialization.json.*
import org.junit.Test

class ProtocolFailureTest {
    @Test
    fun authenticatedMalformedAudioReturnsOneFinalErrorBeforeClose() {
        val plain = fixture().obj("audio").hex("plain")
        plain[2] = 0
        val length = 1 + 8 + plain.size + 16
        val aad =
            ByteBuffer.allocate(13).putInt(length).put(Frames.SEALED.toByte()).putLong(0).array()
        val cipher = javax.crypto.Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(
            javax.crypto.Cipher.ENCRYPT_MODE,
            javax.crypto.spec.SecretKeySpec(ByteArray(32) { 3 }, "AES"),
            javax.crypto.spec.GCMParameterSpec(128, ByteArray(12)),
        )
        cipher.updateAAD(aad)
        val frame = Frame(Frames.SEALED, ByteArray(8) + cipher.doFinal(plain))
        channel().use { receiver ->
            channel().use { peer ->
                assertFails { receiver.open(frame) }
                assertTrue(receiver.closed)
                val final = receiver.takeCloseFrame()!!
                assertNull(receiver.takeCloseFrame())
                val packet = peer.open(final)
                assertEquals(Frames.CONTROL, packet.type)
                assertEquals("bad_message", Messages.parseControl(packet.payload).text("code"))
            }
        }
    }

    private fun channel(key: Int = 3): SealedChannel =
        Secret(ByteArray(32) { key.toByte() }).use { SealedChannel(it, it) }

    private fun pair(): Pair<PhonePairing, JsonObject> {
        val vector = fixture().obj("pair")
        val bodies = vector["bodies"]!!.jsonArray
        val phone =
            PhonePairing(identity(), 0, true, false, PairingRateLimit()) {
                pairingKey(vector, "phone")
            }
        phone.receive(Frame(Frames.HANDSHAKE, body(bodies[0])), 0)
        phone.receive(Frame(Frames.HANDSHAKE, body(bodies[2])), 1)
        return phone to vector
    }

    @Test
    fun corruptionSkippedCountersWrongKeysAndReplayCloseBothDirections() {
        for (mutation in 0..4) {
            channel().use { sender ->
                channel(if (mutation == 4) 4 else 3).use { receiver ->
                    val original = sender.seal(Frames.PING, PingPong.encodePing(1))
                    val bytes = original.body.copyOf()
                    val frame =
                        when (mutation) {
                            0 -> {
                                bytes[bytes.lastIndex] = (bytes.last().toInt() xor 1).toByte()
                                Frame(Frames.SEALED, bytes)
                            }
                            1 -> {
                                bytes[7] = 1
                                Frame(Frames.SEALED, bytes)
                            }
                            2 -> Frame(Frames.SEALED, bytes.copyOf(24))
                            3 -> Frame(Frames.HANDSHAKE, bytes)
                            else -> original
                        }
                    assertFails { receiver.open(frame) }
                    assertTrue(receiver.closed)
                    assertFails { receiver.open(original) }
                    assertFails { receiver.seal(Frames.PING, PingPong.encodePing(1)) }
                }
            }
        }
        channel().use { sender ->
            channel().use { receiver ->
                val frame = sender.seal(Frames.PING, PingPong.encodePing(1))
                receiver.open(frame)
                assertFails { receiver.open(frame) }
                assertTrue(receiver.closed)
            }
        }
    }

    @Test
    fun strictBoundariesUnknownExtensionsAndMalformedMessages() {
        channel().use { sender ->
            channel().use { receiver ->
                val control = "{\"t\":\"request_start\",\"future\":true}".toByteArray()
                val full = control + ByteArray(Frames.MAX_CONTROL - control.size) { 32 }
                receiver.open(sender.seal(Frames.CONTROL, full))
                val unknown = ByteArray(Frames.MAX_FRAME - 26)
                val frame = sender.seal(0xee, unknown)
                assertEquals(Frames.MAX_FRAME, frame.body.size + 1)
                same(unknown, receiver.open(Frames.decode(Frames.encode(frame))).payload)
            }
        }
        channel().use {
            assertFails { it.seal(Frames.CONTROL, ByteArray(Frames.MAX_CONTROL + 1)) }
            assertTrue(it.closed)
        }
        channel().use {
            assertFails { it.seal(0xee, ByteArray(Frames.MAX_FRAME - 25)) }
            assertTrue(it.closed)
        }
        for (length in listOf(0, -1, Frames.MAX_FRAME + 1)) {
            assertFails { Frames.decode(ByteBuffer.allocate(4).putInt(length).array()) }
        }
        for (bytes in
            listOf(
                "[]".toByteArray(),
                byteArrayOf(0xef.toByte(), 0xbb.toByte(), 0xbf.toByte()) + "{}".toByteArray(),
                byteArrayOf(0xff.toByte()),
            )) {
            assertFails { Messages.parseControl(bytes) }
        }
        assertEquals(
            "future_control",
            Messages.parseControl("{\"t\":\"future_control\",\"extra\":true}".toByteArray())
                .text("t"),
        )
        val hello =
            Json.parseToJsonElement(
                    fixture().obj("pair")["bodies"]!!.jsonArray[0].jsonPrimitive.content,
                )
                .jsonObject
        for (bad in listOf("AQ==", "AQ", "!", "AAAAAAAAAAAAAAAAAAAAAB")) {
            val malformed = JsonObject(hello + ("pc_id" to JsonPrimitive(bad)))
            assertFails {
                Messages.parseHandshake(Frame(Frames.HANDSHAKE, Messages.objectBytes(malformed)))
            }
        }
        val original = fixture().obj("audio").hex("plain")
        for ((offset, value) in
            listOf(
                1 to 2,
                2 to 0,
                2 to 5,
                3 to 2,
                4 to 0,
                28 to 0,
                29 to 17,
                30 to 128,
                35 to 127,
            )) {
            val malformed = original.copyOf()
            malformed[offset] = value.toByte()
            assertFails { AudioBlock.decode(malformed) }
        }
        assertFails { PingPong.ping(ByteArray(7)) }
        assertFails { PingPong.pong(ByteArray(17)) }
    }

    @Test
    fun pairingRejectsMismatchBadMacCancellationAndTimeoutWithoutStoring() {
        val vector = fixture().obj("pair")
        val bodies = vector["bodies"]!!.jsonArray
        PhonePairing(identity(), 0, true, false, PairingRateLimit()).use { phone ->
            phone.receive(Frame(Frames.HANDSHAKE, body(bodies[0])), 0)
            val reveal = Messages.parseHandshake(Frame(Frames.HANDSHAKE, body(bodies[2])))
            val changed = JsonObject(reveal + ("nonce" to binary(ByteArray(32) { 9 })))
            phone.receive(Messages.handshake(changed), 1)
            assertEquals("mismatch", phone.failure)
            assertNull(phone.takePairing())
        }
        pair().first.use { phone ->
            phone.receive(
                Messages.handshake(
                    Messages.objectOf("pair_confirm", "mac" to binary(ByteArray(32))),
                ),
                2,
            )
            assertEquals("bad_mac", phone.failure)
            assertNull(phone.takePairing())
        }
        pair().first.use { phone ->
            phone.confirm(false, 2)
            assertEquals("user_cancelled", phone.failure)
            assertNull(phone.takePairing())
        }
        pair().first.use { phone ->
            phone.tick(120_001)
            assertEquals("timeout", phone.failure)
            assertNull(phone.sas)
        }
        PhonePairing(identity(), 0, true, false, PairingRateLimit()).use { phone ->
            phone.tick(150_000)
            assertEquals("timeout", phone.failure)
        }
        Ephemeral.fromSeed(ByteArray(32) { 1 }, ByteArray(32)).use {
            assertFails { it.shared(ByteArray(32)) }
        }
    }

    @Test
    fun foregroundBusyVersionAndRateLimitAreEnforcedAcrossConnections() {
        val hello = Frame(Frames.HANDSHAKE, body(fixture().obj("pair")["bodies"]!!.jsonArray[0]))
        val limiter = PairingRateLimit()
        for ((foreground, busy, expected) in
            listOf(Triple(false, false, "not_in_foreground"), Triple(true, true, "busy"))) {
            PhonePairing(identity(), 0, foreground, busy, limiter).use { phone ->
                assertEquals(
                    expected,
                    Messages.parseHandshake(phone.receive(hello, 0).single()).text("reason"),
                )
            }
        }
        assertTrue(limiter.admit(0))
        assertFalse(limiter.admit(2999))
        assertTrue(limiter.admit(3000))
        for (attempt in 2 until 10) assertTrue(limiter.admit(attempt * 3000L))
        assertFalse(limiter.admit(30_000))
        assertFalse(limiter.admit(629_999))
        assertTrue(limiter.admit(630_000))
        val version = JsonObject(Messages.parseHandshake(hello) + ("v" to JsonPrimitive(2)))
        PhonePairing(identity(), 0, true, false, PairingRateLimit()).use { phone ->
            val error =
                Messages.parseHandshake(phone.receive(Messages.handshake(version), 0).single())
            assertEquals("version", error.text("code"))
            assertEquals(1, error["supported"]!!.jsonArray.single().jsonPrimitive.int)
        }
    }

    @Test
    fun sessionsRejectUnknownPeersWrongKeysUnexpectedReadyAndTimeouts() {
        val root = fixture()
        val session = root.obj("session")
        val hello = Frame(Frames.HANDSHAKE, body(session["bodies"]!!.jsonArray[0]))
        PhoneSession(identity(), null, ready(), 0).use { phone ->
            assertEquals(
                "not_paired",
                Messages.parseHandshake(phone.receive(hello, 0).single()).text("code"),
            )
            assertTrue(phone.failed)
        }
        Secret(root.obj("pair").hex("k_link")).use { link ->
            val paired = PairedPeer(Peer(ByteArray(16) { 1 }, "Desk", ""), link)
            PhoneSession(identity(), paired, ready(), 0, busy = true).use { phone ->
                assertEquals(
                    "busy",
                    Messages.parseHandshake(phone.receive(hello, 0).single()).text("code"),
                )
            }
            PhoneSession(identity(), paired, ready(), 0).use { phone ->
                assertFails { phone.tick(5000) }
                assertTrue(phone.failed)
            }
            PhoneSession(
                    identity(),
                    paired,
                    ready(),
                    0,
                    newEphemeral = { pairingKey(session, "phone") },
                )
                .use { phone ->
                    phone.receive(hello, 0)
                    assertFails {
                        phone.receive(
                            Frames.decode(
                                session["pc_to_phone"]!!.jsonArray[1].jsonObject.hex("wire"),
                            ),
                            1,
                        )
                    }
                    assertTrue(phone.failed)
                }
        }
        Secret(ByteArray(32) { 9 }).use { wrong ->
            val paired = PairedPeer(Peer(ByteArray(16) { 1 }, "Desk", ""), wrong)
            PhoneSession(
                    identity(),
                    paired,
                    ready(),
                    0,
                    newEphemeral = { pairingKey(session, "phone") },
                )
                .use { phone ->
                    phone.receive(hello, 0)
                    assertFails {
                        phone.receive(
                            Frames.decode(
                                session["pc_to_phone"]!!.jsonArray[0].jsonObject.hex("wire"),
                            ),
                            1,
                        )
                    }
                    assertTrue(phone.failed)
                }
        }
    }

    @Test
    fun closingSecretsAndEphemeralKeysPreventsTheirReuse() {
        val secret = Secret(ByteArray(32) { 7 })
        secret.close()
        assertTrue(secret.bytes.all { it == 0.toByte() })
        assertFails { secret.copyForProtectedStore() }
        val ephemeral = Ephemeral.generate()
        ephemeral.close()
        assertFails { ephemeral.shared(ByteArray(32) { 1 }) }
    }

    @Test
    fun malformedNumericFieldsAndDeepJsonCannotReachTheApplication() {
        val stats = Messages.parseControl(body(fixture()["controls"]!!.jsonArray[4]))
        for ((field, value) in
            listOf(
                "queue_ms" to JsonPrimitive(-1),
                "battery_pct" to JsonPrimitive(256),
                "cpu_pct" to JsonPrimitive("bad"),
                "charging" to JsonPrimitive("true"),
            )) {
            assertFails {
                Messages.parseControl(Messages.objectBytes(JsonObject(stats + (field to value))))
            }
        }
        val nested =
            "{\"t\":\"request_start\",\"x\":" + "[".repeat(129) + "0" + "]".repeat(129) + "}"
        assertFails { Messages.parseControl(nested.toByteArray()) }
        val quoted =
            Messages.objectOf("request_start", "future" to JsonPrimitive("[{}]".repeat(500)))
        Messages.parseControl(Messages.objectBytes(quoted))
    }

    @Test
    fun writerQueuePreservesMessagesAndMarksTheNextAudioGapPerStream() {
        fun block(stream: Int, seq: Long) =
            AudioBlock(stream, 1, 44100, seq, seq, Long.MIN_VALUE, 1, 0, -120f, 0f, shortArrayOf(0))
        val queue = WriterQueue()
        queue.pushMessage(Frames.CONTROL, "{\"t\":\"request_start\"}".toByteArray())
        for (seq in 0L until 100) {
            assertFalse(queue.pushAudio(block(0, seq)))
            assertFalse(queue.pushAudio(block(1, seq)))
        }
        assertTrue(queue.pushAudio(block(0, 100)))
        assertEquals(201, queue.size)
        assertTrue(queue.pop() is WriterQueue.Outbound.Message)
        val first = (queue.pop() as WriterQueue.Outbound.Audio).block
        assertEquals(1, first.stream)
        assertEquals(0L, first.seq)
        assertEquals(0, first.flags)
        val next = (queue.pop() as WriterQueue.Outbound.Audio).block
        assertEquals(0, next.stream)
        assertEquals(1L, next.seq)
        assertEquals(1, next.flags and 1)
        val liveness = PhoneLiveness(0)
        assertFalse(liveness.shouldClose(9999))
        liveness.received(9999)
        assertFalse(liveness.shouldClose(19998))
        assertTrue(liveness.shouldClose(19999))
    }
}
