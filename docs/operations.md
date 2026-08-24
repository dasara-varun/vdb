# VDB Operations Runbook

## Local deployment

Build the release binary with `cargo build --release`. Create a database in a user-owned directory and keep the file permissions restricted to the application user. The safest deployment remains a local process with application-level access through the CLI. An opt-in inspection GUI is available, but it is deliberately loopback-only and read-only rather than a general network server.

```bash
cargo run --release -- init --path ./data/app.vdb
cargo run --release -- --path ./data/app.vdb collections create users
```

VDB acquires a per-instance `.lock` file before creating or validating a new database header. On Unix, the file is also held with a nonblocking OS advisory exclusive lock, so a regular stale lock file can be safely reused after the previous process exits and concurrent opens are rejected without deleting a live lock. On non-Unix targets, the conservative create-new lock-file fallback remains in effect and may require manual stale-lock recovery. New database and lock files use Unix mode `0600`, and existing database files are tightened when opened. Network-filesystem and cross-platform lock semantics are not yet certified; record incidents and do not manually delete a lock file while a VDB process may be using the database.

## Health check

```bash
vdb --path ./data/app.vdb health
vdb --path ./data/app.vdb steward --collection users
```

The health output is an operational summary, not a durability guarantee. It reports the current in-memory state and WAL size. The Steward is read-only in the MVP.

## Local read-only GUI

Start the GUI explicitly when local browser inspection is useful:

```bash
vdb --path ./data/app.vdb gui
vdb --path ./data/app.vdb gui --port 4317
```

The command prints a URL such as `http://127.0.0.1:4317`; copy it into a browser yourself. Port `0` (the default) asks the operating system for an ephemeral port. The GUI shows health, collections, and a bounded document sample. It accepts only `GET`, serves embedded local HTML/CSS, and uses the same core read and query limits as the CLI. It does not mutate data or run backup, restore, compaction, import, export, or Steward actions.

Loopback is not authentication. Do not bind or expose the GUI through a LAN address, reverse proxy, port forwarding, container bridge, public hostname, or shared workstation. Stop it with Ctrl-C before making recovery or maintenance changes. Use the CLI for all writes and operational actions. The GUI is a developer-experience baseline and does not change the production-readiness gates.

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

Compaction is serialized with writes. It writes a same-directory temporary WAL, synchronizes that file, records current documents as version-preserving `SnapshotPut` records, and measures the result against `max_wal_bytes` before replacement. If the compacted state would still exceed the quota, the command returns a quota error, removes the temporary output, and leaves the original database and active handle unchanged; compaction may therefore be blocked even when it would remove obsolete history. Otherwise, it closes the active handle, atomically replaces the database path, and reopens the append handle. It preserves current documents and index definitions without replaying discarded update history. The temporary path includes the process ID and may need inspection or cleanup after an interrupted operation. Existing symbolic links are rejected for the database and compaction paths, but a complete race-free no-follow guarantee remains platform-specific. Because filesystem directory-entry durability and replacement behavior vary, compaction should still be followed by a backup and health check.

## Backup

Create a snapshot and manifest:

```bash
vdb --path ./data/app.vdb backup ./backups/app.vdb
vdb --path ./data/app.vdb backup-verify ./backups/app.vdb
vdb --path ./data/app.vdb restore ./backups/app.vdb ./restored/app.vdb
vdb --path ./data/app.vdb backup-verify ./restored/app.vdb
```

A verified backup is one whose required manifest exists, whose checksum and byte count match, and whose contents can be replayed by VDB. Never separate a backup from its manifest or treat a file-only copy as verified. The backup command serializes with writes and compaction, synchronizes the source WAL before copying, and writes a separate manifest. The restore command verifies the source backup and manifest, refuses an existing destination, creates parent directories as needed, reopens the restored database, and writes a new destination manifest. Production deployments should copy verified backups to a separate storage failure domain, use collision-safe destinations, protect manifests with their backups, and perform scheduled restore drills into a new path followed by `backup-verify`.

## Recovery

If VDB reports a checksum mismatch, stop writes to the affected file, preserve the original and logs, and restore the newest verified backup into a separate path with `restore`. Do not delete or overwrite the original until the recovery result has been reviewed. Restore never overwrites an existing destination, so choose a new path for each recovery attempt.

If the file ends with an incomplete record, the MVP truncates the incomplete tail during replay and repairs the file to the last complete record; deterministic coverage is documented in [`durability-matrix.md`](durability-matrix.md). This behavior is intended for process interruption at the end of a write. A complete record with a checksum mismatch fails closed and requires recovery. If compaction leaves a `.compact.<pid>.tmp` file, first verify that no VDB process is active, preserve it for incident analysis if needed, and remove it only after the database path and backup have been validated.

## Resource limits

The MVP limits documents to 1 MiB by default, configured document limits to 64 MiB, JSON Lines import records to 2 MiB, JSON Lines import batches to 64 MiB, WAL records to 64 MiB, and queries to 1–1000 results. The default configurable document-count budget is 100,000 documents and the aggregate payload budget is 256 MiB; their supported ceilings are 10 million documents and 16 GiB respectively. The default configurable WAL quota is 512 MiB, with a supported configuration ceiling of 16 GiB. Health output reports current and configured document-count, payload-byte, and WAL budgets; operators should treat a high ratio as a signal to verify a backup and schedule compaction. A quota rejection occurs before the new record is appended, so the failed write does not change logical state or WAL size.

For CLI usage, pass the quotas on every command that opens the database, for example `vdb --path ./data/app.vdb --max-wal-bytes 1073741824 --max-documents 100000 health`. Import first validates the complete bounded JSON Lines batch, then appends it as one synchronized WAL batch; malformed input and quota rejection do not partially import earlier lines. Opening a database with a lower document quota does not delete existing data, but replay refuses to open a database whose document count exceeds the configured cap. Compaction can reduce obsolete WAL history even when ordinary writes are quota-blocked, but it is refused when the compacted representation itself exceeds the configured quota; this failure does not replace or invalidate the original database. WAL replay is streamed and the document count is explicitly bounded, but document bytes, collection maps, and equality indexes are still held in memory without a full byte-level working-set budget. Operators should monitor WAL growth, document count, payload bytes, disk usage, process memory, and restart replay time.

## Installation and continuation

Use [`installation.md`](installation.md) for native Linux, macOS, and Windows build/install guidance. Use [`agent-tooling.md`](agent-tooling.md) for optional Claude Code extensions and [`loop-engineering.md`](loop-engineering.md) before configuring any recurring or cross-session engineering workflow.

## Current limitations

The MVP does not yet provide encryption at rest, authenticated encryption, remote serving, replication, authentication, production-grade key management, byte-level memory-bounded storage, a query planner, or a model-backed Steward. The local GUI is not remote serving: it provides only an opt-in loopback read-only inspection surface and has no authentication. The local implementation does provide a single-process lock file, secondary equality indexes, bounded queries, checksummed/versioned records, WAL compaction, backup verification, and JSON Lines portability. Do not deploy it for critical data until encryption, lock semantics, API security, crash testing, dependency auditing, and the recovery program are implemented and reviewed.

## Incident record

For every incident, record the database version, operating system, filesystem, file checksum, WAL size, last verified backup, failing command, error text, affected collection, temporary files found, and recovery steps. Do not include secrets or raw sensitive documents in issue reports.
