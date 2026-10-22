# Desktop connection feedback

M3 makes pairing and streaming available before echo processing and gestures. The desktop needs
truthful progress, connection errors and transport-level diagnostics during that work. Appendix C
does not yet name every transport measurement or the command to request the phone's start notice.

Use the existing C.2 pairing, phone-list, status and diagnostic copy. Add C.7 for the connection
preview notice, manual-address validation, disconnect/start-request controls and signal levels.
The notice disappears when the full release is ready. Starting remains a deliberate action on the
phone; the PC only requests its notification. Diagnostics contain metadata and levels, with no
audio, protected link keys or comparison codes. The meter stays at No sound until M5 connects the
echo engine; transport levels never masquerade as a sound-quality score.

Windows continues to use system DNS-SD. Manual addressing remains available when discovery fails.
No desktop listening socket or firewall-rule change is introduced. Device acceptance remains
pending the maintainer's final review.
