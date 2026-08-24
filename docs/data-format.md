# VDB Data Format Contract

## User-facing model

VDB presents collections of documents. A document is an object with a stable application identifier and a user-controlled payload. System metadata such as version and timestamps is maintained outside the payload so users cannot forge concurrency or audit fields.

## Internal file encoding

The Rust MVP stores one database instance as an append-only WAL file. Every file begins with the six-byte header:

```text
[4-byte ASCII magic: VDB1][u16 little-endian format version: 1]
```

Each subsequent WAL record uses a length prefix, a serialized typed record, and a SHA-256 checksum over the record payload:

```text
[u32 little-endian payload length][CBOR payload][32-byte SHA-256 payload checksum]
```

The length is bounded before allocation or decoding. CBOR is compact and typed for internal storage; JSON remains the external interchange model. Acknowledged writes append a complete record and synchronize the WAL before updating the in-memory state.

## Replay and recovery

On startup, VDB validates the `VDB1` header and replays complete valid records in order. If the file ends with an incomplete trailing record, the MVP truncates only the incomplete tail and replays the valid prefix. A complete record with a checksum mismatch fails closed and reports that storage recovery is required. A newer or unknown format version is rejected rather than silently reinterpreted.

SHA-256 here is an integrity check against accidental corruption or incomplete writes. It does not provide confidentiality and does not authenticate an attacker who can rewrite both a payload and its checksum. Encryption at rest and authenticated encryption remain planned work.

## Compaction boundary

Normal writes append to the active WAL. Compaction is serialized with writes and snapshots the current in-memory state into a new same-directory temporary file. VDB writes the header and reconstructed records, synchronizes the temporary file, closes the active WAL handle, replaces the database path with the temporary file, and reopens the active WAL handle.

This prevents the previous unsafe in-place truncate-and-rewrite pattern and leaves the original path either unchanged when replacement fails or containing the compacted file after successful replacement. The exact durability of the directory-entry replacement is filesystem- and platform-dependent; VDB must not claim power-loss-proof compaction until directory synchronization and cross-platform crash tests are complete. Temporary files may require manual cleanup after an interrupted operation.

## Compatibility boundary

JSON remains the easiest external interchange format. The CLI’s JSON Lines export/import is the human-readable migration and recovery escape hatch. A future HTTP API and SDKs may accept JSON and translate it into the internal typed representation. CBOR exports may be offered for compact migration.

Trusted control messages, policies, telemetry events, and future replication messages should use an explicit schema rather than arbitrary model-generated JSON. Protocol Buffers or a similarly versioned typed format are candidates for that control plane.

## Supported values

The MVP accepts JSON-like objects containing nulls, booleans, numbers, strings, arrays, and nested objects. Binary values, timestamps with native types, decimals, and custom extension tags are deferred until the compatibility and SDK model are defined. Unsupported values are rejected before any WAL write occurs.

## Versioning and migration

Every storage file carries a format version. The VDB CLI provides export/import commands as the current migration path. A newer binary must not silently reinterpret an older file. If migration is needed, VDB should create a new destination file, verify it, and preserve the original until the user confirms replacement.

## Security and privacy

The format must never store secrets in document payloads by assumption. Until encryption at rest is implemented and tested, VDB should clearly identify the file as unencrypted storage and recommend filesystem or volume encryption for sensitive data. Backup manifests contain file metadata and a digest; they are not encrypted credentials and should be protected with the backup itself.
