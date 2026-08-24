# VDB Production Gap Audit

## Verified repository state

The selected repository is `dasara-varun/vdb`, its default branch is `main`, and the Rust rebuild and its documentation are maintained in the repository. This audit covers the security and reliability hardening currently being extended on `main`; the exact commit and CI run must be refreshed at each handoff.

## Current MVP status

VDB is not finished as a production database. It is a working Rust MVP with a durable append-only CBOR WAL, `VDB1` header/version checks, per-record SHA-256 checksums, bounded streaming replay, semantic record validation, replay recovery, document versions, bounded queries and imports, private Unix file modes, common-path symlink rejection, schema reports, health diagnostics, backups, backup verification, read-only Steward output, CLI workflows, tests, release benchmarking, and CI. Compaction now uses version-preserving snapshot records in a same-directory temporary file and atomic path replacement rather than in-place truncation. Startup locking occurs before header initialization, and avoidable production panic paths now return typed errors.

## Highest-value next work

| Priority | Gap | Why it matters | Next action |
|---|---|---|---|
| Done | First-open race prevention | Two processes must not initialize the same new path concurrently | Acquire the per-instance lock before header creation; keep regression coverage |
| Done | File-format guard | A durable database must reject incompatible files safely | Validate `VDB1` magic and supported versions 1–2 before replay; use version-2 snapshot records for compacted files |
| Done baseline | Checksummed replay | Complete but modified or semantically inconsistent records must not be silently accepted | Fail closed on checksum mismatch, oversized length prefixes, duplicate snapshot documents, and invalid version sequences; retain recovery guidance |
| Done baseline | Input and state validation | Malformed imports or semantically forged records can cause memory or state corruption | Bound and prevalidate JSON Lines batches, stream WAL replay, validate IDs/documents/metadata/references/version sequences, use consistent nested equality paths, require intact backup manifests, and reject common symlink hazards |
| Done baseline | WAL storage quota | Unbounded append growth can exhaust disk or make restart/recovery operationally unsafe | Reject writes before append when the configured quota would be exceeded; expose the bound in health and CLI |
| Done baseline | Private local artifacts | Database, lock, backup, manifest, or temporary files can expose sensitive data to other local users | Apply Unix mode `0600` on creation and existing-file open; retain platform-specific limitations in the security guide |
| Done baseline | Export/import escape hatch | Users need a human-readable recovery and migration path without surprising partial state | Maintain bounded atomic JSON Lines export/import, collision-safe outputs, and broader CLI contract tests |
| Done | Secondary equality indexes | Repeated full scans become slow | Maintain single-field equality indexes with WAL-persisted definitions and reopen rebuild |
| Done | Safer compaction baseline | In-place truncate/rewrite can destroy the only copy after interruption | Use synchronized same-directory temporary output, replacement, reopen, and post-compaction regression tests |
| Done baseline | Cross-platform release packaging | Users need verifiable native installation artifacts | Build target-specific archives with checksums; certify platform durability separately before calling targets production-supported |
| P0 | Durability proof across filesystems | File replacement and directory-entry persistence vary by platform | Native workspace tests now run on Ubuntu, macOS, and Windows; Unix parent-directory synchronization is attempted at creation/replacement boundaries, but platform-specific replacement tests and bounded crash tests using the [`durability-matrix.md`](durability-matrix.md) evidence contract remain required |
| P0 | Encryption and key management | Current files and manifests are plaintext; SHA-256 is not confidentiality or authentication | Adopt reviewed AEAD, key rotation, secure providers, and encrypted restore drills |
| Done baseline / P0 follow-up | Reproducible dependency graph | Dependency drift and known vulnerable transitive crates must be detectable | Stable Cargo generated and committed the lockfile; hosted CI uses `--locked`, checks lockfile freshness, and runs the pinned RustSec audit action. License/source policy and SBOM generation remain follow-up work |
| P1 | Memory-bounded storage | Full state and indexes are rebuilt in memory and can exceed a predictable byte budget | The configurable document-count cap limits one growth dimension; add byte-accounted segments, checkpoints, bounded caches, and size limits. Streaming replay alone does not bound the materialized working set |
| P1 | Local application API | CLI-only access limits integration | The read-only GUI is not an authenticated application API; design a separate localhost/IPC contract with strict authorization, resource limits, and auditability before adding mutation or remote clients |
| P1 | Query planning | Current indexes cover simple equality and filtering still occurs in memory | Add explain plans, selectivity metrics, and only then consider compound/range indexes |
| Done baseline / P0 follow-up | Stale-lock recovery | A lock file can survive an abnormal process exit | Unix now uses a safe nonblocking advisory lock and reuses stale regular lock files after process exit; retain the non-Unix fallback until cross-platform behavior is validated |
| Done baseline / P1 follow-up | Local read-only GUI | CLI-only inspection is inconvenient for developers, but an early network API would expand the trust boundary | Provide an explicit CLI-launched browser GUI bound only to `127.0.0.1`, with embedded assets, bounded `GET` routes, HTML escaping, and no mutation; defer authenticated API and desktop packaging |
| P2 | Model-backed Steward | Deterministic findings are safe but limited | Add optional private model explanation behind minimized context, typed output, approval, and red-team tests |
| P2 | Replication | Distributed correctness adds ordering, conflict, identity, and operational complexity | Defer until storage, recovery, encryption, API, and observability are mature |

## Decision

Continue with durability evidence, encryption, dependency reproducibility, memory-bounded storage, and an authenticated narrow local API before adding AI autonomy or distributed replication. The opt-in loopback read-only GUI improves day-one usability but does not satisfy or reorder any P0 production gate; it is not remote serving, authentication, encryption, or a production certification. The durability matrix records current logical guarantees and unresolved filesystem assumptions, but it is a plan and evidence baseline—not production certification. The new WAL and document-count quotas, packaging, and process-crash smoke baselines reduce immediate operational risk without closing the P0 gates. See [`docs/gui-research.md`](gui-research.md), [`docs/decisions/0002-loopback-read-only-gui.md`](decisions/0002-loopback-read-only-gui.md), and [`docs/improvement-plan.md`](improvement-plan.md) for the rationale and evidence requirements.
