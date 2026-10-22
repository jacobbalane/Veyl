# 0005 — Local network permission on Android 17

Date: 2026-10-22. Status: M3 implementation.

## Context

The app targets SDK 37. Current [Android local-network documentation](https://developer.android.com/privacy-and-security/local-network-permission)
requires `ACCESS_LOCAL_NETWORK` at runtime for apps targeting Android 17, including incoming TCP
and NSD. The plan's permission table predates that requirement. Its phone is a server, so an
outgoing service-picker grant cannot replace permission for accepting PC connections.

## Decision

Declare the local-network permission and ask on Android 17+ with a short explanation. Refusal
continues to Home with an explanation and Open settings; do not bind or advertise without the
grant. Recheck on return and on permission changes, stopping the link if access is revoked.
Earlier Android versions continue with the normal Internet/network-state permissions. Keep the
listener and advertising limited to the activity/service lifetime from the plan. Use a multicast
lock only on older Android versions that require it for NSD reception, and release it with NSD.

Add the exact network permission and pairing-storage error wording in Appendix C.6 and `docs/ui-copy.md`. This does not
change Veyl Link's bytes, pairing confirmation, encryption or permitted network destinations.

## Consequences

Android 17 gets one additional system permission step. Denial is visible instead of presenting a
phone that silently cannot accept the PC. Older phones get no extra runtime prompt. Real-device
permission and discovery results remain pending until the final app review.
