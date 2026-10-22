package io.github.jacobbalane.veyl.runtime

import io.github.jacobbalane.veyl.core.link.*
import io.github.jacobbalane.veyl.core.util.Base64Url
import java.io.IOException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.spec.GCMParameterSpec
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

class PairingStoreTest {
    private class Document : PairingDocument {
        var bytes: ByteArray? = null
        var fail = false

        override fun read() = bytes?.copyOf()

        override fun write(bytes: ByteArray) {
            if (fail) throw IOException("Simulated write failure.")
            this.bytes = bytes.copyOf()
        }
    }

    private class Wrapper : LinkKeyWrapper {
        private val key = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()

        override fun wrap(id: ByteArray, plain: ByteArray): WrappedLink {
            val c = Cipher.getInstance("AES/GCM/NoPadding")
            c.init(Cipher.ENCRYPT_MODE, key)
            c.updateAAD(id)
            return WrappedLink(c.iv, c.doFinal(plain))
        }

        override fun unwrap(id: ByteArray, wrapped: WrappedLink): ByteArray {
            val c = Cipher.getInstance("AES/GCM/NoPadding")
            c.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, wrapped.iv))
            c.updateAAD(id)
            return c.doFinal(wrapped.ciphertext)
        }
    }

    @Test
    fun onlyWrappedKeysPersistAndTouchAndForgetAreDurable() {
        val document = Document()
        val wrapper = Wrapper()
        var now = "2026-10-22T00:00:00Z"
        val store = PairingStore(document, wrapper) { now }
        val id = ByteArray(16) { it.toByte() }
        val plain = ByteArray(32) { (it + 7).toByte() }
        val peer = PairedPeer(Peer(id, "Desk", ""), Secret(plain))
        try {
            store.remember(peer)
        } finally {
            peer.link.close()
        }
        assertFalse(document.bytes!!.toString(Charsets.UTF_8).contains(Base64Url.encode(plain)))
        val reloaded = PairingStore(document, wrapper) { now }
        val saved = reloaded.find(id)!!
        val copy = saved.link.copyForProtectedStore()
        try {
            assertTrue(Crypto.matches(plain, copy))
        } finally {
            copy.fill(0)
            saved.link.close()
            plain.fill(0)
        }
        now = "2026-10-22T00:00:01Z"
        reloaded.seen(id)
        assertEquals("2026-10-22T00:00:00Z", reloaded.list().single().pairedAt)
        assertEquals(now, reloaded.list().single().lastSeen)
        reloaded.forget(id)
        assertTrue(PairingStore(document, wrapper).list().isEmpty())
        assertNull(reloaded.find(id))
    }

    @Test
    fun aFailedAtomicWriteKeepsThePreviousPairingAndSwappingAnIdCannotUnwrapItsKey() {
        val document = Document()
        val wrapper = Wrapper()
        val store = PairingStore(document, wrapper)
        val id = ByteArray(16) { 1 }
        val peer = PairedPeer(Peer(id, "Desk", ""), Secret(ByteArray(32) { 7 }))
        try {
            store.remember(peer)
            document.fail = true
            assertThrows(IOException::class.java) { store.forget(id) }
            assertEquals(1, store.list().size)
            document.fail = false
            val root = Json.parseToJsonElement(document.bytes!!.toString(Charsets.UTF_8)).jsonObject
            val original = root["pcs"]!!.jsonArray.single().jsonObject
            val otherId = ByteArray(16) { 2 }
            document.bytes =
                buildJsonObject {
                        put("version", 1)
                        put(
                            "pcs",
                            JsonArray(
                                listOf(
                                    JsonObject(
                                        original +
                                            ("pc_id" to JsonPrimitive(Base64Url.encode(otherId))),
                                    ),
                                ),
                            ),
                        )
                    }
                    .toString()
                    .toByteArray(Charsets.UTF_8)
            assertNull(store.find(otherId))
            assertNull(store.find(id))
        } finally {
            peer.link.close()
        }
    }
}
