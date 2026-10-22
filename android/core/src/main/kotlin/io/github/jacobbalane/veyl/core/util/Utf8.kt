package io.github.jacobbalane.veyl.core.util

object Utf8 {
    /** Names are limited in bytes on the wire; never split a UTF-8 character. */
    fun prefix(value: String, maxBytes: Int = 40): String {
        require(maxBytes >= 0)
        var used = 0
        return buildString {
            for (original in value.codePoints().toArray()) {
                val point = if (original in 0xd800..0xdfff) 0xfffd else original
                if (Character.isISOControl(point)) continue
                val part = String(Character.toChars(point))
                val bytes = part.toByteArray(Charsets.UTF_8).size
                if (used + bytes > maxBytes) break
                append(part)
                used += bytes
            }
        }
            .trim()
    }
}
