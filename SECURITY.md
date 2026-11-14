# Security

Firmware and downloaded packages are untrusted inputs. Do not execute host
scripts from a firmware archive. Integrity checks prove agreement with the
catalog digest, not that firmware is safe or that its author is authenticated.

Report suspected security problems privately using the repository’s
**Security → Report a vulnerability** feature when enabled. If it is unavailable,
ask the maintainer for a private reporting channel without posting exploit
payloads, credentials or personal saves in a public issue.

Only the latest application release is maintained. There is no guaranteed
response time. `tools/studio_security_audit.py` records dependency advisory
results; known maintenance warnings and platform limitations are described in
the compatibility report. Do not interpret a successful build as a security audit.
