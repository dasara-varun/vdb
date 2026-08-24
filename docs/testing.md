# VDB Testing and Reliability Plan

## Testing principle

A database is trustworthy only when its behavior is tested under normal requests, malformed input, concurrency, process interruption, partial writes, disk errors, and recovery. VDB should keep the test suite fast enough for every commit and reserve longer fault-injection and benchmark jobs for CI or scheduled runs.

## Test layers

| Layer | Scope | Current status |
|---|---|---|
| Unit tests | Document validation, versions, filters, schema types, WAL replay, backup verification | Implemented in `vdb-core` |
| CLI contract tests | Command parsing, JSON output, error codes, path handling | Next milestone |
| Property tests | Random document and operation sequences preserve invariants | Planned |
| Crash tests | Interrupt writes and replay the WAL; no acknowledged record is silently lost | Planned |
| Corruption tests | Alter payload, length, and checksum bytes; recovery fails closed | Partially covered by checksum logic |
| Security tests | Reserved fields, size limits, path handling, unsafe code, privilege boundaries | Baseline implemented; expand with server mode |
| Performance tests | Release-build throughput, p95/p99, replay, memory, WAL growth | Initial benchmark example implemented |
| Compatibility tests | Versioned file formats and export/import behavior | Planned |

## Invariants

The following invariants should remain true: a document is either absent or has a complete valid payload; an expected-version update never overwrites a newer version; an acknowledged write has a durable WAL record after the configured sync point; a malformed or over-limit request has no side effect; a WAL checksum mismatch causes a recovery error rather than silent acceptance; backups include a verifiable manifest; and Steward output cannot execute an untyped operation.

## Failure injection

The recovery suite should copy a valid VDB file, truncate it at every byte boundary in selected WAL records, flip bits in lengths and payloads, interrupt writes after each write step, and reopen the database. A trailing incomplete record may be safely truncated only when the implementation can prove that no complete record follows it. A checksum mismatch in a complete record must fail closed and require recovery.

## Release gate

Every release must pass `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and the release benchmark smoke test. Security-sensitive features require a regression test and a documented threat model update.
