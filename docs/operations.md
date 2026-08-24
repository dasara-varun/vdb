# VDB Operations Runbook

## Local deployment

Build the release binary with `cargo build --release`. Create a database in a user-owned directory and keep the file permissions restricted to the application user. The MVP has no network server, so the safest deployment is a local process with application-level access through the CLI or a future local API.

```bash
cargo run --release -- init --path ./data/app.vdb
cargo run --release -- --path ./data/app.vdb collections create users
```

VDB acquires a per-instance `.lock` file before creating or validating a new database header. This prevents ordinary concurrent opens of the same path, including the first-open race. New database and lock files use Unix mode `0600`, and existing database files are tightened when opened. The lock file is not yet an OS advisory lock and may remain after a crash; do not delete it automatically unless you have confirmed that no VDB process is using the database. Record the incident and follow the stale-lock recovery procedure agreed for the deployment.

## Health check

```bash
vdb --path ./data/app.vdb health
vdb --path ./data/app.vdb steward --collection users
```

The health output is an operational summary, not a durability guarantee. It reports the current in-memory state and WAL size. The Steward is read-only in the MVP.

## Indexes and compaction

Create an equality index only for a frequently used scalar field:

```bash
vdb --path ./data/app.vdb index-create users plan
vdb --path ./data/app.vdb index-list users
```

The current index implementation is single-field and equality-oriented. It is rebuilt from the WAL on reopen. Compact the WAL after repeated document replacements or imports:

```bash
vdb --path ./data/app.vdb compact
```

Compaction is serialized with writes. It writes a same-directory temporary WAL, synchronizes that file, records current documents as version-preserving `SnapshotPut` records, closes the active handle, atomically replaces the database path, and reopens the append handle. It preserves current documents and index definitions without replaying discarded update history. The temporary path includes the process ID and may need inspection or cleanup after an interrupted operation. Existing symbolic links are rejected for the database and compaction paths, but a complete race-free no-follow guarantee remains platform-specific. Because filesystem directory-entry durability and replacement behavior vary, compaction should still be followed by a backup and health check.

## Backup

Create a snapshot and manifest:

```bash
vdb --path ./data/app.vdb backup ./backups/app.vdb
vdb --path ./data/app.vdb backup-verify ./backups/app.vdb
```

A verified backup is one whose manifest checksum and byte count match and whose contents can be replayed by VDB. The backup command serializes with writes and compaction, synchronizes the source WAL before copying, and writes a separate manifest. Production deployments should copy verified backups to a separate storage failure domain, use collision-safe destinations, protect the manifest with the backup, and perform scheduled restore drills into a new path.

## Recovery

If VDB reports a checksum mismatch, stop writes to the affected file, preserve the original and logs, and restore the newest verified backup into a separate path. Do not delete or overwrite the original until the recovery result has been reviewed.

If the file ends with an incomplete record, the MVP truncates the incomplete tail during replay. This behavior is intended for process interruption at the end of a write. A complete record with a checksum mismatch fails closed and requires recovery. If compaction leaves a `.compact.<pid>.tmp` file, first verify that no VDB process is active, preserve it for incident analysis if needed, and remove it only after the database path and backup have been validated.

## Resource limits

The MVP limits documents to 1 MiB by default, configured document limits to 64 MiB, JSON Lines import records to 2 MiB, WAL records to 64 MiB, and queries to 1–1000 results. WAL replay is streamed, but the materialized working set and equality indexes are currently held in memory, so this MVP is not suitable for datasets larger than the available memory budget. Operators should monitor WAL growth, document count, payload bytes, disk usage, process memory, and restart replay time.

## Current limitations

The MVP does not yet provide encryption at rest, authenticated encryption, network serving, replication, authentication, production-grade key management, OS-level advisory locks, memory-bounded storage, a query planner, or a model-backed Steward. The local implementation does provide a single-process lock file, secondary equality indexes, bounded queries, checksummed/versioned records, WAL compaction, backup verification, and JSON Lines portability. Do not deploy it for critical data until encryption, lock semantics, server isolation, crash testing, dependency auditing, and the recovery program are implemented and reviewed.

## Incident record

For every incident, record the database version, operating system, filesystem, file checksum, WAL size, last verified backup, failing command, error text, affected collection, temporary files found, and recovery steps. Do not include secrets or raw sensitive documents in issue reports.
