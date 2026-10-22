package io.github.jacobbalane.veyl.core.link

import java.io.ByteArrayInputStream
import kotlin.test.*
import kotlinx.serialization.json.*
import org.junit.Test

internal fun unhex(value: String): ByteArray =
    value.chunked(2).map { it.toInt(16).toByte() }.toByteArray()

internal fun JsonObject.hex(key: String): ByteArray = unhex(text(key))

internal fun fixture(): JsonObject =
    Json.parseToJsonElement(
            ProtocolVectorsTest::class.java.getResourceAsStream("/v1.json")!!.use {
                it.readBytes().toString(Charsets.UTF_8)
            },
        )
        .jsonObject

internal fun pairingKey(value: JsonObject, role: String): Ephemeral =
    Ephemeral.fromSeed(value.hex("${role}_private"), value.hex("${role}_nonce"))

internal fun same(expected: ByteArray, actual: ByteArray) {
    assertTrue(expected.contentEquals(actual), "Protocol bytes differ.")
}

internal fun body(value: JsonElement): ByteArray =
    value.jsonPrimitive.content.toByteArray(Charsets.UTF_8)

internal fun identity(): Identity =
    Identity(ByteArray(16) { 2 }, "Phone", "Test phone", "veyl-android/0.0.1")

internal fun ready(): JsonObject = Messages.parseControl(body(fixture()["controls"]!!.jsonArray[0]))

class ProtocolVectorsTest {
    @Test
    fun pairingReproducesRfcDhCommitTranscriptHkdfSasAndBothMacs() {
        val pair = fixture().obj("pair")
        pairingKey(pair, "pc").use { pc ->
            pairingKey(pair, "phone").use { phone ->
                same(pair.hex("pc_public"), pc.public)
                same(pair.hex("phone_public"), phone.public)
                same(pair.hex("commit"), Crypto.commit(pc.public, pc.nonce))
                pc.shared(phone.public).use { shared ->
                    same(pair.hex("shared"), shared.copyForProtectedStore())
                    phone.shared(pc.public).use { other ->
                        same(shared.copyForProtectedStore(), other.copyForProtectedStore())
                    }
                    val bodies = pair["bodies"]!!.jsonArray.map(::body)
                    PairKeys.derive(shared, *bodies.toTypedArray()).use { keys ->
                        same(pair.hex("transcript"), keys.transcript)
                        same(
                            pair.hex("okm"),
                            keys.link.copyForProtectedStore() + keys.sasKey.copyForProtectedStore(),
                        )
                        same(pair.hex("k_link"), keys.link.copyForProtectedStore())
                        same(pair.hex("k_sas"), keys.sasKey.copyForProtectedStore())
                        assertTrue(pair.text("sas") == keys.sas())
                        same(pair.hex("confirm_pc"), keys.confirmation(true))
                        same(pair.hex("confirm_phone"), keys.confirmation(false))
                    }
                }
            }
        }
    }

    @Test
    fun sessionReproducesEverySealedFrameInBothDirections() {
        val root = fixture()
        val session = root.obj("session")
        pairingKey(session, "pc").use { pc ->
            pairingKey(session, "phone").use { phone ->
                same(session.hex("pc_public"), pc.public)
                same(session.hex("phone_public"), phone.public)
                pc.shared(phone.public).use { shared ->
                    Secret(root.obj("pair").hex("k_link")).use { link ->
                        same(session.hex("shared"), shared.copyForProtectedStore())
                        val bodies = session["bodies"]!!.jsonArray.map(::body)
                        same(session.hex("transcript"), Crypto.hash(*bodies.toTypedArray()))
                        SessionKeys.derive(shared, link, *bodies.toTypedArray()).use { keys ->
                            same(
                                session.hex("okm"),
                                keys.pcToPhone.copyForProtectedStore() +
                                    keys.phoneToPc.copyForProtectedStore(),
                            )
                            same(
                                session.hex("k_pc_to_phone"),
                                keys.pcToPhone.copyForProtectedStore(),
                            )
                            same(
                                session.hex("k_phone_to_pc"),
                                keys.phoneToPc.copyForProtectedStore(),
                            )
                            keys.channel(true).use { pcChannel ->
                                keys.channel(false).use { phoneChannel ->
                                    for (direction in listOf("pc_to_phone", "phone_to_pc")) {
                                        val sender =
                                            if (direction == "pc_to_phone") pcChannel
                                            else phoneChannel
                                        val receiver =
                                            if (direction == "pc_to_phone") phoneChannel
                                            else pcChannel
                                        for (element in session[direction]!!.jsonArray) {
                                            val vector = element.jsonObject
                                            val type = vector["type"]!!.jsonPrimitive.int
                                            val payload = vector.hex("payload")
                                            val wire = vector.hex("wire")
                                            assertEquals(
                                                vector.integer("counter"),
                                                sender.sendCounter,
                                            )
                                            same(wire, Frames.encode(sender.seal(type, payload)))
                                            val packet = receiver.open(Frames.decode(wire))
                                            assertEquals(type, packet.type)
                                            same(payload, packet.payload)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    @Test
    fun allJsonBodiesAndBinaryPayloadsRoundTripExactly() {
        val root = fixture()
        for (section in listOf("pair", "session")) {
            for (element in root.obj(section)["bodies"]!!.jsonArray) {
                val bytes = body(element)
                val frame = Frame(Frames.HANDSHAKE, bytes)
                same(bytes, Messages.handshake(Messages.parseHandshake(frame)).body)
                val consecutive = Frames.encode(frame) + Frames.encode(frame)
                val input = ByteArrayInputStream(consecutive)
                same(bytes, Frames.read(input).body)
                same(bytes, Frames.read(input).body)
                assertEquals(0, input.available())
            }
        }
        for (element in root["controls"]!!.jsonArray) {
            val bytes = body(element)
            same(bytes, Messages.objectBytes(Messages.parseControl(bytes)))
        }
        val plain = root.obj("audio").hex("plain")
        val audio = AudioBlock.decode(plain)
        same(plain, audio.encode())
        assertEquals(4, audio.frames)
        assertEquals(2, audio.channels)
        assertEquals(-1234567890L, audio.captureNs)
        same(
            root.obj("ping").hex("payload"),
            PingPong.encodePing(root.obj("ping").integer("pc_ns")),
        )
        same(
            root.obj("pong").hex("payload"),
            PingPong.encodePong(
                root.obj("pong").integer("pc_ns"),
                root.obj("pong").integer("phone_ns"),
            ),
        )
        assertEquals(
            root.obj("ping").integer("pc_ns"),
            PingPong.ping(root.obj("ping").hex("payload")),
        )
        assertEquals(
            root.obj("pong").integer("phone_ns"),
            PingPong.pong(root.obj("pong").hex("payload")).second,
        )
    }

    @Test
    fun phonePairingRequiresBothConfirmationsAndMatchesExactReplyBytes() {
        val pair = fixture().obj("pair")
        val bodies = pair["bodies"]!!.jsonArray
        PhonePairing(identity(), 0, true, false, PairingRateLimit()) { pairingKey(pair, "phone") }
            .use { phone ->
                same(
                    body(bodies[1]),
                    phone.receive(Frame(Frames.HANDSHAKE, body(bodies[0])), 0).single().body,
                )
                assertTrue(phone.receive(Frame(Frames.HANDSHAKE, body(bodies[2])), 1).isEmpty())
                assertTrue(phone.sas == pair.text("sas"))
                assertNull(phone.takePairing())
                val peerConfirm =
                    Messages.handshake(
                        Messages.objectOf("pair_confirm", "mac" to binary(pair.hex("confirm_pc"))),
                    )
                phone.receive(peerConfirm, 2)
                assertFalse(phone.done)
                assertNull(phone.takePairing())
                val confirmation = Messages.parseHandshake(phone.confirm(true, 3).single())
                same(pair.hex("confirm_phone"), confirmation.binary("mac", 32))
                assertTrue(phone.done)
                val paired = phone.takePairing()!!
                same(pair.hex("k_link"), paired.link.copyForProtectedStore())
                paired.link.close()
                assertNull(phone.sas)
            }
    }

    @Test
    fun phoneSessionMatchesHelloReplyAndSealedReadyBeforeOpening() {
        val root = fixture()
        val session = root.obj("session")
        val bodies = session["bodies"]!!.jsonArray
        Secret(root.obj("pair").hex("k_link")).use { link ->
            val paired = PairedPeer(Peer(ByteArray(16) { 1 }, "Desk", ""), link)
            PhoneSession(
                    identity(),
                    paired,
                    ready(),
                    0,
                    newEphemeral = { pairingKey(session, "phone") },
                )
                .use { phone ->
                    val outbound = phone.receive(Frame(Frames.HANDSHAKE, body(bodies[0])), 0)
                    same(body(bodies[1]), outbound[0].body)
                    same(
                        session["phone_to_pc"]!!.jsonArray[0].jsonObject.hex("wire"),
                        Frames.encode(outbound[1]),
                    )
                    assertFalse(phone.done)
                    phone.receive(
                        Frames.decode(session["pc_to_phone"]!!.jsonArray[0].jsonObject.hex("wire")),
                        1,
                    )
                    assertTrue(phone.done)
                    phone.takeChannel()!!.use { channel ->
                        assertEquals(1L, channel.sendCounter)
                        assertEquals(1L, channel.receiveCounter)
                        for (element in session["pc_to_phone"]!!.jsonArray.drop(1)) {
                            val vector = element.jsonObject
                            val packet = channel.open(Frames.decode(vector.hex("wire")))
                            same(vector.hex("payload"), packet.payload)
                        }
                        for (element in session["phone_to_pc"]!!.jsonArray.drop(1)) {
                            val vector = element.jsonObject
                            same(
                                vector.hex("wire"),
                                Frames.encode(
                                    channel.seal(
                                        vector["type"]!!.jsonPrimitive.int,
                                        vector.hex("payload"),
                                    ),
                                ),
                            )
                        }
                    }
                }
        }
    }
}
