# Security policy

## Reporting a vulnerability

**Do not open a public issue.** Use GitHub's private vulnerability reporting:
**Security → Report a vulnerability** on this repository.

Please include steps to reproduce and the affected version or commit. Expect an acknowledgement within
7 days. We aim to release a fix within 90 days and will credit you unless you prefer otherwise.

## Scope

In scope: anything that lets someone other than the vault owner read, alter or roll back vault data, or
that weakens the guarantees in [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md). That includes the
cryptography, sync, autofill origin matching and extension permissions.

Out of scope: the cases listed under "Not protected" in the threat model.

## Supported versions

None yet. Scytale is pre-alpha and unaudited.
