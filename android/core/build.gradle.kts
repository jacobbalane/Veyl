plugins {
    alias(libs.plugins.kotlin.jvm)
}
kotlin {
    jvmToolchain(17)
}
dependencies {
    implementation(libs.bouncycastle)
    implementation(libs.kotlinx.serialization.json)
    testImplementation(libs.kotlin.test)
    testImplementation(libs.junit)
}
tasks.test {
    useJUnit()
}

sourceSets.test {
    resources.srcDir("../../protocol/test-vectors")
}
