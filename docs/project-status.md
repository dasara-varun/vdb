# VDB Project Status

**Status:** Living snapshot
**Last reviewed:** 2026-08-24
**Repository state:** Verify with `git rev-parse HEAD` and `git status --short --branch` at each handoff
**Maintainer rule:** Update this file whenever implementation status, validation state, priorities, or known limitations change.

## Executive summary

VDB is a Rust-based, local-first document NoSQL database MVP. It uses a versioned `VDB1` file header, append-only CBOR WAL records, per-record SHA-256 checksums, replay recovery, optimistic document versions, bounded equality queries, equality indexes, health and schema reports, verified backups, JSON Lines portability, safe baseline compaction, and a deterministic read-only Steward.

The project is **not production-ready**. Its current state is deliberately small and inspectable. Encryption at rest, authenticated encryption, OS-level advisory locking, stale-lock recovery, memory-bounded storage, a query planner, application API, authentication, replication, and a model-backed Steward remain future work.

## Capability status

| Area | Status | Evidence or source of truth |
|---|---|---|
| Rust workspace and CLI | Implemented | `crates/vdb-core`, `crates/vdb-cli`, `README.md` |
| `VDB1` header/version validation | Implemented | `docs/data-format.md`, core tests |
| Append-only CBOR WAL | Implemented | `docs/data-format.md`, core implementation |
| Record checksum and replay recovery | Implemented baseline | `docs/data-format.md`, corruption test |
| Document CRUD and optimistic versions | Implemented | `docs/api.md`, core tests |
| Bounded equality query | Implemented | `docs/api.md`, query tests |
| Single-field equality index | Implemented baseline | `docs/performance.md`, index tests |
| Health and schema reports | Implemented | CLI and core tests |
| Verified backup and JSON Lines portability | Implemented baseline | `docs/operations.md`, backup/export tests |
| WAL compaction | Implemented safer baseline | `docs/data-format.md`, compaction regression test |
| Startup process lock ordering | Implemented baseline | `docs/operations.md`, lock regression test |
| Deterministic read-only Steward | Implemented baseline | `docs/steward.md`, CLI output |
| Encryption/key management | Planned, P0 | `docs/improvement-plan.md`, `docs/security.md` |
| Authentic committed dependency lockfile | Blocked by current mirror environment | `docs/audit-research.md`, `docs/testing.md` |
| Memory-bounded segments/checkpoints | Planned, P1 | `docs/improvement-plan.md` |
| Local application API | Planned, P1 | `docs/roadmap.md`, `docs/improvement-plan.md` |
| Model-backed Steward | Planned, P2 | `docs/steward.md`, `docs/security.md` |
| Replication | Deferred, P2 | `docs/roadmap.md`, `docs/production-gap-audit.md` |
| Documentation system | Implemented baseline | `AGENTS.md`, `CONTRIBUTING.md`, `docs/README.md`, `docs/documentation-maintenance.md` |

## Validation snapshot

The latest audited state passed the following local checks: formatting, Clippy with warnings denied, all workspace tests, release build, and an end-to-end CLI smoke workflow covering initialization, collection creation, CRUD, query, index creation/listing, backup verification, export, compaction, health, Steward output, and reopen. GitHub Actions run `32699151944` passed for commit `e14447a`.

These results demonstrate correctness for the tested MVP paths; they do not prove power-loss durability on every filesystem, production capacity, encryption, multi-process safety beyond the documented lock-file behavior, or model safety under a future LLM adapter.

## Current priorities

The next highest-value work is to generate and commit an authentic crates.io `Cargo.lock` from a clean environment, switch CI to locked builds, and add RustSec dependency auditing. In parallel, the project should define a cross-platform durability matrix, add crash/fuzz testing, design encryption and key management, and prototype memory-bounded segments/checkpoints before adding remote serving or model autonomy.

## How to update this snapshot

When a feature changes state, update its row, add evidence, update the review date and commit, and revise the relevant roadmap or production-gap entry. When a test or CI result changes, update the validation paragraph with the exact command or run. When no implementation changes but a periodic review occurs, update the review date and record what was checked. The documentation system itself is now implemented as a baseline: future work must follow [`AGENTS.md`](../AGENTS.md), [`CONTRIBUTING.md`](../CONTRIBUTING.md), and [`docs/documentation-maintenance.md`](documentation-maintenance.md).
