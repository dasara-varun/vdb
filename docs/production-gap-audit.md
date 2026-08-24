# VDB Production Gap Audit

## Verified repository state

The selected repository is `dasara-varun/vdb`, its default branch is `main`, and the Rust rebuild and its documentation are maintained in the repository. This audit covers the security and reliability hardening committed as `8820310`; GitHub Actions run `32701586336` completed successfully.

## Current MVP status

VDB is not finished as a production database. It is a working Rust MVP with a durable append-only CBOR WAL, `VDB1` header/version checks, per-record SHA-256 checksums, bounded streaming replay, semantic record validation, replay recovery, document versions, bounded queries and imports, private Unix file modes, common-path symlink rejection, schema reports, health diagnostics, backups, backup verification, read-only Steward output, CLI workflows, tests, release benchmarking, and CI. Compaction now uses version-preserving snapshot records in a same-directory temporary file and atomic path replacement rather than in-place truncation. Startup locking occurs before header initialization, and avoidable production panic paths now return typed errors.

## Highest-value next work

| Priority | Gap | Why it matters | Next action |
|---|---|---|---|
| Done | First-open race prevention | Two processes must not initialize the same new path concurrently | Acquire the per-instance lock before header creation; keep regression coverage |
| Done | File-format guard | A durable database must reject incompatible files safely | Validate `VDB1` magic and supported versions 1–2 before replay; use version-2 snapshot records for compacted files |
| Done | Checksummed replay | Complete but modified records must not be silently accepted | Fail closed on checksum mismatch and retain recovery guidance |
| Done baseline | Input and state validation | Malformed imports or semantically forged records can cause memory or state corruption | Bound JSON Lines records and configuration, stream WAL replay, validate IDs/documents/metadata/references/version sequences, and reject common symlink hazards |
| Done baseline | Private local artifacts | Database, lock, backup, manifest, or temporary files can expose sensitive data to other local users | Apply Unix mode `0600` on creation and existing-file open; retain platform-specific limitations in the security guide |
| Done | Export/import escape hatch | Users need a human-readable recovery and migration path | Maintain JSON Lines export/import and add broader CLI contract tests |
| Done | Secondary equality indexes | Repeated full scans become slow | Maintain single-field equality indexes with WAL-persisted definitions and reopen rebuild |
| Done | Safer compaction baseline | In-place truncate/rewrite can destroy the only copy after interruption | Use synchronized same-directory temporary output, replacement, reopen, and post-compaction regression tests |
| P0 | Durability proof across filesystems | File replacement and directory-entry persistence vary by platform | Add directory synchronization where supported, a platform matrix, and crash tests |
| P0 | Encryption and key management | Current files and manifests are plaintext; SHA-256 is not confidentiality or authentication | Adopt reviewed AEAD, key rotation, secure providers, and encrypted restore drills |
| P0 | Reproducible dependency graph | The current sandbox cannot produce a valid lockfile, so dependencies are not yet pinned in Git | Generate an authentic crates.io lockfile in a clean environment, commit it, use `--locked`, and add RustSec checks |
| P1 | Memory-bounded storage | Full state and indexes are rebuilt in memory | Add segments, checkpoints, bounded caches, and size limits; streaming replay is implemented but the materialized working set remains memory-bound only by available process memory |
| P1 | Local application API | CLI-only access limits integration | Add a localhost-only or IPC API with strict validation and explicit exposure controls |
| P1 | Query planning | Current indexes cover simple equality and filtering still occurs in memory | Add explain plans, selectivity metrics, and only then consider compound/range indexes |
| P2 | Stale-lock recovery | A lock file can survive an abnormal process exit | Prefer OS advisory locking or a carefully scoped liveness protocol; never auto-delete a live lock |
| P2 | Model-backed Steward | Deterministic findings are safe but limited | Add optional private model explanation behind minimized context, typed output, approval, and red-team tests |
| P2 | Replication | Distributed correctness adds ordering, conflict, identity, and operational complexity | Defer until storage, recovery, encryption, API, and observability are mature |

## Decision

Continue with durability evidence, encryption, dependency reproducibility, memory-bounded storage, and a narrow local API before adding AI autonomy or distributed replication. This sequence reduces data-loss, security, and lock-in risk while improving day-one usability. See [`docs/improvement-plan.md`](improvement-plan.md) for the full rationale and evidence requirements.
