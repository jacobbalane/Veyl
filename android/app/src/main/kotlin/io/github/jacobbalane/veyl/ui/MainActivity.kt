package io.github.jacobbalane.veyl.ui

import android.Manifest
import android.app.*
import android.content.Intent
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.graphics.Rect
import android.media.projection.MediaProjectionConfig
import android.media.projection.MediaProjectionManager
import android.os.*
import android.provider.DocumentsContract
import android.provider.Settings
import android.util.Rational
import android.view.MotionEvent
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.core.net.toUri
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.lifecycleScope
import io.github.jacobbalane.veyl.BuildConfig
import io.github.jacobbalane.veyl.R
import io.github.jacobbalane.veyl.VeylApp
import io.github.jacobbalane.veyl.capture.ListeningService
import io.github.jacobbalane.veyl.runtime.CheckSnapshot
import io.github.jacobbalane.veyl.ui.theme.VeylTheme
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first

class MainActivity : ComponentActivity() {
    private val runtime
        get() = (application as VeylApp).runtime

    private var page by mutableStateOf("welcome")
    private var notificationStep by mutableStateOf(false)
    private var captureError by mutableStateOf<Int?>(null)
    private var explain by mutableStateOf(false)
    private var touchAt by mutableLongStateOf(0L)
    private var listening = false
    private var pip by mutableStateOf(false)
    private var pipTip by mutableStateOf(false)
    private var pipWarned = false
    private var pendingSource = "onboard"
    private var pendingLab = false
    private var afterCheck: String? = null
    private val projectionResult =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            val data = result.data
            if (result.resultCode == RESULT_OK && data != null)
                startServiceCapture(result.resultCode, data)
            else captureError = R.string.consent_cancelled
        }
    private val notifications =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) {
            finishOnboarding()
        }
    private val microphone =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            if (!granted) captureError = R.string.microphone_denied
            else if (pendingSource == "onboard") notificationStep = true else afterMicrophone()
        }
    private val music =
        registerForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
            val readable = uris.filter { uri ->
                runCatching {
                    contentResolver.takePersistableUriPermission(
                        uri,
                        Intent.FLAG_GRANT_READ_URI_PERMISSION,
                    )
                }
                    .isSuccess
            }
            if (readable.isNotEmpty()) {
                runtime.settings.playerItems(
                    runtime.settings.state.value.playerItems + readable.map { it.toString() },
                )
                runtime.settings.source("player")
            }
        }
    private val folder =
        registerForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
            if (uri != null)
                lifecycleScope.launch {
                    try {
                        contentResolver.takePersistableUriPermission(
                            uri,
                            Intent.FLAG_GRANT_READ_URI_PERMISSION,
                        )
                        val items =
                            withContext(Dispatchers.IO) {
                                val pending = ArrayDeque<String>()
                                pending.add(DocumentsContract.getTreeDocumentId(uri))
                                val seen = mutableSetOf<String>()
                                val found = mutableListOf<String>()
                                while (
                                    pending.isNotEmpty() && seen.size < 10000 && found.size < 10000
                                ) {
                                    ensureActive()
                                    val id = pending.removeFirst()
                                    if (!seen.add(id)) continue
                                    contentResolver
                                        .query(
                                            DocumentsContract.buildChildDocumentsUriUsingTree(
                                                uri,
                                                id,
                                            ),
                                            arrayOf(
                                                DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                                                DocumentsContract.Document.COLUMN_MIME_TYPE,
                                            ),
                                            null,
                                            null,
                                            null,
                                        )
                                        ?.use { cursor ->
                                            while (cursor.moveToNext()) {
                                                val child = cursor.getString(0)
                                                val mime = cursor.getString(1) ?: ""
                                                if (
                                                    mime == DocumentsContract.Document.MIME_TYPE_DIR
                                                )
                                                    pending.add(child)
                                                else if (mime.startsWith("audio/"))
                                                    found.add(
                                                        DocumentsContract.buildDocumentUriUsingTree(
                                                                uri,
                                                                child,
                                                            )
                                                            .toString(),
                                                    )
                                            }
                                        }
                                }
                                found
                            }
                        runtime.settings.playerItems(
                            runtime.settings.state.value.playerItems + items,
                        )
                        runtime.settings.source("player")
                    } catch (cancelled: CancellationException) {
                        throw cancelled
                    } catch (_: Exception) {
                        captureError = R.string.no_source
                    }
                }
        }

    private fun finishOnboarding() {
        runtime.settings.onboard()
        page = "home"
    }

    private fun requestMicrophone() {
        captureError = null
        if (
            ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) !=
                PackageManager.PERMISSION_GRANTED
        )
            microphone.launch(Manifest.permission.RECORD_AUDIO)
        else if (pendingSource == "onboard") notificationStep = true else afterMicrophone()
    }

    private fun requestListening(source: String, laboratory: Boolean) {
        pendingSource = source
        pendingLab = laboratory
        lifecycleScope.launch {
            if (!laboratory && runtime.settings.read().checkedMic == null) {
                afterCheck = source
                pendingSource = "check"
            }
            requestMicrophone()
        }
    }

    private fun requestCheck() {
        afterCheck = null
        pendingSource = "check"
        pendingLab = false
        requestMicrophone()
    }

    private fun afterMicrophone() {
        if (pendingSource == "check") {
            runtime.updateCheck(CheckSnapshot("checking"))
            page = "check"
            startServiceCapture()
            return
        }
        if (pendingSource != "capture" || runtime.hasProjection) {
            startServiceCapture()
            return
        }
        if (Build.VERSION.SDK_INT < 29) {
            captureError = R.string.capture_needs_10
            return
        }
        lifecycleScope.launch {
            if (runtime.settings.read().explained) launchConsent() else explain = true
        }
    }

    private fun launchConsent() {
        explain = false
        runtime.settings.explained()
        if (Build.VERSION.SDK_INT < 29) return
        val manager = getSystemService(MediaProjectionManager::class.java)
        projectionResult.launch(
            if (Build.VERSION.SDK_INT >= 34)
                manager.createScreenCaptureIntent(
                    MediaProjectionConfig.createConfigForDefaultDisplay(),
                )
            else manager.createScreenCaptureIntent(),
        )
    }

    private fun startServiceCapture(result: Int = RESULT_CANCELED, data: Intent? = null) {
        val source = pendingSource
        val laboratory = pendingLab
        lifecycleScope.launch {
            runtime.serviceStopping.first { !it }
            if (source == "capture" && data == null && !runtime.hasProjection) {
                afterMicrophone()
                return@launch
            }
            try {
                ListeningService.start(this@MainActivity, result, data, laboratory, source)
            } catch (_: RuntimeException) {
                captureError = R.string.capture_error_alert
                if (source == "check") runtime.updateCheck(CheckSnapshot("error"))
            }
        }
    }

    @Suppress("DEPRECATION")
    private fun pipAllowed(): Boolean {
        if (!packageManager.hasSystemFeature(PackageManager.FEATURE_PICTURE_IN_PICTURE))
            return false
        val ops = getSystemService(AppOpsManager::class.java)
        return (if (Build.VERSION.SDK_INT >= 29)
            ops.unsafeCheckOpNoThrow(
                AppOpsManager.OPSTR_PICTURE_IN_PICTURE,
                applicationInfo.uid,
                packageName,
            )
        else
            ops.checkOpNoThrow(
                AppOpsManager.OPSTR_PICTURE_IN_PICTURE,
                applicationInfo.uid,
                packageName,
            )) == AppOpsManager.MODE_ALLOWED
    }

    private fun configurePip(active: Boolean, enabled: Boolean) {
        listening = active
        val allowed = enabled && pipAllowed()
        if (active && !allowed && !pipWarned) {
            pipTip = true
            pipWarned = true
        }
        if (!packageManager.hasSystemFeature(PackageManager.FEATURE_PICTURE_IN_PICTURE)) return
        val builder =
            PictureInPictureParams.Builder()
                .setAspectRatio(Rational(1, 1))
                .setSourceRectHint(Rect().also { window.decorView.getGlobalVisibleRect(it) })
        if (Build.VERSION.SDK_INT >= 31) builder.setAutoEnterEnabled(active && allowed)
        setPictureInPictureParams(builder.build())
    }

    override fun onUserLeaveHint() {
        super.onUserLeaveHint()
        if (
            Build.VERSION.SDK_INT < 31 &&
                listening &&
                runtime.settings.state.value.pip &&
                pipAllowed()
        ) {
            try {
                enterPictureInPictureMode(
                    PictureInPictureParams.Builder().setAspectRatio(Rational(1, 1)).build(),
                )
            } catch (_: IllegalStateException) {
                pipTip = true
            }
        }
    }

    override fun onPictureInPictureModeChanged(
        isInPictureInPictureMode: Boolean,
        newConfig: Configuration,
    ) {
        super.onPictureInPictureModeChanged(isInPictureInPictureMode, newConfig)
        pip = isInPictureInPictureMode
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        configurePip(listening, runtime.settings.state.value.pip)
    }

    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        touchAt = SystemClock.elapsedRealtime()
        restoreBrightness()
        return super.dispatchTouchEvent(event)
    }

    private fun restoreBrightness() {
        window.attributes =
            window.attributes.apply {
                screenBrightness = WindowManager.LayoutParams.BRIGHTNESS_OVERRIDE_NONE
            }
    }

    override fun onSaveInstanceState(outState: Bundle) {
        outState.putString("page", page)
        outState.putString("pending", pendingSource)
        outState.putBoolean("lab", pendingLab)
        outState.putString("after_check", afterCheck)
        outState.putBoolean("notifications", notificationStep)
        outState.putBoolean("pip_warned", pipWarned)
        super.onSaveInstanceState(outState)
    }

    // An explicit Settings action for real-time listening; never requested at startup.
    @android.annotation.SuppressLint("BatteryLife")
    private fun batterySettings() {
        val intent =
            Intent(
                Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS,
                "package:$packageName".toUri(),
            )
        try {
            startActivity(intent)
        } catch (_: android.content.ActivityNotFoundException) {
            startActivity(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS))
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        savedInstanceState?.let {
            page = it.getString("page") ?: "welcome"
            pendingSource = it.getString("pending") ?: "onboard"
            pendingLab = it.getBoolean("lab")
            afterCheck = it.getString("after_check")
            notificationStep = it.getBoolean("notifications")
            pipWarned = it.getBoolean("pip_warned")
        }
        enableEdgeToEdge()
        setContent {
            VeylTheme {
                val capture by runtime.capture.collectAsStateWithLifecycle()
                val prefs by runtime.settings.state.collectAsStateWithLifecycle()
                val active = capture.state in listOf("starting", "running")
                LaunchedEffect(prefs.onboarded) {
                    if (prefs.onboarded && page == "welcome") page = "home"
                }
                LaunchedEffect(active, prefs.dim, prefs.pip) {
                    configurePip(active, prefs.pip)
                    if (active) window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
                    else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
                    touchAt = SystemClock.elapsedRealtime()
                    while (active) {
                        delay(1000)
                        if (prefs.dim && SystemClock.elapsedRealtime() - touchAt >= 10_000)
                            window.attributes = window.attributes.apply { screenBrightness = 0.04f }
                    }
                    restoreBrightness()
                }
                DisposableEffect(Unit) {
                    onDispose {
                        window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
                        configurePip(false, false)
                        restoreBrightness()
                    }
                }
                BackHandler(enabled = page !in listOf("welcome", "home") && !pip) {
                    if (page == "check" && active) ListeningService.stop(this@MainActivity)
                    page =
                        when (page) {
                            "licenses" -> "about"
                            "about" -> "settings"
                            else -> "home"
                        }
                }
                Scaffold { padding ->
                    Column(
                        Modifier.fillMaxSize()
                            .padding(padding)
                            .verticalScroll(rememberScrollState())
                            .padding(28.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(16.dp),
                    ) {
                        if (pip) {
                            Text(
                                stringResource(R.string.app_name),
                                style = MaterialTheme.typography.titleLarge,
                            )
                            Text(stringResource(R.string.no_pc_meter))
                        } else
                            when {
                                page == "check" ->
                                    SoundCheckContent(
                                        runtime,
                                        onContinue = {
                                            val source = afterCheck
                                            afterCheck = null
                                            if (
                                                source != null && runtime.check.value.result != null
                                            ) {
                                                page = "home"
                                                requestListening(source, false)
                                            } else page = "settings"
                                        },
                                        onStop = {
                                            ListeningService.stop(this@MainActivity)
                                            afterCheck = null
                                            page = "home"
                                        },
                                    )
                                BuildConfig.DEBUG && page == "lab" ->
                                    DebugLabContent(
                                        runtime,
                                        { requestListening(it, true) },
                                        { ListeningService.stop(this@MainActivity) },
                                        { page = "home" },
                                        { music.launch(arrayOf("audio/*")) },
                                        { folder.launch(null) },
                                    )
                                active -> {
                                    ListeningContent(capture) {
                                        ListeningService.stop(this@MainActivity)
                                    }
                                    if (capture.source == "player")
                                        PlayerControls(runtime) {
                                            ListeningService.player(this@MainActivity, it)
                                        }
                                }
                                page == "permissions" -> {
                                    Text(
                                        stringResource(
                                            if (notificationStep) R.string.notifications_title
                                            else R.string.microphone_title,
                                        ),
                                        style = MaterialTheme.typography.headlineLarge,
                                    )
                                    Text(
                                        stringResource(
                                            if (notificationStep) R.string.notifications_explanation
                                            else R.string.microphone_explanation,
                                        ),
                                    )
                                    if (!notificationStep)
                                        Button(
                                            onClick = {
                                                pendingSource = "onboard"
                                                requestMicrophone()
                                            },
                                        ) {
                                            Text(stringResource(R.string.allow_microphone))
                                        }
                                    else {
                                        Button(
                                            onClick = {
                                                if (Build.VERSION.SDK_INT >= 33)
                                                    notifications.launch(
                                                        Manifest.permission.POST_NOTIFICATIONS,
                                                    )
                                                else finishOnboarding()
                                            },
                                        ) {
                                            Text(stringResource(R.string.allow_notifications))
                                        }
                                        TextButton(onClick = ::finishOnboarding) {
                                            Text(stringResource(R.string.not_now))
                                        }
                                    }
                                }
                                page == "placement" -> PlacementContent { page = "home" }
                                page == "settings" ->
                                    SettingsContent(
                                        runtime,
                                        ::requestCheck,
                                        ::batterySettings,
                                        { page = "about" },
                                        { page = "home" },
                                    )
                                page == "about" ->
                                    AboutContent(
                                        { startActivity(Intent(Intent.ACTION_VIEW, it.toUri())) },
                                        { page = "licenses" },
                                        { page = "settings" },
                                    )
                                page == "licenses" -> LicensesContent { page = "about" }
                                page == "home" -> {
                                    Text(
                                        stringResource(R.string.no_pc_title),
                                        style = MaterialTheme.typography.headlineLarge,
                                    )
                                    Text(stringResource(R.string.this_phone, prefs.phoneName))
                                    Text(
                                        stringResource(
                                            R.string.address,
                                            stringResource(R.string.not_available),
                                        ),
                                    )
                                    Text(stringResource(R.string.waiting_pc))
                                    Text(stringResource(R.string.listening_ready))
                                    SourceControls(
                                        runtime,
                                        false,
                                        { requestListening(it, false) },
                                        { music.launch(arrayOf("audio/*")) },
                                        { folder.launch(null) },
                                    )
                                    TextButton(onClick = { page = "placement" }) {
                                        Text(stringResource(R.string.placement_link))
                                    }
                                    TextButton(onClick = { page = "settings" }) {
                                        Text(stringResource(R.string.settings))
                                    }
                                }
                                else -> {
                                    Image(
                                        painterResource(R.drawable.ic_launcher_foreground),
                                        contentDescription = null,
                                        modifier = Modifier.size(140.dp),
                                    )
                                    Text(
                                        stringResource(R.string.app_name),
                                        style = MaterialTheme.typography.displaySmall,
                                    )
                                    Text(
                                        stringResource(R.string.tagline),
                                        style = MaterialTheme.typography.headlineMedium,
                                    )
                                    Text(stringResource(R.string.welcome_description))
                                    Button(onClick = { page = "permissions" }) {
                                        Text(stringResource(R.string.get_started))
                                    }
                                }
                            }
                        if (!pip) {
                            if (active && pipTip) Text(stringResource(R.string.pip_tip))
                            captureError?.let {
                                Text(stringResource(it), color = MaterialTheme.colorScheme.error)
                                if (it == R.string.microphone_denied)
                                    TextButton(
                                        onClick = {
                                            startActivity(
                                                Intent(
                                                    Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                                                    "package:$packageName".toUri(),
                                                ),
                                            )
                                        },
                                    ) {
                                        Text(stringResource(R.string.open_settings))
                                    }
                            }
                            if (capture.state == "error" && captureError == null)
                                Text(
                                    stringResource(R.string.capture_error_alert),
                                    color = MaterialTheme.colorScheme.error,
                                )
                            Text(
                                stringResource(R.string.version, BuildConfig.VERSION_NAME),
                                style = MaterialTheme.typography.bodyMedium,
                                modifier =
                                    Modifier.combinedClickable(
                                        onClick = {},
                                        onLongClick = { if (BuildConfig.DEBUG) page = "lab" },
                                    ),
                            )
                        }
                    }
                }
                if (explain)
                    AlertDialog(
                        onDismissRequest = { explain = false },
                        title = { Text(stringResource(R.string.one_more_step)) },
                        text = { Text(stringResource(R.string.consent_explainer)) },
                        confirmButton = {
                            TextButton(onClick = ::launchConsent) {
                                Text(stringResource(R.string.continue_label))
                            }
                        },
                        dismissButton = {
                            TextButton(onClick = { explain = false }) {
                                Text(stringResource(R.string.cancel))
                            }
                        },
                    )
            }
        }
    }
}
