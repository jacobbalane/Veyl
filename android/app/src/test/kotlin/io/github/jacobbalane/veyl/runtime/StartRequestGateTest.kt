package io.github.jacobbalane.veyl.runtime

import org.junit.Assert.*
import org.junit.Test

class StartRequestGateTest {
    @Test
    fun forgedReplayedExpiredAndOtherSessionNotificationsCannotStartCapture() {
        var now = 0L
        var sequence = 0
        val gate = StartRequestGate({ now }, { ByteArray(16) { (++sequence).toByte() } })
        val first = Any()
        val other = Any()
        val token = gate.issue(first)
        assertFalse(gate.consume("forged", first))
        assertFalse(gate.consume(token, other))
        assertTrue(gate.consume(token, first))
        assertFalse(gate.consume(token, first))
        val expired = gate.issue(first)
        now = 120_000
        assertFalse(gate.consume(expired, first))
        val replaced = gate.issue(first)
        val fresh = gate.issue(other)
        assertFalse(gate.consume(replaced, first))
        gate.revoke(first)
        assertTrue(gate.consume(fresh, other))
        val revoked = gate.issue(first)
        gate.revoke(first)
        assertFalse(gate.consume(revoked, first))
    }
}
