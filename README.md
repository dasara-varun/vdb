# VDB

**VDB** is a fast, local-first document database written in Rust. It is designed to feel as simple as creating a document while giving developers safe storage, bounded queries, verified backups, and understandable database health guidance.

> VDB stores your documents simply, proves that your backups work, explains database problems, and never changes important data without your approval.

## Current MVP

The current Rust rebuild provides a native workspace with a `vdb-core` library and `vdb` CLI. It uses an append-only, length-prefixed CBOR log, an in-memory document index, optimistic document versions, bounded equality queries, schema reports, health metrics, and snapshot manifests.

The Steward is currently deterministic and read-only. It reports safe findings rather than executing model-generated commands. A future local/private model may explain findings, but all changes must pass typed validation, policy, approval, and verification.

## Quick start

```bash
cargo run -- init --path ./demo.vdb
cargo run -- --path ./demo.vdb collections create users
cargo run -- --path ./demo.vdb put users u_123 '{"name":"Asha","plan":"pro"}'
cargo run -- --path ./demo.vdb get users u_123
cargo run -- --path ./demo.vdb query users --where-json '{"plan":"pro"}'
cargo run -- --path ./demo.vdb health
cargo run -- --path ./demo.vdb steward --collection users
cargo run -- --path ./demo.vdb index-create users plan
cargo run -- --path ./demo.vdb backup ./backups/demo.vdb
cargo run -- --path ./demo.vdb backup-verify ./backups/demo.vdb
cargo run -- --path ./demo.vdb export ./backups/users.jsonl
cargo run -- --path ./demo.vdb compact
```

After installation, the binary can be invoked as `vdb`.

## Build and test

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release
```

The development sandbox may use a locally vendored Debian Rust registry, while GitHub Actions resolves dependencies from the standard crates.io registry. The repository intentionally does not commit a mirror-specific lockfile until an authentic crates.io lockfile can be generated.

## User experience

VDB follows progressive disclosure. Beginners can use templates and a future visual console; developers can use the CLI, HTTP API, or SDKs; operators can inspect policies, health metrics, audit events, and recovery reports. The first successful document should not require knowledge of CBOR, WAL, MVCC, indexes, partitions, or replication.

## Safety defaults

A new VDB instance is local-first. The design uses bounded document and query sizes, optimistic version checks, explicit destructive operations, checksummed/recoverable storage records, and read-only Steward behavior. VDB does not expose a generic shell tool, unrestricted AI database access, or automatic destructive repairs.

## Repository map

| Path | Purpose |
|---|---|
| `crates/vdb-core/` | Rust storage, document model, WAL replay, queries, health, and backups |
| `crates/vdb-cli/` | Native `vdb` command-line interface |
| `docs/` | Product, architecture, API, security, performance, roadmap, and demand documents |
| `diagrams/` | Mermaid architecture and data-flow diagrams |
| `benches/` | Planned benchmark fixtures and performance notes |

## Status and limitations

This is an MVP and not yet a production database. The current implementation uses an in-memory state map rebuilt from the versioned, checksummed WAL. It provides a single-process instance lock, JSON Lines export/import, lightweight single-field equality indexes, and safe WAL compaction. It does not yet provide encryption-at-rest, replication, or a network server. Those are deliberate follow-on milestones, not hidden guarantees.

## License

Apache-2.0 for the current project foundation.
