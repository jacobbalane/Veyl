package io.github.jacobbalane.veyl.core.link

import java.nio.ByteBuffer
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

class Packet(val type: Int, val payload: ByteArray)

/** Each direction has its own key and counter. Any failure closes both directions. */
class SealedChannel(tx: Secret, rx: Secret) : AutoCloseable {
    private val tx = tx.duplicate()
    private val rx = rx.duplicate()
    var sendCounter: Long = 0
        private set

    var receiveCounter: Long = 0
        private set

    var closed = false
        private set

    private var closeFrame: Frame? = null

    fun takeCloseFrame(): Frame? = closeFrame.also { closeFrame = null }

    private fun nonce(counter: Long): ByteArray =
        ByteBuffer.allocate(12).putInt(0).putLong(counter).array()

    private fun aad(counter: Long, length: Int): ByteArray =
        ByteBuffer.allocate(13).putInt(length).put(Frames.SEALED.toByte()).putLong(counter).array()

    private inline fun <T> guarded(block: () -> T): T {
        check(!closed) { "Channel closed." }
        return try {
            block()
        } catch (failure: Exception) {
            close()
            throw failure
        }
    }

    fun seal(type: Int, payload: ByteArray): Frame = guarded {
        require(
            type in 0..255 && sendCounter < COUNTER_LIMIT && payload.size <= Frames.MAX_FRAME - 26,
        ) {
            "Sealed frame limit."
        }
        Messages.validatePayload(type, payload)
        val plain = byteArrayOf(type.toByte()) + payload
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        tx.useBytes {
            cipher.init(
                Cipher.ENCRYPT_MODE,
                SecretKeySpec(it, "AES"),
                GCMParameterSpec(128, nonce(sendCounter)),
            )
        }
        cipher.updateAAD(aad(sendCounter, payload.size + 26))
        val body =
            ByteBuffer.allocate(8 + plain.size + 16)
                .putLong(sendCounter)
                .put(cipher.doFinal(plain))
                .array()
        sendCounter++
        Frame(Frames.SEALED, body)
    }

    fun open(frame: Frame): Packet = guarded {
        require(frame.kind == Frames.SEALED && frame.body.size in 25 until Frames.MAX_FRAME) {
            "Expected sealed frame."
        }
        val counter = ByteBuffer.wrap(frame.body).long
        require(counter == receiveCounter && counter in 0 until COUNTER_LIMIT) {
            "Unexpected sealed counter."
        }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        rx.useBytes {
            cipher.init(
                Cipher.DECRYPT_MODE,
                SecretKeySpec(it, "AES"),
                GCMParameterSpec(128, nonce(counter)),
            )
        }
        cipher.updateAAD(aad(counter, frame.body.size + 1))
        val plain = cipher.doFinal(frame.body, 8, frame.body.size - 8)
        require(plain.isNotEmpty()) { "Empty sealed message." }
        val type = plain[0].toInt() and 255
        val payload = plain.copyOfRange(1, plain.size)
        try {
            Messages.validatePayload(type, payload)
        } catch (failure: Exception) {
            if (type == Frames.AUDIO && sendCounter < COUNTER_LIMIT) {
                val notice =
                    Messages.objectOf(
                        "error",
                        "code" to kotlinx.serialization.json.JsonPrimitive("bad_message"),
                        "message" to
                            kotlinx.serialization.json.JsonPrimitive("Invalid audio block."),
                    )
                closeFrame = seal(Frames.CONTROL, Messages.objectBytes(notice))
            }
            throw failure
        }
        receiveCounter++
        Packet(type, payload)
    }

    override fun close() {
        closed = true
        tx.close()
        rx.close()
    }

    companion object {
        const val COUNTER_LIMIT = 1L shl 48
    }
}
