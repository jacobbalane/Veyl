package io.github.jacobbalane.veyl.core.util

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class Base64UrlTest {
    @Test
    fun encodesRfcVectorsAndEveryByteWithoutPadding() {
        for ((plain, encoded) in listOf("" to "", "f" to "Zg", "fo" to "Zm8", "foo" to "Zm9v", "foobar" to "Zm9vYmFy")) {
            assertEquals(encoded, Base64Url.encode(plain.toByteArray()))
            assertContentEquals(plain.toByteArray(), Base64Url.decode(encoded))
        }
        val bytes = ByteArray(256) { it.toByte() }
        assertContentEquals(bytes, Base64Url.decode(Base64Url.encode(bytes)))
    }

    @Test
    fun rejectsPaddingWrongAlphabetWhitespaceAndNoncanonicalBits() {
        for (invalid in listOf("Zg==", "a+b/", "Z g", "Zg\n", "A", "Zh")) {
            assertFailsWith<IllegalArgumentException>(invalid) { Base64Url.decode(invalid) }
        }
    }
}
