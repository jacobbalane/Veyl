package io.github.jacobbalane.veyl.ui

import android.Manifest
import android.app.Activity
import android.app.PictureInPictureParams
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Rect
import android.media.projection.MediaProjectionConfig
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.util.Rational
import android.view.MotionEvent
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.core.net.toUri
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.jacobbalane.veyl.BuildConfig
import io.github.jacobbalane.veyl.R
import io.github.jacobbalane.veyl.VeylApp
import io.github.jacobbalane.veyl.capture.ListeningService
import io.github.jacobbalane.veyl.ui.theme.VeylTheme
import kotlinx.coroutines.delay

class MainActivity : ComponentActivity() {
    private var captureError by mutableStateOf<Int?>(null)
    private var explain by mutableStateOf(false)
    private var touchAt by mutableLongStateOf(0L)
    private var listening = false
    private var pip by mutableStateOf(false)
    private val projectionResult =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            val data = result.data
            if (result.resultCode == Activity.RESULT_OK && data != null)
                ListeningService.start(this, result.resultCode, data, laboratory = true)
            else captureError = R.string.consent_cancelled
        }
    private val notifications =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { requestConsent() }
    private val microphone =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            if (granted) afterMicrophone() else captureError = R.string.microphone_denied
        }

    private fun startLab() {
        captureError = null
        if (
            ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) !=
                PackageManager.PERMISSION_GRANTED
        )
            microphone.launch(Manifest.permission.RECORD_AUDIO)
        else afterMicrophone()
    }

    private fun afterMicrophone() {
        if (
            Build.VERSION.SDK_INT >= 33 &&
                ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) !=
                    PackageManager.PERMISSION_GRANTED
        )
            notifications.launch(Manifest.permission.POST_NOTIFICATIONS)
        else requestConsent()
    }

    private fun requestConsent() {
        explain = true
    }

    private fun launchConsent() {
        explain = false
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

    private fun configurePip(active: Boolean) {
        listening = active
        if (!packageManager.hasSystemFeature(PackageManager.FEATURE_PICTURE_IN_PICTURE)) return
        val builder =
            PictureInPictureParams.Builder()
                .setAspectRatio(Rational(1, 1))
                .setSourceRectHint(Rect().also { window.decorView.getGlobalVisibleRect(it) })
        if (Build.VERSION.SDK_INT >= 31) builder.setAutoEnterEnabled(active)
        setPictureInPictureParams(builder.build())
    }

    override fun onUserLeaveHint() {
        super.onUserLeaveHint()
        if (
            Build.VERSION.SDK_INT < 31 &&
                listening &&
                packageManager.hasSystemFeature(PackageManager.FEATURE_PICTURE_IN_PICTURE)
        ) {
            try {
                enterPictureInPictureMode(
                    PictureInPictureParams.Builder().setAspectRatio(Rational(1, 1)).build(),
                )
            } catch (_: IllegalStateException) {}
        }
    }

    override fun onPictureInPictureModeChanged(
        isInPictureInPictureMode: Boolean,
        newConfig: android.content.res.Configuration,
    ) {
        super.onPictureInPictureModeChanged(isInPictureInPictureMode, newConfig)
        pip = isInPictureInPictureMode
    }

    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        touchAt = android.os.SystemClock.elapsedRealtime()
        window.attributes =
            window.attributes.apply {
                screenBrightness = WindowManager.LayoutParams.BRIGHTNESS_OVERRIDE_NONE
            }
        return super.dispatchTouchEvent(event)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            VeylTheme {
                var started by rememberSaveable { mutableStateOf(false) }
                var lab by rememberSaveable { mutableStateOf(false) }
                val runtime = (application as VeylApp).runtime
                val state by runtime.capture.collectAsStateWithLifecycle()
                val active = state.state in listOf("starting", "running")
                LaunchedEffect(active) {
                    configurePip(active)
                    if (active) window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
                    else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
                    touchAt = android.os.SystemClock.elapsedRealtime()
                    while (active) {
                        delay(1000)
                        if (android.os.SystemClock.elapsedRealtime() - touchAt >= 10_000)
                            window.attributes = window.attributes.apply { screenBrightness = 0.04f }
                    }
                    window.attributes =
                        window.attributes.apply {
                            screenBrightness = WindowManager.LayoutParams.BRIGHTNESS_OVERRIDE_NONE
                        }
                }
                DisposableEffect(Unit) {
                    onDispose {
                        window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
                        configurePip(false)
                        window.attributes =
                            window.attributes.apply {
                                screenBrightness =
                                    WindowManager.LayoutParams.BRIGHTNESS_OVERRIDE_NONE
                            }
                    }
                }
                Scaffold { padding ->
                    Column(
                        modifier =
                            Modifier.fillMaxSize()
                                .padding(padding)
                                .verticalScroll(rememberScrollState())
                                .padding(28.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center,
                    ) {
                        if (pip) {
                            Text(
                                stringResource(R.string.app_name),
                                style = MaterialTheme.typography.titleLarge,
                            )
                            Text(stringResource(R.string.waiting_pc))
                        } else if (BuildConfig.DEBUG && lab) {
                            DebugLabContent(
                                runtime,
                                ::startLab,
                                { ListeningService.stop(this@MainActivity) },
                                { lab = false },
                            )
                        } else if (active) {
                            ListeningContent(state) { ListeningService.stop(this@MainActivity) }
                        } else {
                            Image(
                                painterResource(R.drawable.ic_launcher_foreground),
                                contentDescription = null,
                                modifier = Modifier.size(140.dp),
                            )
                            Text(
                                stringResource(R.string.app_name),
                                style = MaterialTheme.typography.displaySmall,
                            )
                            Spacer(Modifier.height(16.dp))
                            Text(
                                stringResource(
                                    if (started) R.string.no_pc_title else R.string.tagline,
                                ),
                                style = MaterialTheme.typography.headlineMedium,
                                textAlign = TextAlign.Center,
                            )
                            Spacer(Modifier.height(16.dp))
                            Text(
                                stringResource(
                                    if (started) R.string.waiting_pc
                                    else R.string.welcome_description,
                                ),
                                textAlign = TextAlign.Center,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Spacer(Modifier.height(28.dp))
                            Card {
                                Column(
                                    Modifier.padding(20.dp),
                                    verticalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    Text(
                                        stringResource(R.string.foundation_title),
                                        style = MaterialTheme.typography.titleMedium,
                                    )
                                    Text(
                                        stringResource(R.string.foundation_detail),
                                        style = MaterialTheme.typography.bodyMedium,
                                    )
                                }
                            }
                            Spacer(Modifier.height(28.dp))
                            if (!started)
                                Button(onClick = { started = true }) {
                                    Text(stringResource(R.string.get_started))
                                }
                            Spacer(Modifier.height(28.dp))
                            Text(
                                stringResource(R.string.version, BuildConfig.VERSION_NAME),
                                style = MaterialTheme.typography.bodyMedium,
                                modifier =
                                    Modifier.combinedClickable(
                                        onClick = {},
                                        onLongClick = { if (BuildConfig.DEBUG) lab = true },
                                    ),
                            )
                            Text(
                                stringResource(R.string.license),
                                style = MaterialTheme.typography.bodyMedium,
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
                captureError?.let { message ->
                    AlertDialog(
                        onDismissRequest = { captureError = null },
                        text = { Text(stringResource(message)) },
                        confirmButton = {
                            TextButton(
                                onClick = {
                                    captureError = null
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
                        },
                        dismissButton = {
                            TextButton(onClick = { captureError = null }) {
                                Text(stringResource(R.string.back))
                            }
                        },
                    )
                }
            }
        }
    }
}
