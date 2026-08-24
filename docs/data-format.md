# VDB Data Format Contract

## User-facing model

VDB presents collections of documents. A document is an object with a stable application identifier and a user-controlled payload. System metadata such as version and timestamps is maintained outside the payload so users cannot forge concurrency or audit fields.

## Internal encoding

The Rust MVP encodes WAL records with CBOR. Each WAL record uses a length prefix, a serialized typed record, and a SHA-256 checksum over the record payload. This gives compact binary storage while keeping replay validation explicit. The record envelope is:

```text
[u32 little-endian payload length][CBOR payload][32-byte SHA-256 payload checksum]
```

The file is append-only during normal writes. On startup, VDB replays complete valid records. An incomplete trailing record is truncated; a complete record with a checksum mismatch fails closed and reports that storage recovery is required.

## Compatibility boundary

JSON remains the easiest external interchange format. A future HTTP API and SDKs may accept JSON and translate it into the internal typed representation. CBOR exports may be offered for compact migration, while JSON export remains the human-readable escape hatch.

Trusted control messages, policies, telemetry events, and future replication messages should use an explicit schema rather than arbitrary model-generated JSON. Protocol Buffers or a similarly versioned typed format are candidates for that control plane.

## Supported values

The MVP accepts JSON-like objects containing nulls, booleans, numbers, strings, arrays, and nested objects. Binary values, timestamps with native types, decimals, and custom extension tags are deferred until the compatibility and SDK model are defined. Unsupported values are rejected before any WAL write occurs.

## Versioning and migration

Every storage file must carry a format version before the first stable release. The VDB CLI should provide export/import commands and a migration tool. A newer binary must not silently reinterpret an older file. If migration is needed, VDB should create a new destination file, verify it, and preserve the original until the user confirms replacement.

## Security and privacy

The format must never store secrets in document payloads by assumption. Encryption at rest is a planned milestone. Until it is implemented and tested, VDB should clearly identify the file as unencrypted storage and recommend filesystem or volume encryption for sensitive data.
