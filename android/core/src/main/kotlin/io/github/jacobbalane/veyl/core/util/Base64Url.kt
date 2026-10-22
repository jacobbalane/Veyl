package io.github.jacobbalane.veyl.core.util

import java.util.Base64

object Base64Url {
    private val alphabet = Regex("[A-Za-z0-9_-]*")

    fun encode(bytes: ByteArray): String = Base64.getUrlEncoder().withoutPadding().encodeToString(bytes)

    fun decode(text: String): ByteArray {
        require(alphabet.matches(text)) { "Expected unpadded base64url." }
        val bytes = Base64.getUrlDecoder().decode(text)
        require(encode(bytes) == text) { "Expected canonical base64url." }
        return bytes
    }
}
