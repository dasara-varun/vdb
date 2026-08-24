# VDB Operations Runbook

## Local deployment

Build the release binary with `cargo build --release`. Create a database in a user-owned directory and keep the file permissions restricted to the application user. The MVP has no network server, so the safest deployment is a local process with application-level access through the CLI or a future local API.

```bash
cargo run --release -- init --path ./data/app.vdb
cargo run --release -- --path ./data/app.vdb collections create users
```

## Health check

```bash
vdb --path ./data/app.vdb health
vdb --path ./data/app.vdb steward --collection users
```

The health output is an operational summary, not a durability guarantee. It reports the current in-memory state and WAL size. The Steward is read-only in the MVP.

## Backup

Create a snapshot and manifest:

```bash
vdb --path ./data/app.vdb backup ./backups/app.vdb
vdb --path ./data/app.vdb backup-verify ./backups/app.vdb
```

A verified backup is one whose manifest checksum and byte count match and whose contents can be replayed by VDB. Production deployments should copy verified backups to a separate storage failure domain and perform scheduled restore drills.

## Recovery

If VDB reports a checksum mismatch, stop writes to the affected file, preserve the original file and logs, and restore the newest verified backup into a separate path. Do not delete or overwrite the original until the recovery result has been reviewed.

If the file ends with an incomplete record, the MVP truncates the incomplete tail during replay. This behavior is intended for process interruption at the end of a write. A complete record with a checksum mismatch fails closed and requires recovery.

## Resource limits

The MVP limits documents to 1 MiB by default and queries to 1–1000 results. These are conservative defaults. Operators should monitor WAL growth, document count, payload bytes, disk usage, process memory, and restart replay time.

## Current limitations

The MVP does not yet provide encryption at rest, multi-process locking, secondary indexes, compaction, a network server, replication, authentication, or production-grade key management. Do not deploy it for critical data until those controls and the recovery test program are implemented and reviewed.

## Incident record

For every incident, record the database version, operating system, file checksum, WAL size, last verified backup, failing command, error text, affected collection, and recovery steps. Do not include secrets or raw sensitive documents in issue reports.
