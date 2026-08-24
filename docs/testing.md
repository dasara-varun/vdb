# VDB Testing and Reliability Plan

## Testing principle

A database is trustworthy only when its behavior is tested under normal requests, malformed input, concurrency, process interruption, partial writes, disk errors, and recovery. VDB should keep the test suite fast enough for every commit and reserve longer fault-injection, cross-platform, fuzz, and benchmark jobs for CI or scheduled runs.

## Test layers

| Layer | Scope | Current status |
|---|---|---|
| Unit tests | Document validation, versions, filters, schema types, WAL replay, backup verification | Implemented in `vdb-core` |
| Lock and compaction regressions | First-open lock ordering, atomic WAL rotation, post-compaction writes, index preservation | Implemented in `vdb-core` |
| CLI contract tests | Command parsing, JSON output, error codes, path handling | Next milestone |
| Property tests | Random document and operation sequences preserve invariants | Planned |
| Crash tests | Interrupt writes and compaction; no acknowledged record is silently lost | Planned |
| Corruption tests | Alter payload, length, and checksum bytes; recovery fails closed | Partially covered by checksum logic |
| Security tests | Reserved fields, size limits, path handling, unsafe code, privilege boundaries | Baseline implemented; expand with server mode |
| Performance tests | Release-build throughput, p95/p99, replay, memory, WAL growth | Initial benchmark example implemented |
| Compatibility tests | Versioned file formats, JSON Lines, and platform-specific replacement behavior | Partially implemented; expand cross-platform |
| Supply-chain tests | Locked dependency resolution, RustSec advisories, license/source policy | Pending authentic crates.io lockfile |

## Invariants

The following invariants should remain true: a document is either absent or has a complete valid payload; an expected-version update never overwrites a newer version; an acknowledged write has a durable WAL record after the configured sync point; a malformed or over-limit request has no side effect; a WAL checksum mismatch causes a recovery error rather than silent acceptance; backups include a verifiable manifest; compaction preserves the logical state; and Steward output cannot execute an untyped operation.

## Failure injection

The recovery suite should copy a valid VDB file, truncate it at every byte boundary in selected WAL records, flip bits in lengths and payloads, interrupt writes after each write step, interrupt compaction before and after temporary-file synchronization and replacement, and reopen the database. A trailing incomplete record may be safely truncated only when the implementation can prove that no complete record follows it. A checksum mismatch in a complete record must fail closed and require recovery.

Compaction tests must run on every supported operating system because Rust documents that `std::fs::rename` has platform-specific behavior. The same-directory temporary-file rule should be asserted, and tests should distinguish logical correctness from power-loss durability. A future durability test must include directory-entry synchronization where the target platform supports it and document any unavoidable platform limitation.

## Parser and adversarial testing

WAL length decoding, CBOR decoding, checksum validation, JSON Lines import, document-size limits, query limits, and Steward inputs are untrusted-byte boundaries. Add property tests and fuzz targets for these paths. Fuzz cases must be bounded in CPU, memory, and input size; each failure should be reduced to a deterministic regression fixture. Run sanitizers and thread/concurrency checks in a separate supported environment rather than making the ordinary developer loop unnecessarily slow.

## Supply-chain testing

Once an authentic crates.io `Cargo.lock` is available, CI should use `--locked` and run RustSec `cargo-audit`. Add `cargo-deny` when license, source, duplicate-version, and advisory policies have been agreed. Dependabot or an equivalent scheduled dependency update process should produce reviewable pull requests. The current sandbox-generated lockfile is intentionally not committed because its mirror-specific placeholder checksums are not valid for GitHub CI.

## Release gate

Every release must pass `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, a release build, the CLI smoke workflow, and the release benchmark smoke test. Security-sensitive features require a regression test and a documented threat-model update. A production release additionally requires a reviewed encryption design, backup/restore drill, cross-platform lock and compaction results, and a dependency advisory report.
