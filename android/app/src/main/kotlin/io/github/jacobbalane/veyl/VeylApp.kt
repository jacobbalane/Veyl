package io.github.jacobbalane.veyl

import android.app.Application
import io.github.jacobbalane.veyl.runtime.VeylRuntime

class VeylApp : Application() {
    lateinit var runtime: VeylRuntime
        private set

    override fun onCreate() {
        super.onCreate()
        runtime = VeylRuntime(this)
    }
}
