# Audit Research Notes

## Rust concurrency

The official Rust Book states that ownership and type checking can turn many concurrency errors into compile-time errors, while also noting that Rust provides multiple concurrency models for different performance and system requirements.

Source: https://doc.rust-lang.org/book/ch16-00-concurrency.html

## Cargo reproducibility

The official Cargo Book states that `Cargo.lock` contains exact dependency information, is maintained by Cargo, and should generally be checked into version control for reproducible application builds. VDB previously removed a lockfile because the sandbox-generated file contained mirror-specific placeholder checksums that failed on GitHub. The correct long-term fix is to generate an authentic crates.io lockfile in a clean registry environment, then restore lockfile-based CI rather than leaving dependency resolution floating indefinitely.

Source: https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html

## Dependency security and file replacement

RustSec describes itself as a vulnerability database for the Rust ecosystem and documents `cargo-audit` as a tool that audits `Cargo.lock` for known vulnerable crates. It also identifies `cargo-deny` for advisory, license, source, and duplicate-dependency checks, and `cargo-auditable` for embedding dependency metadata in binaries. Source: https://rustsec.org/

The Rust standard library documents `std::fs::rename` as replacing an existing destination, but notes that it cannot cross mount points and that exact behavior differs between Unix and Windows. The implementation must therefore keep the temporary file in the database directory, document platform behavior, and add platform-specific validation before claiming universal crash-safe replacement. Source: https://doc.rust-lang.org/std/fs/fn.rename.html

The current VDB compaction path writes and `sync_data`s a same-directory temporary file, drops the active WAL handle, renames the temporary file over the database, and reopens the WAL. This is materially safer than in-place truncation, but directory metadata durability and Windows-specific replacement behavior remain future hardening work.

## Embedded-database durability comparison

SQLite’s atomic-commit documentation emphasizes that crash safety depends on a defined commit protocol, filesystem locking, and flush behavior, and it includes dedicated sections for failure modes such as incomplete flushes, partial deletions, garbage writes, and rename/delete handling. VDB should use the same discipline: specify assumptions, test interruption points, distinguish logical recovery from power-loss durability, and avoid promising more than the target filesystem can provide. Source: https://www.sqlite.org/atomiccommit.html
