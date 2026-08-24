# VDB Project Status

**Status:** Living snapshot
**Last reviewed:** 2026-08-24
**Repository state:** Verify with `git rev-parse HEAD` and `git status --short --branch` at each handoff
**Maintainer rule:** Update this file whenever implementation status, validation state, priorities, or known limitations change.

## Executive summary

VDB is a Rust-based, local-first document NoSQL database MVP. It uses a versioned `VDB1` file header, append-only CBOR WAL records, per-record SHA-256 checksums, bounded streaming replay, semantic WAL validation, private Unix file modes, common-path symlink rejection, optimistic document versions, bounded equality queries and imports, equality indexes, health and schema reports, verified backups, JSON Lines portability, safe version-preserving compaction, and a deterministic read-only Steward.

The project is **not production-ready**. Its current state is deliberately small and inspectable. Encryption at rest, authenticated encryption, OS-level advisory locking, stale-lock recovery, memory-bounded storage, a query planner, application API, authentication, replication, and a model-backed Steward remain future work.

## Capability status

| Area | Status | Evidence or source of truth |
|---|---|---|
| Rust workspace and CLI | Implemented | `crates/vdb-core`, `crates/vdb-cli`, `README.md` |
| `VDB1` header/version validation | Implemented for versions 1–2, with version-preserving compaction snapshots | `docs/data-format.md`, core tests |
| Append-only CBOR WAL | Implemented | `docs/data-format.md`, core implementation |
| Record checksum and replay recovery | Implemented baseline with bounded streaming and semantic checks | `docs/data-format.md`, corruption and adversarial tests |
| Document CRUD and optimistic versions | Implemented | `docs/api.md`, core tests |
| Bounded equality query | Implemented baseline for top-level and dotted nested equality fields | `docs/api.md`, query tests |
| Single-field equality index | Implemented baseline | `docs/performance.md`, index tests |
| Health and schema reports | Implemented | CLI and core tests |
| Verified backup and JSON Lines portability | Implemented baseline; backup verification requires an intact manifest and import is prevalidated/atomic | `docs/operations.md`, backup/export/import tests |
| WAL compaction | Implemented safer version-preserving baseline | `docs/data-format.md`, compaction regression test |
| Configurable WAL storage quota | Implemented bounded write-time guard; default 512 MiB, configurable up to 16 GiB; compaction replacement is also bounded | `docs/api.md`, core quota and compaction-quota regression tests |
| Cross-platform installation/release workflow | Implemented baseline artifact workflow and device guide; target-platform durability validation remains pending | `.github/workflows/release.yml`, `docs/installation.md` |
| Startup process lock ordering | Implemented baseline | `docs/operations.md`, lock regression test |
| Deterministic read-only Steward | Implemented baseline | `docs/steward.md`, CLI output |
| Encryption/key management | Planned, P0 | `docs/improvement-plan.md`, `docs/security.md` |
| Authentic committed dependency lockfile | Blocked by current mirror environment | `docs/audit-research.md`, `docs/testing.md` |
| Memory-bounded segments/checkpoints | Planned, P1 | `docs/improvement-plan.md` |
| Local application API | Planned, P1 | `docs/roadmap.md`, `docs/improvement-plan.md` |
| Model-backed Steward | Planned, P2 | `docs/steward.md`, `docs/security.md` |
| Replication | Deferred, P2 | `docs/roadmap.md`, `docs/production-gap-audit.md` |
| Documentation system | Implemented baseline | `AGENTS.md`, `CONTRIBUTING.md`, `docs/README.md`, `docs/documentation-maintenance.md` |
| Agent-tooling and bounded-loop guidance | Implemented baseline; optional and non-authoritative | `docs/agent-tooling.md`, `docs/loop-engineering.md`, `CLAUDE.md` |
| Security-conscious tool inventory | Implemented for the current development environment; refresh on tool changes | `docs/agent-tooling.md` |
| Filesystem and input hardening | Implemented baseline with strict backup paths and atomic bounded imports | `docs/security.md`, core adversarial tests |
| Semantic WAL replay validation | Implemented baseline | `docs/data-format.md`, version and corruption tests |

## Validation snapshot

The current local validation passed formatting, Clippy with warnings denied, **29 core tests and 4 CLI contract tests**, workspace tests, and the replay-hardening regression suite. The broader previous validation also passed shell syntax and documentation consistency checks, release compilation, and the CLI lifecycle smoke workflow. The current adversarial tests cover oversized and atomic imports, multiple-record JSON Lines import, oversized WAL length prefixes failing closed, duplicate snapshot rejection, deterministic truncated-tail recovery with valid-prefix preservation, invalid WAL version sequences, legacy format reopening, unsupported-format rejection, strict backup-manifest verification including symlink rejection, backup self-target rejection, backup/export overwrite protection, Unix private modes for database/lock/backup/manifest files, bounded document and WAL configuration, larger configured documents, nested indexed and unindexed equality queries, and no-partial-append quota failure. GitHub Actions run `32706132858` passed for commit `d8d97bf` (`test: document and cover truncated WAL recovery`). The audit milestone commit `b833a72` (`fix: harden import queries and backup verification`) passed hosted VDB CI run `32707712911`. Replay-hardening commit `a4ce7a7` (`fix: fail closed on malformed replay records`) passed hosted VDB CI run `32727612212`.

These results demonstrate correctness for the tested MVP paths; they do not prove power-loss durability on every filesystem, production capacity, encryption, multi-process safety beyond the documented lock-file behavior, or model safety under a future LLM adapter.

## Current priorities

The WAL quota, baseline release packaging, durability matrix, nested equality queries, strict backup verification, atomic bounded import, fail-closed oversized-length handling, and duplicate-snapshot rejection are implemented baselines. The next highest-value work is to generate and commit an authentic crates.io `Cargo.lock` from a clean environment, switch CI to locked builds, and add RustSec dependency auditing. In parallel, the project should execute the durability matrix with cross-platform crash/fuzz testing, design authenticated encryption and key management, replace the remaining in-memory materialized state with bounded segments/checkpoints, and close the platform-specific no-follow and advisory-lock gaps before adding remote serving or model autonomy.

## How to update this snapshot

When a feature changes state, update its row, add evidence, update the review date and commit, and revise the relevant roadmap or production-gap entry. When a test or CI result changes, update the validation paragraph with the exact command or run. When no implementation changes but a periodic review occurs, update the review date and record what was checked. The documentation and agent-tooling system is now implemented as a baseline: future work must follow [`AGENTS.md`](../AGENTS.md), [`CONTRIBUTING.md`](../CONTRIBUTING.md), [`docs/documentation-maintenance.md`](documentation-maintenance.md), [`docs/agent-tooling.md`](agent-tooling.md), and [`docs/loop-engineering.md`](loop-engineering.md).
