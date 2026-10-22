package io.github.jacobbalane.veyl.core.audio

import kotlin.test.*
import org.junit.Test

class NoPcDeadlineTest {
    @Test
    fun productionStopsAtTenMinutesAndAConnectionRenewsTheDeadline() {
        val policy = NoPcDeadline(50)
        assertFalse(policy.expired(600_049, false))
        assertTrue(policy.expired(600_050, false))
        assertFalse(policy.expired(700_000, true))
        assertFalse(policy.expired(1_299_999, false))
        assertTrue(policy.expired(1_300_000, false))
    }

    @Test
    fun theExplicitDebugLabCanFinishTheThirtyMinuteNullLinkCheck() {
        assertFalse(NoPcDeadline(0, debugLab = true).expired(1_800_000, false))
        assertTrue(NoPcDeadline(0).expired(1_800_000, false))
    }
}
