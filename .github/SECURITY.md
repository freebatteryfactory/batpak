# Security

Please report suspected vulnerabilities privately through the repository
security advisory flow before opening a public issue:
<https://github.com/freebatteryfactory/batpak/security/advisories/new>.

Security-sensitive changes should preserve:

- sync-only production runtime
- explicit trust boundaries
- append-only event durability
- hash-chain integrity (blake3 is a mandatory dependency, always on)
- traceability and auditability of behavior changes
