# VDB Improvement Plan

**Status:** Research-backed plan for the Rust MVP
**Author:** Manus AI
**Updated:** 2026-08-24

## Executive decision

VDB should remain a **small, local-first, document database with a read-only Steward**, rather than trying to become a general-purpose distributed database immediately. The current MVP has a sound direction: Rust provides strong compile-time ownership and concurrency guarantees, while the storage layer keeps writes explicit, bounded, checksummed, and replayable. The next gains should come from making durability claims precise, protecting data at rest, scaling beyond an in-memory working set, and exposing a narrow application interface before adding model autonomy.

> The governing product rule remains: **the Steward may explain, inspect, and propose; it must not silently mutate data, run arbitrary commands, or weaken a storage policy.**

## Current baseline

The repository currently contains a Rust workspace with `vdb-core` and a CLI. It implements a versioned `VDB1` file header, append-only CBOR WAL records, per-record SHA-256 integrity checks, replay recovery, optimistic document versions, bounded equality queries, equality indexes, schema and health reports, verified backups, JSON Lines export/import, single-process locking, configurable WAL quotas, WAL compaction, deterministic read-only Steward findings, tests, a release benchmark, cross-platform artifact packaging, and GitHub Actions checks.

The latest audit removed avoidable production panics from collection lookup, WAL handle access, manifest serialization, compaction, export, and WAL length decoding. Startup now acquires the process lock before creating or validating the database header, preventing two first-open callers from racing on a new path. Compaction now writes a same-directory temporary WAL, synchronizes it, atomically replaces the active path, and reopens the append handle using version-preserving `SnapshotPut` records. Replay now streams records instead of loading the complete WAL into memory, validates semantic references and version sequences, bounds JSON Lines import records and configured document limits, enforces configurable materialized document-count and aggregate-payload budgets during writes/import/replay, applies private Unix file modes, and rejects common symlink path hazards. These changes improve failure behavior but do not, by themselves, establish a universal power-loss guarantee across operating systems and filesystems or account for every index allocation and process working-set byte.

## Ranked roadmap

| Priority | Workstream | Why it matters | Recommended outcome | Exit evidence |
|---|---|---|---|---|
| P0 | Crash and replacement durability | `rename` behavior and directory durability differ by platform; a database must define what survives interruption | Maintain the storage durability matrix, Unix directory-sync strategy, bounded Unix process-crash smoke, explicit Windows replacement handling, and crash tests for compaction, backup, and replay | Cross-platform tests, documented guarantees, and fault-injection results |
| Done baseline | WAL quota and bounded growth | Unbounded append growth can exhaust disk or make recovery operationally unsafe | Enforce a configurable write-time WAL quota and reject oversized compaction replacements before path replacement | Quota regression tests, health/CLI exposure, and documented operational response |
| Done baseline | Input and replay hardening | WAL and import data are untrusted byte boundaries and can otherwise cause memory or state corruption | Stream WAL replay, fail closed on oversized length prefixes and duplicate snapshots, atomically prevalidate bounded imports, validate record references/version sequences and supported format versions, support nested equality paths consistently, reject common symlink paths, require intact backup manifests, use private Unix modes, and refuse unsafe output overwrites | 29 core tests plus 4 CLI contract tests, strict Clippy, and documented limitations |
| P0 | Encryption and key management | VDB files and manifests are currently plaintext; checksums detect accidental corruption but do not provide confidentiality or authenticity | Use a reviewed AEAD design with key rotation, secure key-provider boundaries, and secret-free diagnostics; never invent cryptography in the database layer | Threat-model update, key lifecycle tests, encrypted backup/restore drills |
| Done baseline / P0 follow-up | Reproducible dependency supply chain | Dependency drift and known vulnerable or unmaintained transitive crates must be detectable | Stable Cargo generated and committed an authentic lockfile; hosted CI compares lockfile freshness, uses `--locked`, runs RustSec auditing, and enforces cargo-deny license/source/advisory policy. The unmaintained `serde_cbor` finding is remediated with maintained `ciborium`; the only wildcard exception is the documented private workspace path dependency. SBOM generation remains open | Authentic lockfile, green locked builds, advisory scan, documented exceptions, and release SBOM |
| P1 | Memory-bounded storage | The MVP rebuilds the complete state and indexes in memory, so database size is bounded by process memory and replay time | Retain configurable document-count and aggregate-payload caps, then introduce byte-accounted immutable segments, checkpoints, bounded caches, and an explicit maximum database size | Large-dataset tests showing bounded RSS and bounded restart work |
| P1 | Query planner and index lifecycle | Current equality indexes are useful but limited to scalar equality and still require final filtering | Add query-planner explain output, index selectivity metrics, compound/range indexes only when justified, and safe rebuild/drop workflows; nested dotted equality is now supported consistently | Explain plans, benchmark comparisons, consistency tests under updates and compaction |
| P1 | Narrow local application API | A CLI is approachable for humans but not sufficient for applications; a network listener would enlarge the threat surface | Add a localhost-only API or local IPC boundary with request limits, structured errors, timeouts, and explicit bind controls; default to no remote exposure | API contract, negative security tests, bind/authentication tests |
| Done baseline / P1 follow-up | Recovery and backup isolation | Copying and restoring files is safe only when operators understand snapshot consistency and failure domains | Backup destination collision rules, atomic manifest creation, verified restore-to-new-path command, and operator runbook are implemented; scheduled restore drills and failure-domain validation remain | Restore/reopen/manifest regression and CLI smoke pass; automated cross-platform backup/restore matrix remains required |
| Done baseline / P0 cross-platform follow-up | OS-level process locking | A lock file alone can remain after an abnormal termination and is not an advisory lock held by the operating system | Unix now uses rustix’s safe nonblocking advisory lock and permits stale regular lock-file reuse after process exit; non-Unix retains the conservative create-new fallback until a cross-platform abstraction is validated | Unix concurrent-open and stale-lock tests pass; Windows/macOS/network-filesystem results and crash procedures remain required |
| P2 | Property, fuzz, and crash testing | WAL parsers and recovery code process untrusted bytes and are high-value fault boundaries | Fuzz record decoding, length/checksum handling, JSON import, and Steward input; inject interruption at each durable write boundary | Fuzz corpus, sanitizer runs, mutation tests, and reproducible crash cases |
| P2 | Steward adapter | Deterministic findings provide a safe baseline; a model can improve explanations but introduces prompt-injection and data-exfiltration risk | Add an optional local/private model adapter that receives minimized context, returns a typed proposal, and cannot directly access filesystem, shell, or mutation APIs | Red-team suite, schema validation, approval gate, and no-side-effect tests |
| P3 | Replication and remote operation | Replication adds ordering, conflict, identity, authentication, and operational complexity | Defer until the single-node format, recovery, API, encryption, and observability are stable | Written consistency model, failure tests, and operational ownership plan |

## Security priorities

VDB should treat the document payload, WAL, backup, manifest, CLI arguments, import stream, and future model output as separate trust boundaries. A SHA-256 checksum is an integrity check against accidental changes; it is not encryption and does not authenticate an attacker who can rewrite both payload and checksum. Encryption, key management, permissions, and authenticated metadata therefore remain first-class work rather than optional polish.

The future Steward should use a typed, versioned control message rather than arbitrary model-generated JSON. Its read context should be minimized to the fields required for a finding, and sensitive values should be redacted or summarized before model access. Any proposed action should pass schema validation, policy checks, explicit user approval, and post-action verification. The safe default is still read-only.

## Performance priorities

The fastest safe design is not the one with the most low-level mechanisms; it is the one that keeps the hot path measurable and the recovery path understandable. VDB should benchmark durable writes, reads, query latency, replay time, compaction time, memory usage, and backup throughput separately. The in-memory MVP benchmark is useful as a baseline, but it must not be presented as a production capacity claim.

The recommended storage evolution is append-only segments plus periodic checkpoints. Segments reduce the cost and risk of rewriting one large file, while checkpoints reduce restart replay. A planner can then choose between an index and a bounded scan using observed selectivity. Each optimization should ship with a workload, a regression threshold, a correctness test, and a complexity decision.

## Reproducibility and release policy

The repository now commits the stable-Cargo `Cargo.lock`, and hosted CI verifies that a fresh stable resolver produces no diff before running locked builds. The sandbox’s Debian registry can still report replacement-source checksum comparisons against the official crates.io checksums; local development should treat hosted locked CI as the authoritative resolver check. Dependency updates must be reviewed as lockfile diffs and must preserve the pinned RustSec audit job.

RustSec documents `cargo-audit` for scanning `Cargo.lock` for known vulnerable crates, `cargo-deny` for advisory, license, source, and duplicate-dependency policy, and `cargo-auditable` for embedding dependency metadata in binaries.[3] VDB now runs the pinned `rustsec/audit-check@v2.0.0` action and immutable cargo-deny action against the committed lockfile. The policy retains external wildcard denial, permits only the private workspace path dependency through the documented `allow-wildcard-paths` behavior, and has no blanket advisory ignore. Release SBOM generation remains open.

## What VDB should not do yet

VDB should not add a general-purpose embedded scripting engine, arbitrary shell access, autonomous destructive repair, remote-by-default networking, custom cryptography, or multi-node consensus merely to appear feature-complete. Each would expand the attack surface and operational burden before the core storage contract has enough evidence. The project’s differentiator is **simple, inspectable, constrained local storage**, not maximal feature count.

## References

[1]: https://doc.rust-lang.org/book/ch16-00-concurrency.html "The Rust Book: Fearless Concurrency"
[2]: https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html "The Cargo Book: Cargo.toml vs Cargo.lock"
[3]: https://rustsec.org/ "RustSec Advisory Database"
[4]: https://doc.rust-lang.org/std/fs/fn.rename.html "Rust std::fs::rename documentation"
[5]: https://www.sqlite.org/atomiccommit.html "SQLite: Atomic Commit In SQLite"
