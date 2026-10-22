package io.github.jacobbalane.veyl.runtime

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import io.github.jacobbalane.veyl.core.link.*
import io.github.jacobbalane.veyl.core.util.Base64Url
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileNotFoundException
import java.io.IOException
import java.security.KeyStore
import java.time.Instant
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlinx.serialization.json.*

data class SavedPc(val id: String, val name: String, val pairedAt: String, val lastSeen: String)

interface PeerStore {
    fun list(): List<SavedPc>

    fun find(id: ByteArray): PairedPeer?

    fun remember(peer: PairedPeer)

    fun seen(id: ByteArray)

    fun forget(id: ByteArray)
}

interface PairingDocument {
    fun read(): ByteArray?

    fun write(bytes: ByteArray)
}

class WrappedLink(val iv: ByteArray, val ciphertext: ByteArray)

interface LinkKeyWrapper {
    fun wrap(id: ByteArray, plain: ByteArray): WrappedLink

    fun unwrap(id: ByteArray, wrapped: WrappedLink): ByteArray
}

/** Only encrypted link keys are serialized. Callers own and must close returned Secrets. */
class PairingStore(
    private val document: PairingDocument,
    private val wrapper: LinkKeyWrapper,
    private val now: () -> String = { Instant.now().toString() },
) : PeerStore {
    private class Entry(val pc: SavedPc, val wrapped: WrappedLink)

    override fun list(): List<SavedPc> = synchronized(this) { load().map { it.pc } }

    override fun find(id: ByteArray): PairedPeer? =
        synchronized(this) {
            require(id.size == 16)
            val entry = load().find { it.pc.id == Base64Url.encode(id) } ?: return@synchronized null
            val plain =
                try {
                    wrapper.unwrap(id, entry.wrapped)
                } catch (_: Exception) {
                    return@synchronized null
                }
            try {
                PairedPeer(Peer(id.copyOf(), entry.pc.name, ""), Secret(plain))
            } finally {
                plain.fill(0)
            }
        }

    override fun remember(peer: PairedPeer) =
        synchronized(this) {
            require(
                peer.peer.id.size == 16 && peer.peer.name.toByteArray(Charsets.UTF_8).size <= 40,
            )
            val entries = load().toMutableList()
            val id = Base64Url.encode(peer.peer.id)
            val previous = entries.find { it.pc.id == id }
            entries.removeAll { it.pc.id == id }
            check(entries.size < 64) { "Too many paired PCs." }
            val plain = peer.link.copyForProtectedStore()
            val wrapped =
                try {
                    wrapper.wrap(peer.peer.id, plain)
                } finally {
                    plain.fill(0)
                }
            entries.add(
                Entry(SavedPc(id, peer.peer.name, previous?.pc?.pairedAt ?: now(), now()), wrapped),
            )
            save(entries)
        }

    override fun seen(id: ByteArray) =
        synchronized(this) {
            val encoded = Base64Url.encode(id)
            val entries = load()
            save(
                entries.map {
                    if (it.pc.id == encoded) Entry(it.pc.copy(lastSeen = now()), it.wrapped) else it
                },
            )
        }

    override fun forget(id: ByteArray) =
        synchronized(this) {
            val encoded = Base64Url.encode(id)
            save(load().filterNot { it.pc.id == encoded })
        }

    private fun load(): List<Entry> {
        val bytes = document.read() ?: return emptyList()
        require(bytes.size <= 1_048_576) { "Pairing file too large." }
        val root = Json.parseToJsonElement(bytes.toString(Charsets.UTF_8)).jsonObject
        require(root.unsigned("version") == 1uL)
        val entries = root["pcs"]!!.jsonArray
        require(entries.size <= 64)
        return entries
            .map { value ->
                val pc = value.jsonObject
                val id = pc.binary("pc_id", 16)
                val key = pc.obj("k_link")
                val paired = pc.text("paired_at")
                val seen = pc.text("last_seen")
                Instant.parse(paired)
                Instant.parse(seen)
                Entry(
                    SavedPc(Base64Url.encode(id), pc.name("pc_name"), paired, seen),
                    WrappedLink(key.binary("iv", 12), key.binary("ct", 48)),
                )
            }
            .also { list -> require(list.map { it.pc.id }.distinct().size == list.size) }
    }

    private fun save(entries: List<Entry>) {
        val json = buildJsonObject {
            put("version", 1)
            put(
                "pcs",
                buildJsonArray {
                    entries.forEach { entry ->
                        add(
                            buildJsonObject {
                                put("pc_id", entry.pc.id)
                                put("pc_name", entry.pc.name)
                                put("paired_at", entry.pc.pairedAt)
                                put("last_seen", entry.pc.lastSeen)
                                put(
                                    "k_link",
                                    buildJsonObject {
                                        put("iv", Base64Url.encode(entry.wrapped.iv))
                                        put("ct", Base64Url.encode(entry.wrapped.ciphertext))
                                    },
                                )
                            },
                        )
                    }
                },
            )
        }
        document.write(json.toString().toByteArray(Charsets.UTF_8))
    }

    companion object {
        fun android(context: Context): PairingStore =
            PairingStore(
                AndroidPairingDocument(File(context.noBackupFilesDir, "pairings.json")),
                AndroidLinkKeyWrapper(),
            )
    }
}

private class AndroidPairingDocument(file: File) : PairingDocument {
    private val atomic = AtomicFile(file)

    override fun read(): ByteArray? =
        try {
            atomic.openRead().use { input ->
                val result = ByteArrayOutputStream()
                val block = ByteArray(8192)
                while (true) {
                    val count = input.read(block)
                    if (count < 0) break
                    if (result.size() + count > 1_048_576)
                        throw IOException("Pairing file too large.")
                    result.write(block, 0, count)
                }
                result.toByteArray()
            }
        } catch (_: FileNotFoundException) {
            null
        }

    override fun write(bytes: ByteArray) {
        val output = atomic.startWrite()
        try {
            output.write(bytes)
            atomic.finishWrite(output)
        } catch (failure: Exception) {
            atomic.failWrite(output)
            throw failure
        }
    }
}

private class AndroidLinkKeyWrapper : LinkKeyWrapper {
    private val store by lazy { KeyStore.getInstance("AndroidKeyStore").apply { load(null) } }

    @Synchronized
    private fun key(create: Boolean): SecretKey {
        (store.getKey("veyl_link_wrap", null) as? SecretKey)?.let {
            return it
        }
        check(create) { "The pairing key is unavailable." }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").run {
            init(
                KeyGenParameterSpec.Builder(
                        "veyl_link_wrap",
                        KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
                    )
                    .setKeySize(256)
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                    .setUserAuthenticationRequired(false)
                    .build(),
            )
            generateKey()
        }
    }

    override fun wrap(id: ByteArray, plain: ByteArray): WrappedLink {
        require(plain.size == 32)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key(true))
        cipher.updateAAD(id)
        return WrappedLink(cipher.iv, cipher.doFinal(plain))
    }

    override fun unwrap(id: ByteArray, wrapped: WrappedLink): ByteArray {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(false), GCMParameterSpec(128, wrapped.iv))
        cipher.updateAAD(id)
        return cipher.doFinal(wrapped.ciphertext)
    }
}
