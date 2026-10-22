package io.github.jacobbalane.veyl.link

import io.github.jacobbalane.veyl.core.link.*
import io.github.jacobbalane.veyl.runtime.PeerStore
import java.io.IOException
import java.net.Socket
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.locks.ReentrantLock
import kotlin.concurrent.withLock
import kotlinx.serialization.json.*

data class LinkCapture(val runId: Long, val config: JsonObject?, val state: JsonObject)

data class PairingPrompt(
    val pcName: String,
    val code: String,
    val confirmed: Boolean = false,
    val attempt: Long = 0,
)

interface PhoneLinkCallbacks {
    fun canPair(): Boolean

    fun ready(): JsonObject

    fun capture(): LinkCapture

    fun stats(queueMs: Int, dropped: Long): JsonObject

    fun pairing(prompt: PairingPrompt?) {}

    fun paired(peer: Peer) {}

    fun connected(session: LinkSession, peer: Peer) {}

    fun closed(session: LinkSession) {}

    fun meter(value: JsonObject) {}

    fun gesture(value: JsonObject) {}

    fun requestStart() {}

    fun note(reason: String) {}
}

/** Socket I/O surrounds the core machines. Readers never restart a partially read frame. */
class LinkSession(
    private val socket: Socket,
    private val identity: Identity,
    private val store: PeerStore,
    private val limiter: PairingRateLimit,
    private val clockMs: () -> Long,
    private val callbacks: PhoneLinkCallbacks,
) : AutoCloseable {
    private sealed interface Event {
        class Incoming(val frame: Frame, val receivedNs: Long) : Event

        class Confirm(val matches: Boolean) : Event

        class Bye(val reason: String) : Event

        class Failure(val reason: String) : Event
    }

    private val events = ArrayBlockingQueue<Event>(16)
    private val ending = AtomicBoolean(false)
    private val authenticated = AtomicBoolean(false)
    private var channel: SealedChannel? = null
    private val queue = WriterQueue()
    private val queueLock = ReentrantLock()
    private val queueChanged = queueLock.newCondition()
    private val wireLock = ReentrantLock()
    private val dropped = AtomicLong()
    private val attempt = attempts.incrementAndGet()
    private var configuredRun: Long? = null // Guarded by queueLock.
    private var lastState: JsonObject? = null
    private var latestCapture: LinkCapture? = null
    private var reader: Thread? = null
    private var writer: Thread? = null
    @Volatile
    var peerId: ByteArray? = null
        private set

    fun confirm(matches: Boolean) {
        if (!events.offer(Event.Confirm(matches))) close()
    }

    fun confirm(expectedAttempt: Long, matches: Boolean) {
        if (expectedAttempt == attempt) confirm(matches)
    }

    fun disconnect(reason: String) {
        if (!events.offer(Event.Bye(reason))) close()
    }

    fun sendControl(value: JsonObject) {
        enqueue(Frames.CONTROL, Messages.objectBytes(value))
    }

    fun updateCapture(capture: LinkCapture) = queueLock.withLock {
        if (!authenticated.get() || ending.get()) return@withLock
        latestCapture = capture
        try {
            var changed = false
            if (capture.config != null && configuredRun != capture.runId) {
                queue.clearAudio()
                queue.pushMessage(Frames.CONTROL, Messages.objectBytes(capture.config))
                configuredRun = capture.runId
                changed = true
            } else if (capture.config == null) {
                queue.clearAudio()
                configuredRun = null
            }
            if (changed || lastState != capture.state) {
                queue.pushMessage(Frames.CONTROL, Messages.objectBytes(capture.state))
                lastState = capture.state
            }
            queueChanged.signalAll()
        } catch (_: RuntimeException) {
            close()
        }
    }

    fun audio(id: Long, block: AudioBlock) = queueLock.withLock {
        val capture = latestCapture ?: return@withLock
        if (!authenticated.get() || ending.get() || capture.config == null || capture.runId != id)
            return@withLock
        try {
            if (configuredRun != id) {
                queue.clearAudio()
                queue.pushMessage(Frames.CONTROL, Messages.objectBytes(capture.config))
                queue.pushMessage(Frames.CONTROL, Messages.objectBytes(capture.state))
                configuredRun = id
                lastState = capture.state
            }
            if (queue.pushAudio(block)) dropped.incrementAndGet()
            queueChanged.signalAll()
        } catch (_: RuntimeException) {
            close()
        }
    }

    private fun enqueue(type: Int, payload: ByteArray) = queueLock.withLock {
        if (!authenticated.get() || ending.get()) return@withLock
        try {
            queue.pushMessage(type, payload)
            queueChanged.signalAll()
        } catch (_: RuntimeException) {
            close()
        }
    }

    fun run() {
        val started = clockMs()
        try {
            socket.tcpNoDelay = true
            socket.sendBufferSize = 256 * 1024
            // The owner checks deadlines independently; closing the socket interrupts any partial
            // read.
            socket.soTimeout = 0
            reader =
                Thread(
                        {
                            try {
                                while (!ending.get()) {
                                    val frame = Frames.read(socket.getInputStream())
                                    if (
                                        !events.offer(
                                            Event.Incoming(frame, System.nanoTime()),
                                            200,
                                            TimeUnit.MILLISECONDS,
                                        )
                                    ) {
                                        close()
                                        break
                                    }
                                }
                            } catch (_: Exception) {
                                events.offer(Event.Failure("transport"))
                            }
                        },
                        "Veyl-link-reader",
                    )
                    .apply {
                        isDaemon = true
                        start()
                    }
            val first = awaitFrame(started + 5000) ?: return
            val hello = Messages.parseHandshake(first)
            when (hello.text("t")) {
                "pair_hello" -> pair(first, hello)
                "hello" -> session(first, hello, started)
                else -> callbacks.note("bad_message")
            }
        } catch (_: IOException) {
            callbacks.note("link_closed")
        } catch (_: InterruptedException) {
            Thread.currentThread().interrupt()
        } catch (_: Exception) {
            callbacks.note("bad_message")
        } finally {
            close()
            try {
                reader?.join(1000)
                writer?.join(1000)
            } catch (_: InterruptedException) {
                Thread.currentThread().interrupt()
            }
            channel?.let { synchronized(it) { it.close() } }
            channel = null
            queueLock.withLock { queue.clear() }
            callbacks.pairing(null)
            callbacks.closed(this)
        }
    }

    private fun awaitFrame(deadline: Long): Frame? {
        while (!ending.get()) {
            if (clockMs() >= deadline) throw IOException("Handshake timeout.")
            when (val event = events.poll(50, TimeUnit.MILLISECONDS)) {
                is Event.Incoming -> return event.frame
                is Event.Failure -> throw IOException("Connection closed.")
                is Event.Bye -> return null
                else -> Unit
            }
        }
        return null
    }

    private fun writeHandshake(frames: List<Frame>) {
        frames.forEach { Frames.write(socket.getOutputStream(), it) }
    }

    private fun pair(first: Frame, hello: JsonObject) {
        val pc = Peer(hello.binary("pc_id", 16), hello.name("pc_name"), "")
        PhonePairing(identity, clockMs(), callbacks.canPair(), false, limiter).use { pair ->
            writeHandshake(pair.receive(first, clockMs()))
            var shown = false
            var confirmed = false
            var code = ""
            while (!ending.get() && !pair.done && pair.failure == null) {
                if (!callbacks.canPair()) {
                    writeHandshake(listOf(Messages.abort("not_in_foreground")))
                    return
                }
                writeHandshake(pair.tick(clockMs()))
                if (pair.failure != null) break
                when (val event = events.poll(50, TimeUnit.MILLISECONDS)) {
                    is Event.Incoming -> writeHandshake(pair.receive(event.frame, clockMs()))
                    is Event.Confirm -> {
                        if (!event.matches) {
                            writeHandshake(pair.cancel())
                            break
                        }
                        if (!confirmed && pair.sas != null) {
                            confirmed = true
                            writeHandshake(pair.confirm(true, clockMs()))
                            callbacks.pairing(PairingPrompt(pc.name, code, true, attempt))
                        }
                    }
                    is Event.Bye -> {
                        writeHandshake(pair.cancel())
                        break
                    }
                    is Event.Failure -> throw IOException("Pairing connection closed.")
                    else -> Unit
                }
                if (!shown && pair.sas != null) {
                    shown = true
                    code = pair.sas!!
                    callbacks.pairing(PairingPrompt(pc.name, code, attempt = attempt))
                }
            }
            val paired = pair.takePairing()
            if (paired != null) {
                try {
                    store.remember(paired)
                    callbacks.paired(paired.peer)
                } finally {
                    paired.link.close()
                }
            } else callbacks.note("pairing_failed")
        }
    }

    private fun session(first: Frame, hello: JsonObject, started: Long) {
        val paired = store.find(hello.binary("pc_id", 16))
        val handshake = PhoneSession(identity, paired, callbacks.ready(), started)
        try {
            writeHandshake(handshake.receive(first, clockMs()))
            while (!ending.get() && !handshake.done && !handshake.failed) {
                val frame = awaitFrame(started + 5000) ?: return
                writeHandshake(handshake.receive(frame, clockMs()))
            }
            if (!handshake.done || paired == null) return
            channel = handshake.takeChannel()
            peerId = paired.peer.id.copyOf()
        } finally {
            handshake.close()
            paired?.link?.close()
        }
        if (ending.get()) return
        runCatching { store.seen(paired.peer.id) }
            .onFailure { callbacks.note("pair_store_write_failed") }
        authenticated.set(true)
        updateCapture(callbacks.capture())
        callbacks.connected(this, paired.peer)
        startWriter()
        val liveness = PhoneLiveness(clockMs())
        var nextStats = clockMs() + 1000
        val ignored = mutableSetOf<Int>()
        while (!ending.get()) {
            val now = clockMs()
            if (liveness.shouldClose(now)) {
                callbacks.note("link_timeout")
                break
            }
            if (now >= nextStats) {
                val queued = queueLock.withLock { queue.audioQueueMs }
                sendControl(callbacks.stats(queued, dropped.get()))
                nextStats = now + 1000
            }
            when (val event = events.poll(50, TimeUnit.MILLISECONDS)) {
                is Event.Incoming -> {
                    val cipher = channel!!
                    val packet =
                        try {
                            synchronized(cipher) {
                                try {
                                    cipher.open(event.frame)
                                } catch (failure: Exception) {
                                    ending.set(true)
                                    throw failure
                                }
                            }
                        } catch (_: Exception) {
                            val notice = synchronized(cipher) { cipher.takeCloseFrame() }
                            writeFinal(notice)
                            callbacks.note("bad_message")
                            break
                        }
                    liveness.received(clockMs())
                    when (packet.type) {
                        Frames.PING ->
                            enqueue(
                                Frames.PONG,
                                PingPong.encodePong(
                                    PingPong.ping(packet.payload),
                                    event.receivedNs,
                                ),
                            )
                        Frames.CONTROL -> {
                            val message = Messages.parseControl(packet.payload)
                            when (message.text("t")) {
                                "request_start" -> callbacks.requestStart()
                                "meter" -> callbacks.meter(message)
                                "gesture" -> callbacks.gesture(message)
                                "bye" -> ending.set(true)
                                "error" -> callbacks.note("peer_error")
                            }
                        }
                        Frames.AUDIO -> {
                            finish(
                                Messages.objectOf(
                                    "error",
                                    "code" to JsonPrimitive("bad_message"),
                                    "message" to JsonPrimitive("PC audio is not supported."),
                                ),
                            )
                            break
                        }
                        else -> if (ignored.add(packet.type)) callbacks.note("ignored_payload_type")
                    }
                }
                is Event.Bye -> {
                    finish(Messages.objectOf("bye", "reason" to JsonPrimitive(event.reason)))
                    break
                }
                is Event.Failure -> throw IOException("Session connection closed.")
                else -> Unit
            }
        }
    }

    private fun startWriter() {
        writer =
            Thread(
                    {
                        try {
                            while (!ending.get()) {
                                val item =
                                    queueLock.withLock {
                                        while (queue.size == 0 && !ending.get()) queueChanged.await(
                                            100,
                                            TimeUnit.MILLISECONDS,
                                        )
                                        queue.pop()
                                    } ?: continue
                                wireLock.lock()
                                try {
                                    val cipher = channel!!
                                    val frame =
                                        synchronized(cipher) {
                                            if (ending.get()) return@Thread
                                            when (item) {
                                                is WriterQueue.Outbound.Audio -> {
                                                    val bytes = item.block.encode()
                                                    cipher.seal(
                                                        Frames.AUDIO,
                                                        bytes.copyOfRange(1, bytes.size),
                                                    )
                                                }
                                                is WriterQueue.Outbound.Message ->
                                                    cipher.seal(item.type, item.payload)
                                            }
                                        }
                                    Frames.write(socket.getOutputStream(), frame)
                                } finally {
                                    wireLock.unlock()
                                }
                            }
                        } catch (_: Exception) {
                            if (!ending.get()) {
                                events.offer(Event.Failure("transport"))
                                close()
                            }
                        }
                    },
                    "Veyl-link-writer",
                )
                .apply {
                    isDaemon = true
                    start()
                }
    }

    private fun finish(message: JsonObject) {
        val cipher = channel ?: return
        val frame =
            synchronized(cipher) {
                ending.set(true)
                cipher.seal(Frames.CONTROL, Messages.objectBytes(message))
            }
        writeFinal(frame)
    }

    private fun writeFinal(frame: Frame?) {
        if (frame == null || !wireLock.tryLock(1000, TimeUnit.MILLISECONDS)) return
        try {
            runCatching { Frames.write(socket.getOutputStream(), frame) }
        } finally {
            wireLock.unlock()
        }
    }

    override fun close() {
        ending.set(true)
        authenticated.set(false)
        runCatching { socket.close() }
        queueLock.withLock { queueChanged.signalAll() }
    }

    companion object {
        private val attempts = AtomicLong()
    }
}
