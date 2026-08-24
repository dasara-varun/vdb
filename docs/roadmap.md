# VDB Roadmap

## Completed Rust MVP

The current `main` branch contains a fast Rust workspace with a local document store, nested JSON-like document payloads, CBOR WAL records, versioned file headers, per-record SHA-256 checksums, bounded streaming replay, semantic WAL validation, private Unix file modes, common-path symlink rejection, replay recovery, single-process instance locking, optimistic document versions, bounded equality queries and imports, single-field equality indexes, schema reports, health diagnostics, snapshot manifests, backup verification, JSON Lines export/import, WAL compaction with version-preserving snapshot records, a read-only Steward, CLI workflows, a release benchmark, automated tests, documentation consistency checks, and GitHub Actions checks.

## Phase 1: Production storage safety

The next production milestone is authenticated encryption at rest with a reviewed key-management design, stronger backup consistency guarantees, cross-platform filesystem validation, and extended fault-injection testing. The MVP now includes an explicit configurable WAL quota and a conservative cross-platform release-artifact workflow, but neither replaces the remaining durability and security work. VDB should not claim production readiness until the full set of controls is implemented and reviewed.

## Phase 2: Query and memory scalability

Extend equality indexes into a query planner, add selective compound or range indexes only when benchmark evidence justifies them, and replace the unbounded in-memory state map with immutable segments, checkpoints, and a memory-bounded storage/index strategy. Replay is now streamed, but the materialized working set remains memory-bound only by available process memory. Every change must preserve versioning, checksums, recovery, and bounded work.

## Phase 3: Local application API

Add a localhost-only HTTP API with strict request validation, request-size limits, authentication hooks, stable error codes, and API contract tests. Provide small Python and TypeScript clients that use the public API rather than duplicating storage logic.

## Phase 4: Steward recommendation mode

Add redacted context construction, optional local/private model adapters, evidence-linked recommendations, typed action-plan schemas, policy validation, audit records, human approval, and simulation. The model remains untrusted and no generic shell or arbitrary command tool is introduced.

## Phase 5: Controlled automation

Allow only allowlisted, reversible operations such as verified snapshots, statistics refresh, or non-critical index creation. Each action requires preconditions, resource budgets, approval policy, postcondition verification, and a rollback path.

## Phase 6: Server and replication

Evaluate authenticated multi-user server mode, read replicas, explicit consistency policies, export/import compatibility, and optional synchronization. Distributed writes and multi-region conflict resolution require a separate design, threat model, and failure-testing program.

## Release gates

Every release must pass `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, release compilation, CLI smoke tests, corruption/recovery tests, documentation consistency checks, dependency checks, and benchmark regression review. Cross-platform release artifacts must additionally pass the target-platform validation matrix before being called supported. Security-sensitive changes require a threat-model update and regression test.

## Commit policy

Commits should be small and coherent. Each commit must leave the default test command runnable. Feature branches may be used for risky work, but completed work must be merged into `main` and pushed regularly.
