plugins {
    alias(libs.plugins.kotlin.jvm)
}
kotlin {
    jvmToolchain(17)
}
dependencies {
    testImplementation(libs.kotlin.test)
    testImplementation(libs.junit)
}
tasks.test {
    useJUnit()
}
