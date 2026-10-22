package io.github.jacobbalane.veyl.runtime

import io.github.jacobbalane.veyl.core.link.Crypto
import io.github.jacobbalane.veyl.core.util.Base64Url
import java.security.SecureRandom

/** The exported launcher accepts a notification start only once, for the same live session. */
class StartRequestGate(
    private val nowMs: () -> Long,
    private val random: () -> ByteArray = { ByteArray(16).also { SecureRandom().nextBytes(it) } },
) {
    private var session: Any? = null
    private var token: ByteArray? = null
    private var expires = 0L

    @Synchronized
    fun issue(owner: Any): String {
        clear()
        session = owner
        expires = nowMs() + 120_000
        val bytes = random()
        require(bytes.size == 16)
        token = bytes
        return Base64Url.encode(bytes)
    }

    @Synchronized
    fun consume(value: String?, owner: Any?): Boolean {
        val expected = token ?: return false
        if (nowMs() >= expires) {
            clear()
            return false
        }
        if (owner == null || session !== owner || value == null) return false
        val supplied = runCatching { Base64Url.decode(value) }.getOrNull() ?: return false
        val matches =
            try {
                Crypto.matches(expected, supplied)
            } finally {
                supplied.fill(0)
            }
        if (matches) clear()
        return matches
    }

    @Synchronized
    fun revoke(owner: Any) {
        if (session === owner) clear()
    }

    private fun clear() {
        token?.fill(0)
        token = null
        session = null
        expires = 0
    }
}
