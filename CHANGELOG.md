# Changelog

## 0.2.0 — Rust MVP rebuild

This release replaces the exploratory Python prototype with a Rust workspace containing `vdb-core` and the `vdb` CLI. The core provides append-only CBOR WAL records with SHA-256 checksums, replay-based recovery, collections, nested JSON-like documents, optimistic versions, bounded equality queries, schema reports, health metrics, snapshot manifests, and backup verification.

The CLI supports initialization, collection creation/listing, document put/get/query/delete, schema inspection, health reporting, read-only Steward findings, backup creation, and backup verification. The repository includes architecture, security, performance, data-format, operations, testing, demand-validation, and roadmap documentation, plus GitHub Actions quality checks.

This remains an MVP. It does not yet provide encryption at rest, authentication, multi-process locking, secondary indexes, compaction, network serving, replication, or autonomous AI changes.
