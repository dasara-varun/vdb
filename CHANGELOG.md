# Changelog

## Unreleased — Audit hardening

The storage audit now acquires the per-instance lock before creating a new `VDB1` header, returns typed errors instead of panicking on recoverable collection, WAL, and serialization assumptions, and validates WAL length decoding without an unchecked conversion. WAL compaction writes and synchronizes a same-directory temporary file, atomically replaces the active path, reopens the append handle using version-preserving `SnapshotPut` records, and is covered by a regression test that verifies indexes, reopen, and subsequent writes. Replay now streams the WAL, validates semantic record references and version sequences, and supports format versions 1 and 2. JSON Lines imports and configured document limits are bounded, new database artifacts use private Unix modes, common symlink path hazards are rejected, and backup/export/manifest destinations refuse overwrites. New adversarial tests cover oversized imports, invalid version sequences, legacy format reopening, output overwrite protection, Unix permissions, symlink rejection, larger configured documents, and configuration limits.

Documentation now records the exact header and compaction boundary, platform-dependent `rename` semantics, backup and stale-lock procedures, a cross-platform crash/fuzz testing strategy, dependency reproducibility constraints, and a ranked improvement plan covering encryption, advisory locks, memory-bounded storage, APIs, query planning, dependency auditing, and the constrained Steward roadmap. The repository also adds `AGENTS.md`, `CONTRIBUTING.md`, a central documentation index, a living project-status snapshot, documentation-maintenance rules, ADR guidance, reusable templates, and a pull-request checklist so future people and AI agents can keep project knowledge synchronized.


## 0.2.0 — Rust MVP rebuild

This release replaces the exploratory Python prototype with a Rust workspace containing `vdb-core` and the `vdb` CLI. The core provides append-only CBOR WAL records with SHA-256 checksums, replay-based recovery, collections, nested JSON-like documents, optimistic versions, bounded equality queries, schema reports, health metrics, snapshot manifests, and backup verification.

The CLI supports initialization, collection creation/listing, document put/get/query/delete, schema inspection, equality-index creation/listing, health reporting, read-only Steward findings, backup creation/verification, JSON Lines export/import, and WAL compaction. The repository includes architecture, security, performance, data-format, operations, testing, demand-validation, production-gap, Steward, and roadmap documentation, plus GitHub Actions quality checks.

This remains an MVP. It does not yet provide encryption at rest, authentication, network serving, replication, or autonomous AI changes. The local core now includes a single-process lock, versioned/checksummed records, equality indexes, JSON Lines portability, and safe WAL compaction.
