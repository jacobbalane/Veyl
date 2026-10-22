package io.github.jacobbalane.veyl.core.util

import kotlin.test.*
import org.junit.Test

class Utf8Test {
    @Test
    fun wireNamesRespectBytesWithoutBreakingEmojiOrMultibyteNames() {
        assertEquals("a".repeat(40), Utf8.prefix("a".repeat(41)))
        assertEquals("界".repeat(13), Utf8.prefix("界".repeat(20)))
        assertEquals("😀".repeat(10), Utf8.prefix("😀".repeat(12)))
        assertEquals("AB", Utf8.prefix(" A\nB\u0000 "))
        assertEquals("�", Utf8.prefix("\ud800"))
        assertEquals("", Utf8.prefix("😀", 3))
    }
}
