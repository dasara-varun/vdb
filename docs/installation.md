# VDB Installation Guide

VDB is currently distributed as a Rust command-line application. The supported installation model is a platform-native `vdb` binary built from a tagged source release or compiled locally. VDB does not require a server, daemon, container, cloud account, or AI service for its local MVP.

## Platform support

| Platform | Development build | Release artifact target | Current status |
|---|---|---|---|
| Linux x86_64 | `cargo build --release` | `x86_64-unknown-linux-gnu` | Primary tested target |
| Linux aarch64 | Cross-build or native build | `aarch64-unknown-linux-gnu` | Release target; validate on hardware before production use |
| macOS Intel | Native build or cross-build | `x86_64-apple-darwin` | Release target; filesystem behavior requires platform validation |
| macOS Apple Silicon | Native build or cross-build | `aarch64-apple-darwin` | Release target; filesystem behavior requires platform validation |
| Windows x86_64 | Native build | `x86_64-pc-windows-msvc` | Release target; lock and replacement semantics require additional validation |

The project’s current CI gate validates the Linux workspace. Cross-platform artifacts must not be described as production-supported until the cross-platform durability matrix, locking tests, permission behavior, backup tests, and recovery tests have passed on the target operating system.

## Build from source

Install a current stable Rust toolchain from [rustup.rs](https://rustup.rs/), clone the repository, and build the release binary:

```bash
git clone https://github.com/dasara-varun/vdb.git
cd vdb
cargo build --release
```

The binary is written to `target/release/vdb` on Unix-like systems and `target/release/vdb.exe` on Windows. Verify the build before placing it on `PATH`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
./target/release/vdb --help
```

On Linux or macOS, an individual developer may install the binary into a user-owned directory:

```bash
mkdir -p "$HOME/.local/bin"
install -m 0755 target/release/vdb "$HOME/.local/bin/vdb"
```

Do not use `sudo` to install VDB unless the operating system’s package policy requires it. A user-owned binary and user-owned database directory reduce the blast radius of mistakes.

## First database

Create a local database and a collection:

```bash
vdb init --path ./demo.vdb
vdb --path ./demo.vdb collections create users
vdb --path ./demo.vdb put users u_123 '{"name":"Asha","plan":"pro"}'
vdb --path ./demo.vdb get users u_123
vdb --path ./demo.vdb health
```

By default, VDB allows documents up to 1 MiB and limits the WAL to 512 MiB. Set a lower quota for a small embedded deployment or a larger quota for a controlled local workload:

```bash
vdb --path ./demo.vdb --max-wal-bytes 67108864 health
vdb --path ./demo.vdb --max-wal-bytes 1073741824 put users u_124 '{"name":"Ravi"}'
```

The quota is a write-time guard. When a write would exceed it, VDB returns a typed quota error before appending bytes. Run `compact` after verified backups when the WAL has accumulated obsolete history. A quota is not a substitute for filesystem quotas, monitoring, backups, or capacity planning.

## Release artifacts

The release workflow is intended to build checksummed archives for the supported target matrix when a maintainer creates a version tag. A release archive should contain the native `vdb` executable, `LICENSE`, `README.md`, and the relevant documentation. Download artifacts only from the project’s GitHub Releases page, verify the published SHA-256 checksum, and inspect the release notes before installation.

A future package manager formula or installer may wrap this process, but it must preserve these rules:

1. The binary must be verified against a trusted release checksum or signature.
2. The installer must not create a database, enable a network listener, install an AI model, or upload data.
3. Database files must be created in a user-selected directory, not an undisclosed global location.
4. Upgrades must preserve existing files and provide an explicit rollback path.
5. The installer must document its permissions and any files it creates.

## Windows notes

Build and run VDB from PowerShell after installing the MSVC Rust toolchain:

```powershell
cargo build --workspace --release
.\target\release\vdb.exe init --path .\demo.vdb
.\target\release\vdb.exe --path .\demo.vdb health
```

Windows support is a release target, not yet a completed production guarantee. In particular, the project still needs native advisory-lock coverage, atomic replacement tests, permission-equivalent protection, crash recovery, and path-race testing on Windows.

## macOS notes

Build on the host architecture when possible. On Apple Silicon, prefer an `aarch64-apple-darwin` binary; on Intel, prefer `x86_64-apple-darwin`. Do not place a database inside a cloud-synchronized folder until backup, rename, and durability behavior has been validated for that provider.

## Data and privacy

The current MVP stores plaintext CBOR records with integrity checksums. Checksums detect accidental corruption but do not provide confidentiality or authenticated encryption. Keep databases, WALs, lock files, backup manifests, exports, and agent memory outside shared directories; use restrictive filesystem permissions; and do not put secrets or personal data into test fixtures or AI prompts.

VDB is local-first and does not contact a remote service during normal CLI operation. A future HTTP API, telemetry system, model-backed Steward, or synchronization feature requires a separate threat model, opt-in configuration, authentication, tests, and documentation before it can be included in an installer.

## Troubleshooting

If VDB reports that the instance is locked, first confirm that no VDB process is using the path. The current MVP uses a lock-file protocol and can leave a stale lock after an abnormal exit; follow the recovery procedure in [`operations.md`](operations.md) rather than deleting files blindly. If a write is rejected for the WAL quota, make a verified backup, inspect health, and compact only after confirming that the database is not needed by another process.

For corrupt or unsupported storage, preserve the original file, copy it to an isolated location, run backup verification or recovery diagnostics, and do not attempt ad hoc byte edits. Use JSON Lines export/import as the portable migration path when the source can still be opened.

## References

1. [VDB README](../README.md)
2. [VDB API contract](api.md)
3. [VDB operations runbook](operations.md)
4. [VDB data format](data-format.md)
5. [VDB security model](security.md)
6. [VDB production-gap audit](production-gap-audit.md)
7. [Rust installation](https://www.rust-lang.org/tools/install)

_Last reviewed: 2026-08-24._
_Last reviewed by: Manus AI._
