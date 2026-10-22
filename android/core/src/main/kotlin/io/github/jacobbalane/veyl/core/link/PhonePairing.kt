package io.github.jacobbalane.veyl.core.link

import kotlinx.serialization.json.JsonPrimitive

class Identity(val id: ByteArray, val name: String, val model: String, val app: String) {
    init {
        require(
            id.size == 16 &&
                name.toByteArray(Charsets.UTF_8).size <= 40 &&
                model.toByteArray(Charsets.UTF_8).size <= 40,
        )
    }
}

class Peer(val id: ByteArray, val name: String, val model: String)

class PairedPeer(val peer: Peer, val link: Secret)

class PairingRateLimit {
    private val attempts = ArrayDeque<Long>()
    private var last: Long? = null
    private var blockedUntil = 0L

    fun admit(nowMs: Long): Boolean {
        require(nowMs >= 0)
        if (nowMs < blockedUntil || last?.let { nowMs - it < 3000 } == true) return false
        while (attempts.isNotEmpty() && nowMs - attempts.first() >= 600_000) attempts.removeFirst()
        if (attempts.size >= 10) {
            blockedUntil = nowMs + 600_000
            return false
        }
        last = nowMs
        attempts.addLast(nowMs)
        return true
    }
}

/** Call for pair_hello only. The caller keeps the limiter across TCP connections. */
class PhonePairing(
    private val identity: Identity,
    private val startedMs: Long,
    foreground: Boolean,
    busy: Boolean,
    limiter: PairingRateLimit,
    private val newEphemeral: () -> Ephemeral = Ephemeral::generate,
) : AutoCloseable {
    private enum class Phase {
        HELLO,
        REVEAL,
        CONFIRM,
        DONE,
        FAILED,
    }

    private var phase = Phase.HELLO
    private val denial =
        when {
            busy -> "busy"
            !foreground -> "not_in_foreground"
            !limiter.admit(startedMs) -> "rate_limited"
            else -> null
        }
    private var ephemeral: Ephemeral? = null
    private var commitment: ByteArray? = null
    private val bodies = mutableListOf<ByteArray>()
    private var peer: Peer? = null
    private var keys: PairKeys? = null
    private var ownConfirmed = false
    private var peerConfirmed = false
    private var confirmationStarted: Long? = null
    private var paired: PairedPeer? = null
    var sas: String? = null
        private set

    var failure: String? = null
        private set

    val done: Boolean
        get() = phase == Phase.DONE

    private fun fail(reason: String) {
        phase = Phase.FAILED
        failure = reason
        sas = null
        ephemeral?.close()
        ephemeral = null
        keys?.close()
        keys = null
        paired?.link?.close()
        paired = null
    }

    private fun abort(reason: String): List<Frame> {
        fail(reason)
        return listOf(Messages.abort(reason))
    }

    fun tick(nowMs: Long): List<Frame> {
        if (phase == Phase.DONE || phase == Phase.FAILED) return emptyList()
        if (
            nowMs - startedMs >= 150_000 ||
                confirmationStarted?.let { nowMs - it >= 120_000 } == true
        )
            return abort("timeout")
        return emptyList()
    }

    fun receive(frame: Frame, nowMs: Long): List<Frame> {
        val expired = tick(nowMs)
        if (expired.isNotEmpty()) return expired
        check(phase != Phase.DONE && phase != Phase.FAILED) { "Pairing closed." }
        return try {
            receiveInner(frame, nowMs)
        } catch (failure: Exception) {
            fail("bad_message")
            throw failure
        }
    }

    private fun receiveInner(frame: Frame, nowMs: Long): List<Frame> {
        val message = Messages.parseHandshake(frame)
        val type = message.text("t")
        if (type == "pair_abort" || type == "error") {
            fail(message.text(if (type == "error") "code" else "reason"))
            return emptyList()
        }
        return when {
            phase == Phase.HELLO && type == "pair_hello" -> {
                if (message.version() != 1uL) {
                    fail("version")
                    return listOf(Messages.error("version"))
                }
                denial?.let {
                    return abort(it)
                }
                peer = Peer(message.binary("pc_id", 16), message.name("pc_name"), "")
                commitment = message.binary("commit", 32)
                bodies.add(frame.body.copyOf())
                val key = newEphemeral()
                ephemeral = key
                val reply =
                    Messages.handshake(
                        Messages.objectOf(
                            "pair_reply",
                            "v" to JsonPrimitive(1),
                            "phone_id" to binary(identity.id),
                            "phone_name" to JsonPrimitive(identity.name),
                            "model" to JsonPrimitive(identity.model),
                            "app" to JsonPrimitive(identity.app),
                            "e_pub" to binary(key.public),
                            "nonce" to binary(key.nonce),
                        ),
                    )
                bodies.add(reply.body.copyOf())
                phase = Phase.REVEAL
                listOf(reply)
            }
            phase == Phase.REVEAL && type == "pair_reveal" -> {
                val public = message.binary("e_pub", 32)
                val nonce = message.binary("nonce", 32)
                if (!Crypto.matches(commitment!!, Crypto.commit(public, nonce)))
                    return abort("mismatch")
                bodies.add(frame.body.copyOf())
                val key = ephemeral!!
                try {
                    key.shared(public).use { keys = PairKeys.derive(it, *bodies.toTypedArray()) }
                } finally {
                    key.close()
                    ephemeral = null
                }
                sas = keys!!.sas()
                phase = Phase.CONFIRM
                confirmationStarted = nowMs
                emptyList()
            }
            phase == Phase.CONFIRM && type == "pair_confirm" -> {
                if (
                    peerConfirmed ||
                        !Crypto.matches(keys!!.confirmation(true), message.binary("mac", 32))
                )
                    return abort("bad_mac")
                peerConfirmed = true
                finish()
                emptyList()
            }
            else -> error("Unexpected pairing message.")
        }
    }

    fun confirm(matches: Boolean, nowMs: Long): List<Frame> {
        val expired = tick(nowMs)
        if (expired.isNotEmpty()) return expired
        check(phase == Phase.CONFIRM && !ownConfirmed) { "Not awaiting confirmation." }
        if (!matches) return abort("user_cancelled")
        val reply =
            Messages.handshake(
                Messages.objectOf("pair_confirm", "mac" to binary(keys!!.confirmation(false))),
            )
        ownConfirmed = true
        finish()
        return listOf(reply)
    }

    private fun finish() {
        if (ownConfirmed && peerConfirmed) {
            paired = PairedPeer(peer!!, keys!!.link.duplicate())
            keys!!.close()
            keys = null
            sas = null
            phase = Phase.DONE
        }
    }

    fun cancel(): List<Frame> {
        check(phase != Phase.DONE && phase != Phase.FAILED) { "Pairing closed." }
        return abort("user_cancelled")
    }

    fun takePairing(): PairedPeer? = paired.also { paired = null }

    override fun close() {
        if (phase != Phase.FAILED) fail("closed")
    }
}
