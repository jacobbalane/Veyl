# 0026: Wording the interface needs beyond Appendix C, and how links open

Date: 2026-10-26

Building the Live tab, Settings, the poke test, onboarding and Help (M5.2, M6 and M7.1) needed
text that Appendix C does not give. `docs/ui-copy.md` adds it in two sections, so all interface
text still has one source:

- **C.10 Live engine feedback:** the running state ("Watching for pokes"), the suspension when the
  source cannot be used, the three engine warnings in plain words (a sudden noise reuses the
  `noisy_room` tip), the echo view's "poke" label and the poke-delay rows in Diagnostics.
- **C.11 Desktop actions, settings, poke test, onboarding and Help:** how hotkeys are written (the
  Windows key as Win), the hotkey recorder's prompt and errors, settings notes and errors, poke
  test and onboarding controls, the Help privacy summary and About labels, and a short fix for
  every tip and connection error. The Help tab shows these fixes beside the Appendix C text and
  links to the troubleshooting guide for more.

The M3 "Connection preview" banner said gesture control was coming; it is retired now that pokes
act. The version moved to Help → About.

**Choices.** Closing onboarding with Escape or × hides it until the next start; only Finish saves
`onboarding_done`, so a first-time user who closes it by accident sees it again. Help lists the
tips and errors of the current mode; the two modes together cover every tip. A settings change
that Rust saved but could not hand to the engine (`engine_busy`) keeps the new value on screen,
because it is saved and applies when the engine restarts.

**Links.** Help's links used `target="_blank"`, and whether the app window hands such links to
the browser was not tested. A new `open_link` command opens them in the default browser through
the opener crate, and accepts only Veyl's website and repository pages, so privacy rule 4 holds
and no page can make Veyl open other addresses. The browser preview opens them in a new tab.
