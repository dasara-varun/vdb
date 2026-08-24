# VDB Security Policy

## Scope

This policy applies to the VDB source code, CLI, storage format, release workflows, documentation that affects security behavior, and future packaged artifacts. The current MVP is local-first, plaintext, and not production-ready; its security model and limitations are documented in [`docs/security.md`](docs/security.md).

## Reporting a vulnerability

Please do not disclose a suspected vulnerability in a public issue before maintainers have had an opportunity to assess it. Use a private GitHub Security Advisory for this repository when available. If private advisories are unavailable, contact the repository maintainers through a private GitHub channel and include only the minimum information needed to reproduce the issue.

A useful report includes the affected commit or release, operating system and filesystem, command or API entrypoint, a minimal non-sensitive reproducer, impact, and any suggested mitigation. Do not include credentials, encryption keys, private database files, personal data, or unpublished exploit code in public reports.

## Response expectations

Maintainers should acknowledge a private report, reproduce it in an isolated environment, classify its impact, identify affected versions, prepare a regression test, and document the remediation. Security fixes should be reviewed independently, released with a clear advisory, and backported only when the compatibility and risk are understood.

The project does not promise a fixed response time while it remains an MVP. Reports involving data loss, arbitrary code execution, authentication bypass, secret exposure, or unsafe model/tool authority receive priority.

## Dependency and release security

Dependency changes require review of their source, license, advisory status, and transitive graph. The desired release posture is an authentic committed crates.io `Cargo.lock`, locked CI, RustSec auditing, dependency update pull requests, checksummed release archives, and target-platform validation. Until those gates are complete, VDB releases must retain the MVP limitations in the release notes.

## Safe disclosure

Security research should avoid accessing data that does not belong to the researcher, bypassing access controls, degrading shared services, or testing against production systems. When in doubt, stop and request maintainer guidance.

_Last reviewed: 2026-08-24._
