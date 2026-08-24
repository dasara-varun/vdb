# VDB Security and Safety Model

## Security objective

VDB reduces risk by reducing authority. The database remains the source of truth, validation and replay are deterministic, and the Steward is an untrusted analyst. VDB should fail closed when a request is malformed, over budget, outside scope, or inconsistent with the storage contract.

The current MVP is local-first and does not provide authentication, remote serving, encryption at rest, or a model with mutation authority. Security statements in this document distinguish **implemented baseline controls** from **planned production controls**.

## Implemented baseline posture

The Rust core currently applies the following controls:

| Control | Current behavior | Security value and limitation |
|---|---|---|
| Local process boundary | One database path uses a per-instance lock file | Prevents ordinary concurrent opens; the lock is not yet an OS advisory lock and may remain after a crash |
| Creation ordering | The lock is acquired before a new header is created | Prevents first-open initialization races |
| File permissions | New database, lock, backup, manifest, and compaction files use mode `0600` on Unix; existing database files are tightened when opened | Reduces accidental exposure to other local users; Windows uses its native permission model and requires separate validation |
| Symlink handling | Database, backup, manifest, and compaction target paths reject existing symbolic links | Prevents common path-redirection mistakes; a complete race-free no-follow guarantee requires platform-specific open flags or a dedicated abstraction |
| Storage format | `VDB1` header and explicit supported format versions are validated before replay | Prevents silent interpretation of unknown formats |
| Record integrity | Length-bounded CBOR records include SHA-256 checksums | Detects accidental or incomplete changes; SHA-256 is not encryption or attacker authentication |
| Quota enforcement | `VdbOptions` defaults to a 512 MiB WAL quota and rejects writes or oversized compaction replacements above the configured limit | Prevents ordinary append growth beyond an operator-selected bound; it is not a filesystem quota and does not bound the in-memory state |
| Replay validation | Collection names, IDs, documents, metadata, version sequencing, and record references are checked before state application | Rejects malformed or semantically inconsistent WAL data instead of manufacturing state |
| Input and storage budgets | Document size, configured maximum size, query result count, WAL record size, JSON Lines record size, and configured WAL size are bounded | Reduces memory, disk-exhaustion, and denial-of-service risk; quota checks happen before normal append and compaction replacement; the complete working set is still in memory |
| Steward authority | Deterministic findings are read-only and do not execute generic commands | Limits prompt-injection impact; a future model adapter needs additional isolation |

Rust’s Unix `OpenOptionsExt::mode` sets permissions for newly created files and the operating system applies the `umask` to the requested mode.[1] The implementation uses this facility only on Unix and documents the platform boundary rather than pretending that one permission API is portable.

## Default posture

Operators should run VDB as a non-root user in a user-owned directory with restrictive directory permissions. Sensitive data should use filesystem or volume encryption until database-level authenticated encryption is implemented. Remote access, external model calls, and automatic changes are not available in the MVP and should remain opt-in in future releases.

## Identity and authorization

The MVP has no network authentication or multi-user authorization. Future application, administrator, backup, and Steward identities must be separate. Any future action capability must be short-lived, scoped to an instance and collection, restricted by operation type, bounded by resource budget, and recorded in the audit ledger. The authorization model should be deny-by-default: a capability must specify its actor, resource, operation, expiry, budget, and approval requirement.

## AI threat model

| Threat | Example | Required control |
|---|---|---|
| Prompt injection | A document contains instructions telling the Steward to export all data | Treat documents as untrusted data; isolate policy from retrieved content; validate actions deterministically |
| Excessive agency | A model attempts a shell command or privilege change | Typed tools only; no generic shell; least privilege; approval gates |
| Sensitive-data disclosure | Raw documents enter a remote prompt or audit log | Redaction, field masking, local inference default, output scanning |
| Data poisoning | Malicious feedback changes future recommendations | Approved feedback only; versioned memory; evaluation before deployment |
| Insecure model output | Natural language is interpreted as an executable query | Require schema-validated plans and explicit operation allowlists |
| Supply-chain compromise | A plugin or model package introduces unsafe behavior | Pinned versions, signatures, provenance, SBOM, sandboxed extensions, advisory scans |
| Audit tampering | An attacker deletes evidence of an AI action | Append-only events, hash chaining or remote export, restricted deletion |

## Data minimization and secrets

The Steward should receive metrics, query fingerprints, schema summaries, and redacted samples rather than the complete database. Sensitive fields should be classified using explicit user rules plus conservative heuristics. Raw data access should be a separate, visible diagnostic mode with approval and automatic expiration.

Do not place credentials, encryption keys, tokens, or private source material in document payloads, examples, fixtures, logs, manifests, or issue reports. Secret scanning belongs in CI and local review. Errors should expose enough information to recover while avoiding raw payloads and credentials.

## Encryption and key management plan

The MVP stores database files, WAL content, backups, and manifests as plaintext. Checksums provide integrity detection but do not provide confidentiality or authentication. OWASP recommends selecting cryptography from a threat model, minimizing sensitive data, using established primitives, and treating key management as a distinct design problem.[2]

The production design should use a reviewed authenticated-encryption construction, key identifiers and rotation, secure provider boundaries, backup key handling, revocation and recovery procedures, and tests for wrong keys, tampering, rotation, and restore. VDB must not invent a custom cipher, password derivation scheme, or key-wrapping protocol.

## Audit events and recovery

Every future privileged event should record the timestamp, actor, capability, request hash, target, policy decision, approval, before/after configuration, result, error, and rollback reference. Audit records should be exportable for incident analysis, integrity-protected, and free of raw secrets.

Backup success means more than a file being written. VDB must record snapshot checksums, metadata, source instance version, encryption key identifier when encryption exists, and last restore verification. Restore tests should use an isolated destination and compare checksums, document counts, and representative reads. The current backup manifest is a checksum and size verification mechanism, not an authenticated security envelope.

## Secure development requirements

The project should maintain dependency reproducibility, secret scanning, static analysis, bounded parsers, property and fuzz tests for document and WAL boundaries, crash-injection tests for durability, cross-platform filesystem tests, and adversarial tests for prompt injection and unsafe action plans. Security-sensitive changes require a threat-model update, regression coverage, and explicit limitation text.

## References

[1]: https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html "Rust OpenOptionsExt documentation"
[2]: https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html "OWASP Cryptographic Storage Cheat Sheet"
