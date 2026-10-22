# Security

## Reporting a vulnerability

Report privately through [GitHub's private vulnerability reporting](https://github.com/jacobbalane/Veyl/security/advisories/new).
Include desktop and phone app versions, Windows and Android versions, and steps to reproduce.
Never attach real recordings, pairing codes or keys. Repository setup must enable private
reporting in M0.9; until then, do not put sensitive findings in public issues.

Useful areas include pairing, the session handshake, sealed frames, capture services, the privacy
filter, DPAPI and Android Keystore storage, and installation.

## Supported versions

Only the latest release will receive security fixes. Veyl is currently in development; the
foundation has no verified release yet.

## Verifying a download

Every release is planned to include a SHA-256 checksum beside each installer and APK. Compare it
with a local hash, then verify build provenance:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath .\Veyl_0.0.1.apk
gh attestation verify Veyl_0.0.1.apk --repo jacobbalane/Veyl
```

Attestations identify the build source. They do not establish sensing accuracy or device safety.

## Threat model

These are design requirements for the forthcoming Veyl Link implementation.

| Threat                                 | Result                                                                           |
| -------------------------------------- | -------------------------------------------------------------------------------- |
| Someone on the same Wi-Fi listens      | Sees DNS-SD phone name and model, plus encrypted traffic                         |
| Active attacker during pairing         | Must make the compared codes match by chance; phone rate limits cap attempts     |
| A rogue PC requests pairing            | Requires confirmation while Veyl is open on the phone                            |
| Malware steals a pairing key on the PC | Could impersonate that PC; the phone shows the connection and lets you forget it |
| Recorded traffic is replayed           | Fresh session keys and strict counters reject it                                 |
| Flooding the phone's port              | Out of scope; additional connections are dropped while a PC is connected         |

The privacy filter makes speech very hard to understand; it does not make recovery impossible.
See [privacy](docs/privacy.md) for capture and retention requirements.
