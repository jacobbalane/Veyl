package io.github.jacobbalane.veyl.core.link

import java.security.MessageDigest
import java.security.SecureRandom
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec
import org.bouncycastle.math.ec.rfc7748.X25519

/** Secrets deliberately have no data-class toString or serializer. */
class Secret(bytes: ByteArray) : AutoCloseable {
    internal val bytes = bytes.copyOf().also { require(it.size == 32) }
    private var closed = false

    internal fun <T> useBytes(block: (ByteArray) -> T): T {
        check(!closed) { "Secret closed." }
        return block(bytes)
    }

    fun copyForProtectedStore(): ByteArray = useBytes { it.copyOf() }

    internal fun duplicate(): Secret = useBytes { Secret(it) }

    override fun close() {
        bytes.fill(0)
        closed = true
    }

    companion object {
        internal fun fromRange(bytes: ByteArray, start: Int, end: Int): Secret {
            val part = bytes.copyOfRange(start, end)
            return try {
                Secret(part)
            } finally {
                part.fill(0)
            }
        }
    }
}

class Ephemeral private constructor(seed: ByteArray, val nonce: ByteArray) : AutoCloseable {
    private val privateKey = seed.copyOf()
    val public = ByteArray(32).also { X25519.scalarMultBase(privateKey, 0, it, 0) }
    private var closed = false

    fun shared(peer: ByteArray): Secret {
        check(!closed)
        require(peer.size == 32)
        val shared = ByteArray(32)
        try {
            X25519.scalarMult(privateKey, 0, peer, 0, shared, 0)
            require(!Crypto.matches(shared, ByteArray(32))) { "Invalid X25519 peer." }
            return Secret(shared)
        } finally {
            shared.fill(0)
        }
    }

    override fun close() {
        privateKey.fill(0)
        closed = true
    }

    companion object {
        fun generate(): Ephemeral {
            val random = SecureRandom()
            val seed = ByteArray(32)
            val nonce = ByteArray(32)
            random.nextBytes(seed)
            random.nextBytes(nonce)
            return try {
                Ephemeral(seed, nonce)
            } finally {
                seed.fill(0)
            }
        }

        /** Fixed inputs are only for protocol vectors. */
        fun fromSeed(seed: ByteArray, nonce: ByteArray): Ephemeral {
            require(seed.size == 32 && nonce.size == 32)
            return Ephemeral(seed, nonce.copyOf())
        }
    }
}

object Crypto {
    fun hash(vararg parts: ByteArray): ByteArray =
        MessageDigest.getInstance("SHA-256").run {
            parts.forEach(::update)
            digest()
        }

    fun commit(public: ByteArray, nonce: ByteArray): ByteArray =
        hash("veyl-commit-v1".toByteArray(Charsets.US_ASCII), public, nonce)

    fun matches(a: ByteArray, b: ByteArray): Boolean = MessageDigest.isEqual(a, b)

    fun mac(key: ByteArray, vararg parts: ByteArray): ByteArray =
        Mac.getInstance("HmacSHA256").run {
            init(SecretKeySpec(key, "HmacSHA256"))
            parts.forEach(::update)
            doFinal()
        }

    fun hkdf(salt: ByteArray, ikm: ByteArray, info: ByteArray, size: Int = 64): ByteArray {
        require(size in 1..(255 * 32))
        val prk = mac(salt, ikm)
        var previous = ByteArray(0)
        val result = ByteArray(size)
        var offset = 0
        var counter = 1
        try {
            while (offset < size) {
                val next = mac(prk, previous, info, byteArrayOf(counter.toByte()))
                previous.fill(0)
                previous = next
                val count = minOf(32, size - offset)
                next.copyInto(result, offset, 0, count)
                offset += count
                counter++
            }
            return result
        } finally {
            prk.fill(0)
            previous.fill(0)
        }
    }
}

class PairKeys(val link: Secret, val sasKey: Secret, val transcript: ByteArray) : AutoCloseable {
    fun sas(): String {
        val digest = sasKey.useBytes {
            Crypto.mac(it, "veyl sas v1".toByteArray(Charsets.US_ASCII))
        }
        val number = java.nio.ByteBuffer.wrap(digest).int.toLong().and(0xffffffffL) % 1_000_000
        return "%03d %03d".format(java.util.Locale.ROOT, number / 1000, number % 1000)
    }

    fun confirmation(pc: Boolean): ByteArray = link.useBytes {
        Crypto.mac(
            it,
            (if (pc) "veyl confirm pc" else "veyl confirm phone").toByteArray(Charsets.US_ASCII),
            transcript,
        )
    }

    override fun close() {
        link.close()
        sasKey.close()
    }

    companion object {
        fun derive(shared: Secret, vararg bodies: ByteArray): PairKeys {
            val transcript = Crypto.hash(*bodies)
            val okm = shared.useBytes {
                Crypto.hkdf(transcript, it, "veyl pair v1".toByteArray(Charsets.US_ASCII))
            }
            return try {
                PairKeys(Secret.fromRange(okm, 0, 32), Secret.fromRange(okm, 32, 64), transcript)
            } finally {
                okm.fill(0)
            }
        }
    }
}

class SessionKeys(val pcToPhone: Secret, val phoneToPc: Secret) : AutoCloseable {
    fun channel(pc: Boolean): SealedChannel =
        if (pc) SealedChannel(pcToPhone, phoneToPc) else SealedChannel(phoneToPc, pcToPhone)

    override fun close() {
        pcToPhone.close()
        phoneToPc.close()
    }

    companion object {
        fun derive(shared: Secret, link: Secret, vararg bodies: ByteArray): SessionKeys {
            val info = "veyl session v1".toByteArray(Charsets.US_ASCII) + Crypto.hash(*bodies)
            val okm = link.useBytes { salt -> shared.useBytes { Crypto.hkdf(salt, it, info) } }
            return try {
                SessionKeys(Secret.fromRange(okm, 0, 32), Secret.fromRange(okm, 32, 64))
            } finally {
                okm.fill(0)
            }
        }
    }
}
