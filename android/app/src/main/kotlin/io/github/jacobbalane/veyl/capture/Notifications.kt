package io.github.jacobbalane.veyl.capture

import android.Manifest
import android.app.*
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import io.github.jacobbalane.veyl.R
import io.github.jacobbalane.veyl.ui.MainActivity

object Notifications {
    const val LISTENING_ID = 48

    fun create(context: Context) {
        context
            .getSystemService(NotificationManager::class.java)
            .createNotificationChannels(
                listOf(
                    NotificationChannel(
                        "listening",
                        context.getString(R.string.listening_channel),
                        NotificationManager.IMPORTANCE_LOW,
                    ),
                    NotificationChannel(
                        "alerts",
                        context.getString(R.string.alerts_channel),
                        NotificationManager.IMPORTANCE_DEFAULT,
                    ),
                ),
            )
    }

    private fun open(context: Context): PendingIntent =
        PendingIntent.getActivity(
            context,
            0,
            Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

    fun listening(context: Context): Notification =
        NotificationCompat.Builder(context, "listening")
            .setSmallIcon(R.drawable.ic_launcher_monochrome)
            .setContentTitle(context.getString(R.string.listening_notification))
            .setContentText(context.getString(R.string.notification_waiting))
            .setContentIntent(open(context))
            .setOngoing(true)
            .setSilent(true)
            .addAction(
                0,
                context.getString(R.string.stop),
                PendingIntent.getService(
                    context,
                    1,
                    Intent(context, ListeningService::class.java).setAction(ListeningService.STOP),
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
                ),
            )
            .build()

    fun alert(context: Context, reason: String) {
        if (
            Build.VERSION.SDK_INT >= 33 &&
                ContextCompat.checkSelfPermission(
                    context,
                    Manifest.permission.POST_NOTIFICATIONS,
                ) != PackageManager.PERMISSION_GRANTED
        )
            return
        val text =
            when (reason) {
                "screen_locked" -> R.string.screen_locked_alert
                "thermal" -> R.string.thermal_alert
                "no_pc_timeout" -> R.string.no_pc_timeout_alert
                else -> R.string.capture_error_alert
            }
        context
            .getSystemService(NotificationManager::class.java)
            .notify(
                49,
                NotificationCompat.Builder(context, "alerts")
                    .setSmallIcon(R.drawable.ic_launcher_monochrome)
                    .setContentTitle(context.getString(R.string.app_name))
                    .setContentText(context.getString(text))
                    .setStyle(NotificationCompat.BigTextStyle().bigText(context.getString(text)))
                    .setContentIntent(open(context))
                    .setAutoCancel(true)
                    .build(),
            )
    }
}
