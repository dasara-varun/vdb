# VDB Durability Matrix

**Review date:** 2026-08-24
**Status:** MVP evidence and explicit gaps

This matrix is a release-gate document, not a claim of universal crash safety. VDB acknowledges a write only after it has appended the complete WAL record and `sync_data` has returned successfully. Rust documents that [`File::sync_data`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_data) attempts to synchronize file content but may not synchronize metadata, while [`File::sync_all`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all) also attempts to synchronize metadata. Rust also documents platform-specific behavior for [`std::fs::rename`](https://doc.rust-lang.org/std/fs/fn.rename.html). These APIs are necessary building blocks, not proof that every storage device honors power-loss durability.

SQLite’s [atomic commit documentation](https://www.sqlite.org/atomiccommit.html) is a useful comparison: it explicitly states hardware, filesystem, flushing, sector-write, deletion, and locking assumptions, and describes a larger journal-and-recovery protocol than VDB currently implements. VDB must not inherit SQLite’s guarantees merely because both use files and a log.

## Current matrix

| Operation or failure point | Current intended behavior | Evidence in this repository | Durability boundary and unresolved work |
|---|---|---|---|
| Normal acknowledged append | Append a complete length-prefixed CBOR record and checksum, synchronize the active WAL, then publish the in-memory state | Core write path; WAL quota no-partial-append test; 22 core tests | Depends on the operating system and storage honoring the successful sync. No power-loss certification exists. |
| Process interruption during the final WAL record | On reopen, discard and truncate an incomplete trailing record; retain the valid prefix | Deterministic trailing-tail recovery test; replay implementation | A complete record with a checksum mismatch fails closed. Crash tests that interrupt each write step are still required. |
| Corruption of a complete WAL record | Refuse recovery rather than silently applying a record whose checksum does not match | Checksum mismatch regression test | SHA-256 is not an attacker-authentication mechanism and does not provide confidentiality. |
| Compaction before temporary-file replacement | Write a same-directory temporary WAL; if quota is insufficient or replacement fails before the path changes, retain the original database | Compaction quota-preservation regression; compaction error paths | Crash interruption at every temporary-file sync, close, rename, and reopen boundary is not yet exercised. |
| Successful compaction replacement | Replace the active path with a version-preserving snapshot and reopen the append handle | Compaction data/index/reopen regression | Rust documents that rename behavior varies by platform. Directory-entry synchronization and post-power-loss outcomes are not certified. |
| Backup creation | Synchronize the source WAL, copy it to a collision-safe destination, synchronize the destination, and write a verifiable manifest | Backup verification and destination-safety tests | Cross-device backup and restore drills, destination filesystem failures, and power-loss timing are not covered. |
| Startup process exclusion | Create a per-instance lock file before creating or validating the database header; reject ordinary concurrent opens | First-open race and concurrent-open regression tests | The lock file is not an OS advisory lock, can remain after a crash, and has no race-free stale-lock proof across platforms. |
| Unix file permissions | New database, lock, backup, export, and manifest files use private modes where supported | Unix permission regression tests | Windows and macOS permission behavior require platform-specific validation. |

## Required next evidence

Before a production claim, the project needs a cross-platform test job or dedicated harness that records filesystem, operating system, filesystem mount, Rust version, test case, interruption point, reopened result, and artifact hashes. The harness should exercise bounded writes, trailing truncations, checksum mutations, compaction interruption before and after temporary-file synchronization and replacement, backup interruption, and stale-lock recovery. It must distinguish logical consistency after a process crash from power-loss durability, which cannot be inferred from an ordinary process-kill test.

The release gate remains blocked until the matrix has platform results, a reviewed directory-sync strategy, explicit stale-lock/advisory-lock semantics, and documented backup/restore drills. The current MVP remains suitable only for non-critical local data where operators accept these limits.

## Research references

1. Rust standard library, [`File::sync_data`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_data) and [`File::sync_all`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all), accessed 2026-08-24.
2. Rust standard library, [`std::fs::rename`](https://doc.rust-lang.org/std/fs/fn.rename.html), including platform-specific behavior, accessed 2026-08-24.
3. SQLite documentation, [Atomic Commit In SQLite](https://www.sqlite.org/atomiccommit.html), including hardware assumptions, flush ordering, rollback, and failure modes, accessed 2026-08-24.
