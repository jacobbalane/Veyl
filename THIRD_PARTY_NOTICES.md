# Third-party notices

Veyl's original application code and brand assets are MIT licensed. Third-party components retain
their own licenses. This inventory lists direct dependencies through M4. Bundled
components include their own notices where required.

## Manrope

Manrope is by the Manrope Project Authors and is licensed under the SIL Open Font License 1.1.
The committed source font is from [Google Fonts](https://github.com/google/fonts/tree/main/ofl/manrope).
Desktop and website builds use @fontsource-variable/manrope. Android and brand generation use
assets/fonts/manrope-variable.ttf. Its full license is in assets/fonts/Manrope-LICENSE.txt.
The desktop installer ships its font license under licenses/manrope.txt; Android bundles it in
assets/licenses/manrope.txt; the website serves licenses/manrope.txt.

## npm packages

| Package                      | Locked version | License           |
| ---------------------------- | -------------- | ----------------- |
| @fontsource-variable/manrope | 5.3.0          | OFL-1.1           |
| @playwright/test             | 1.64.1         | Apache-2.0        |
| @resvg/resvg-js              | 2.6.2          | MPL-2.0           |
| @tauri-apps/api              | 2.12.4         | Apache-2.0 OR MIT |
| @tauri-apps/cli              | 2.12.4         | Apache-2.0 OR MIT |
| @testing-library/react       | 16.3.3         | MIT               |
| @types/node                  | 26.6.5         | MIT               |
| @types/react                 | 19.3.0         | MIT               |
| @types/react-dom             | 19.3.0         | MIT               |
| @vitejs/plugin-react         | 6.1.2          | MIT               |
| jsdom                        | 30.1.3         | MIT               |
| lucide-react                 | 1.61.0         | ISC               |
| motion                       | 14.5.0         | MIT               |
| opentype.js                  | 2.0.0          | MIT               |
| prettier                     | 3.9.10         | MIT               |
| react                        | 19.3.1         | MIT               |
| react-dom                    | 19.3.1         | MIT               |
| typescript                   | 7.0.2          | Apache-2.0        |
| vite                         | 8.3.5          | MIT               |
| vitest                       | 5.0.5          | MIT               |

## Rust crates

These are direct external dependencies of the workspace crates, including build and test tools.

| Crate                        | Locked version   | License           |
| ---------------------------- | ---------------- | ----------------- |
| aes-gcm                      | 0.11.1           | Apache-2.0 OR MIT |
| audioadapter-buffers         | 5.2.0            | MIT OR Apache-2.0 |
| base64                       | 0.23.1           | MIT OR Apache-2.0 |
| getrandom                    | 0.4.3            | MIT OR Apache-2.0 |
| hkdf                         | 0.13.0           | MIT OR Apache-2.0 |
| hmac                         | 0.13.0           | MIT OR Apache-2.0 |
| hound                        | 3.5.1            | Apache-2.0        |
| png                          | 0.18.1           | MIT OR Apache-2.0 |
| proptest                     | 1.12.0           | MIT OR Apache-2.0 |
| rubato                       | 5.0.1            | MIT OR Apache-2.0 |
| rustfft                      | 6.4.1            | MIT OR Apache-2.0 |
| serde                        | 1.0.229          | MIT OR Apache-2.0 |
| serde_json                   | 1.0.154          | MIT OR Apache-2.0 |
| sha2                         | 0.11.1           | MIT OR Apache-2.0 |
| subtle                       | 2.6.1            | BSD-3-Clause      |
| tauri                        | 2.12.5           | Apache-2.0 OR MIT |
| tauri-build                  | 2.7.4            | Apache-2.0 OR MIT |
| tauri-plugin-single-instance | 2.5.4            | Apache-2.0 OR MIT |
| tempfile                     | 3.27.0           | MIT OR Apache-2.0 |
| time                         | 0.3.55           | MIT OR Apache-2.0 |
| tokio                        | 1.54.1           | MIT               |
| toml                         | 1.1.8+spec-1.1.0 | MIT OR Apache-2.0 |
| tracing                      | 0.1.44           | MIT               |
| tracing-appender             | 0.2.5            | MIT               |
| tracing-subscriber           | 0.3.23           | MIT               |
| x25519-dalek                 | 3.0.0            | BSD-3-Clause      |
| windows-sys                  | 0.61.2           | MIT OR Apache-2.0 |
| windows                      | 0.62.2           | MIT OR Apache-2.0 |
| windows-core                 | 0.62.2           | MIT OR Apache-2.0 |
| zeroize                      | 1.9.1            | Apache-2.0 OR MIT |

Hound is copyright © 2015 Ruud van Asseldonk. Its Apache 2.0 license is in
`assets/licenses/hound-Apache-2.0.txt`; desktop installers ship it as `licenses/hound.txt`.

Rubato and audioadapter-buffers are by Henrik Enquist. Copies of their MIT licenses are in
`assets/licenses/rubato-MIT.txt` and `assets/licenses/audioadapter-buffers-MIT.txt` and ship in the
desktop installer's `licenses` folder.

RustFFT's MIT license is in `assets/licenses/rustfft-MIT.txt` and ships as `licenses/rustfft.txt`.
The portable sound-check analyzer uses it in production; it was already used by the simulator and
DSP tests.

The engine's parameter loader uses the TOML crate. Its MIT license is in
`assets/licenses/toml-MIT.txt` and ships as `licenses/toml.txt`.

The lab's offline motion plots use the PNG crate. Its MIT license is in
`assets/licenses/png-MIT.txt`. The lab does not upload plots or recordings.

## Android dependencies

| Component                                | Version        | License               |
| ---------------------------------------- | -------------- | --------------------- |
| Android Gradle Plugin                    | 9.4.1          | Apache-2.0            |
| Kotlin, Compose compiler and kotlin-test | 2.4.21         | Apache-2.0            |
| AndroidX Core KTX                        | 1.19.1         | Apache-2.0            |
| AndroidX Activity Compose                | 1.13.0         | Apache-2.0            |
| Compose UI, Material 3 and tooling       | BOM 2026.09.00 | Apache-2.0            |
| JUnit                                    | 4.13.2         | EPL-1.0               |
| kotlinx.serialization JSON               | 1.12.0         | Apache-2.0            |
| Bouncy Castle provider                   | 1.86           | Bouncy Castle Licence |
| kotlinx.coroutines Android               | 1.11.0         | Apache-2.0            |
| AndroidX Lifecycle Compose               | 2.11.0         | Apache-2.0            |
| AndroidX DataStore Preferences           | 1.3.0          | Apache-2.0            |
| AndroidX Media3 ExoPlayer                | 1.11.1         | Apache-2.0            |
| Gradle wrapper                           | 9.8.2          | Apache-2.0            |

AndroidX and Kotlin libraries retain their upstream notices in the distribution. The Gradle
wrapper's upstream license is in android/gradle/wrapper/LICENSE. The Android Licenses screen
bundles Veyl's MIT license, the font and Bouncy Castle notices and AndroidX's Apache 2.0 license.

Bouncy Castle supplies only `org.bouncycastle.math.ec.rfc7748.X25519`; Veyl does not install its
JCA provider. Other link cryptography uses the platform APIs. Its license is included in the
dependency JAR and copied to Android's `assets/licenses/bouncycastle.txt`.

No recorded music is bundled. The test sound is generated by Veyl's own code.
