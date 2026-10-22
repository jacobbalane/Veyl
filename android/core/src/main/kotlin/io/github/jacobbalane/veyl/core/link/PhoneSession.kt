package io.github.jacobbalane.veyl.core.link

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/** The supplied pairing stays owned by the protected store; this machine copies its link key. */
class PhoneSession(
    private val identity: Identity,
    paired: PairedPeer?,
    private val ready: JsonObject,
    private val startedMs: Long,
    private val busy: Boolean = false,
    private val newEphemeral: () -> Ephemeral = Ephemeral::generate,
) : AutoCloseable {
    init {
        Messages.parseControl(Messages.objectBytes(ready))
        require(ready.text("t") == "ready" && ready["device"] != null)
    }

    private enum class Phase {
        HELLO,
        READY,
        DONE,
        FAILED,
    }

    private var phase = Phase.HELLO
    private val peerId = paired?.peer?.id?.copyOf()
    private var link = paired?.link?.duplicate()
    private var channel: SealedChannel? = null
    var peerReady: JsonObject? = null
        private set

    val done: Boolean
        get() = phase == Phase.DONE

    val failed: Boolean
        get() = phase == Phase.FAILED

    fun tick(nowMs: Long) {
        if (phase != Phase.DONE && phase != Phase.FAILED && nowMs - startedMs >= 5000) {
            close()
            error("Session handshake timed out.")
        }
    }

    fun receive(frame: Frame, nowMs: Long): List<Frame> {
        tick(nowMs)
        return try {
            receiveInner(frame)
        } catch (failure: Exception) {
            close()
            throw failure
        }
    }

    private fun receiveInner(frame: Frame): List<Frame> {
        check(phase != Phase.DONE && phase != Phase.FAILED) { "Session handshake closed." }
        if (phase == Phase.READY) {
            val packet = channel!!.open(frame)
            require(packet.type == Frames.CONTROL) { "Expected PC ready." }
            val message = Messages.parseControl(packet.payload)
            require(
                message.text("t") == "ready" && message["pc"] != null && message["device"] == null,
            ) {
                "Expected PC ready."
            }
            peerReady = message
            phase = Phase.DONE
            return emptyList()
        }
        val hello = Messages.parseHandshake(frame)
        require(hello.text("t") == "hello") { "Expected hello." }
        val code =
            when {
                hello.version() != 1uL -> "version"
                busy -> "busy"
                peerId == null || !peerId.contentEquals(hello.binary("pc_id", 16)) -> "not_paired"
                else -> null
            }
        if (code != null) {
            close()
            return listOf(Messages.error(code))
        }
        return newEphemeral().use { ephemeral ->
            val reply =
                Messages.handshake(
                    Messages.objectOf(
                        "hello_reply",
                        "v" to JsonPrimitive(1),
                        "phone_id" to binary(identity.id),
                        "app" to JsonPrimitive(identity.app),
                        "e_pub" to binary(ephemeral.public),
                        "nonce" to binary(ephemeral.nonce),
                    ),
                )
            ephemeral.shared(hello.binary("e_pub", 32)).use { shared ->
                SessionKeys.derive(shared, link!!, frame.body, reply.body).use { keys ->
                    channel = keys.channel(false)
                }
            }
            link!!.close()
            link = null
            val sealed = channel!!.seal(Frames.CONTROL, Messages.objectBytes(ready))
            phase = Phase.READY
            listOf(reply, sealed)
        }
    }

    fun takeChannel(): SealedChannel? {
        check(done)
        return channel.also { channel = null }
    }

    override fun close() {
        phase = Phase.FAILED
        link?.close()
        link = null
        channel?.close()
        channel = null
        peerReady = null
    }
}
