# Contributing to Veyl

Keep sound local. Never add a cloud service. Never store or upload sound unless the person starts
a test recording. Keep the meter honest. Prefer missing a poke over a wrong click.

Before a pull request, run the checks for the areas you touched in
[development](docs/development.md). Record the changes and verification in
[progress](docs/progress.md). Use Conventional Commits, such as `feat(android): …`, `fix(dsp): …`,
`test(link): …`, `docs: …`, `ci: …` and `chore: …`.

Signal-processing changes need the scenario suite and a before/after table of lab results.
Include real-device results when available, as numbers only. Protocol changes need regenerated
test vectors, both Rust and Kotlin implementations, and updated [protocol docs](docs/protocol.md).
Breaking wire changes require a new protocol version.

UI wording comes from [UI copy](docs/ui-copy.md). Keep motion purposeful, keyboard navigation
clear, focus visible and reduced motion supported. Explain Veyl in plain words for its users.

Use the Device report issue template when it is available. Reports will populate
[compatibility](docs/compatibility.md). Do not attach recordings containing people talking.
Share music recordings privately with maintainers only if asked. Never commit recordings,
secrets, signing keys, local settings or generated installers.

The maintainer owns account setup, signing secrets, tags and publishing. Follow
[release instructions](docs/release.md) when preparing a release.
