# Changelog

## 0.2.0 — Rust MVP rebuild

This release replaces the exploratory Python prototype with a Rust workspace containing `vdb-core` and the `vdb` CLI. The core provides append-only CBOR WAL records with SHA-256 checksums, replay-based recovery, collections, nested JSON-like documents, optimistic versions, bounded equality queries, schema reports, health metrics, snapshot manifests, and backup verification.

The CLI supports initialization, collection creation/listing, document put/get/query/delete, schema inspection, equality-index creation/listing, health reporting, read-only Steward findings, backup creation/verification, JSON Lines export/import, and WAL compaction. The repository includes architecture, security, performance, data-format, operations, testing, demand-validation, production-gap, Steward, and roadmap documentation, plus GitHub Actions quality checks.

This remains an MVP. It does not yet provide encryption at rest, authentication, network serving, replication, or autonomous AI changes. The local core now includes a single-process lock, versioned/checksummed records, equality indexes, JSON Lines portability, and safe WAL compaction.
