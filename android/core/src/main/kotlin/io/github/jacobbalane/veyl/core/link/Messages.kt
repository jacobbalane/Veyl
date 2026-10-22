package io.github.jacobbalane.veyl.core.link

import io.github.jacobbalane.veyl.core.util.Base64Url
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import kotlinx.serialization.json.*

/** JSON bodies remain byte arrays for transcript hashing. Parsing never replaces those bytes. */
object Messages {
    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = false
        allowSpecialFloatingPointValues = false
    }

    fun objectBytes(value: JsonObject): ByteArray = value.toString().toByteArray(Charsets.UTF_8)

    fun objectOf(type: String, vararg fields: Pair<String, JsonElement>): JsonObject =
        buildJsonObject {
            put("t", type)
            fields.forEach { (key, value) -> put(key, value) }
        }

    fun parse(bytes: ByteArray, limit: Int): JsonObject {
        require(bytes.size <= limit) { "JSON size limit." }
        require(
            !(bytes.size >= 3 &&
                bytes[0] == 0xef.toByte() &&
                bytes[1] == 0xbb.toByte() &&
                bytes[2] == 0xbf.toByte()),
        ) {
            "JSON BOM is not allowed."
        }
        val text =
            Charsets.UTF_8.newDecoder()
                .onMalformedInput(CodingErrorAction.REPORT)
                .onUnmappableCharacter(CodingErrorAction.REPORT)
                .decode(ByteBuffer.wrap(bytes))
                .toString()
        var depth = 0
        var quoted = false
        var escaped = false
        for (character in text) {
            if (quoted) {
                if (escaped) escaped = false
                else if (character == '\\') escaped = true else if (character == '"') quoted = false
            } else
                when (character) {
                    '"' -> quoted = true
                    '{',
                    '[' -> {
                        depth++
                        require(depth <= 128) { "JSON nesting limit." }
                    }
                    '}',
                    ']' -> {
                        depth--
                        require(depth >= 0) { "Invalid JSON nesting." }
                    }
                }
        }
        val value =
            try {
                json.parseToJsonElement(text) as? JsonObject
                    ?: kotlin.error("Expected JSON object.")
            } catch (_: Exception) {
                kotlin.error("Invalid JSON object.")
            }
        value.text("t")
        return value
    }

    fun handshake(value: JsonObject): Frame {
        val frame = Frame(Frames.HANDSHAKE, objectBytes(value))
        parseHandshake(frame)
        return frame
    }

    fun parseHandshake(frame: Frame): JsonObject {
        require(frame.kind == Frames.HANDSHAKE) { "Expected handshake." }
        val message = parse(frame.body, Frames.MAX_HANDSHAKE)
        when (message.text("t")) {
            "pair_hello" -> {
                message.version()
                message.binary("pc_id", 16)
                message.name("pc_name")
                message.text("app")
                message.binary("commit", 32)
            }
            "pair_reply" -> {
                message.version()
                message.binary("phone_id", 16)
                message.name("phone_name")
                message.name("model")
                message.text("app")
                message.binary("e_pub", 32)
                message.binary("nonce", 32)
            }
            "pair_reveal" -> {
                message.binary("e_pub", 32)
                message.binary("nonce", 32)
            }
            "pair_confirm" -> message.binary("mac", 32)
            "pair_abort" -> message.text("reason")
            "hello" -> {
                message.version()
                message.binary("pc_id", 16)
                message.text("app")
                message.binary("e_pub", 32)
                message.binary("nonce", 32)
            }
            "hello_reply" -> {
                message.version()
                message.binary("phone_id", 16)
                message.text("app")
                message.binary("e_pub", 32)
                message.binary("nonce", 32)
            }
            "error" -> {
                message.text("code")
                message["message"]
                    ?.takeIf { it != JsonNull }
                    ?.let { require(it is JsonPrimitive && it.isString) }
                message["supported"]
                    ?.takeIf { it != JsonNull }
                    ?.jsonArray
                    ?.forEach { it.unsigned() }
            }
            else -> kotlin.error("Unknown handshake message.")
        }
        return message
    }

    fun error(code: String): Frame =
        handshake(
            objectOf(
                "error",
                "code" to JsonPrimitive(code),
                *if (code == "version") arrayOf("supported" to JsonArray(listOf(JsonPrimitive(1))))
                else emptyArray(),
            ),
        )

    fun abort(reason: String): Frame =
        handshake(objectOf("pair_abort", "reason" to JsonPrimitive(reason)))

    fun parseControl(bytes: ByteArray): JsonObject {
        val value = parse(bytes, Frames.MAX_CONTROL)
        when (value.text("t")) {
            "ready" ->
                if (value["device"] != null) {
                    val device = value.obj("device")
                    device.name("phone_name")
                    device.text("manufacturer")
                    device.text("model")
                    device.unsigned("android_sdk", 0xffffffffuL)
                    device.text("app_version")
                    capture(value.obj("capture"))
                } else {
                    val pc = value.obj("pc")
                    pc.name("pc_name")
                    pc.text("app_version")
                    value.obj("want").boolean("meter")
                }
            "audio_config" -> {
                value.unsigned("run_id")
                value.text("source")
                value.unsigned("sample_rate", 0xffffffffuL)
                value.unsigned("frames_per_block", 65535u)
                value.unsigned("ref_channels", 255u)
                value.unsigned("mic_channels", 255u)
                value.text("mic_source")
                value.unsigned("hp_cutoff_hz", 0xffffffffuL)
                value["effects_off"]!!.jsonArray.forEach {
                    require(it is JsonPrimitive && it.isString) { "Invalid effect name." }
                }
                value["mics"]!!.jsonArray.forEach { element ->
                    val mic = element.jsonObject
                    mic.unsigned("index", 0xffffffffuL)
                    mic.text("type")
                    mic["position_m"]
                        ?.takeIf { it != JsonNull }
                        ?.let { position ->
                            require(position.jsonArray.size == 3)
                            position.jsonArray.forEach { it.number() }
                        }
                    mic["channels"]!!.jsonArray.forEach { it.unsigned(255u) }
                }
                value["sound_check"]
                    ?.takeIf { it != JsonNull }
                    ?.let { element ->
                        val check = element.jsonObject
                        check.text("mic_source")
                        check.unsigned("mic_channels", 255u)
                        check.number("score")
                        check.number("mean_coherence")
                    }
            }
            "capture_state" -> capture(value)
            "stats" -> {
                value.unsigned("queue_ms", 0xffffffffuL)
                value.unsigned("dropped_blocks")
                value.number("media_volume")
                value.unsigned("battery_pct", 255u)
                value.boolean("charging")
                value.text("thermal")
                value.number("cpu_pct")
            }
            "phone_info" -> value.name("phone_name")
            "meter" -> {
                value.unsigned("score", 255u)
                value.text("label")
                value.text("tip")
            }
            "gesture" -> {
                value.text("kind")
                value.boolean("acted")
            }
            "request_start" -> Unit
            "error" -> {
                value.text("code")
                value.text("message")
            }
            "bye" -> value.text("reason")
            else -> Unit
        }
        return value
    }

    private fun capture(value: JsonObject) {
        value.text("state")
        value.text("source")
        value.text("route")
        value["reason"]?.let { require(it == JsonNull || (it is JsonPrimitive && it.isString)) }
    }

    fun validatePayload(type: Int, payload: ByteArray) {
        when (type) {
            Frames.CONTROL -> parseControl(payload)
            Frames.AUDIO -> AudioBlock.decode(byteArrayOf(Frames.AUDIO.toByte()) + payload)
            Frames.PING -> PingPong.ping(payload)
            Frames.PONG -> PingPong.pong(payload)
        }
    }
}

fun JsonObject.text(key: String): String {
    val value = this[key] as? JsonPrimitive ?: error("Missing string field.")
    require(value.isString) { "Expected string field." }
    return value.content
}

fun JsonObject.integer(key: String): Long {
    val value = this[key] as? JsonPrimitive ?: error("Missing integer field.")
    require(!value.isString)
    return value.longOrNull ?: error("Expected integer field.")
}

fun JsonElement.unsigned(max: ULong = ULong.MAX_VALUE): ULong {
    val value = this as? JsonPrimitive ?: error("Expected unsigned integer.")
    require(!value.isString)
    return (value.content.toULongOrNull() ?: error("Expected unsigned integer.")).also {
        require(it <= max) { "Unsigned integer limit." }
    }
}

fun JsonObject.unsigned(key: String, max: ULong = ULong.MAX_VALUE): ULong =
    (this[key] ?: error("Missing integer field.")).unsigned(max)

fun JsonElement.number(): Float {
    val value = this as? JsonPrimitive ?: error("Expected number.")
    require(!value.isString)
    return (value.floatOrNull ?: error("Expected number.")).also {
        require(it.isFinite()) { "Number must be finite." }
    }
}

fun JsonObject.number(key: String): Float = (this[key] ?: error("Missing number field.")).number()

fun JsonObject.version(): ULong = unsigned("v")

fun JsonObject.boolean(key: String): Boolean {
    val value = this[key] as? JsonPrimitive ?: error("Missing boolean field.")
    require(!value.isString)
    return value.booleanOrNull ?: error("Expected boolean field.")
}

fun JsonObject.obj(key: String): JsonObject =
    this[key] as? JsonObject ?: error("Missing object field.")

fun JsonObject.name(key: String): String =
    text(key).also {
        require(it.toByteArray(Charsets.UTF_8).size <= 40) { "Name exceeds 40 UTF-8 bytes." }
    }

fun JsonObject.binary(key: String, size: Int): ByteArray =
    Base64Url.decode(text(key)).also { require(it.size == size) { "Wrong binary field length." } }

fun binary(bytes: ByteArray): JsonPrimitive = JsonPrimitive(Base64Url.encode(bytes))
