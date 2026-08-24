# VDB Roadmap

## Phase 0: Foundation

Set up the package, test runner, formatting, linting, type checking, CI, versioned storage interface, and documentation. The repository must remain runnable after every commit.

## Phase 1: Local document MVP

Implement collections, nested documents, `_id`, versions, CRUD, bounded filters, pagination, TTL, structured errors, and a local CLI/API surface. Use a mature embedded storage foundation behind a repository interface.

## Phase 2: Durability and recovery

Add WAL or equivalent durability, checksums, encrypted snapshots, backup manifests, restore-to-sandbox verification, crash-injection tests, and storage format versioning.

## Phase 3: Health and Steward Observe mode

Add schema fingerprints, drift detection, query budgets, storage metrics, backup freshness, and read-only findings. Add redaction and an optional local/private model interface only after deterministic diagnostics work.

## Phase 4: Recommend mode

Add evidence-linked recommendations, typed action-plan schemas, policy validation, audit ledger, human approval flow, and simulated/canary execution. No destructive automation.

## Phase 5: Controlled automation

Allow only allowlisted reversible actions, such as verified snapshots, statistics refresh, or non-critical index creation. Every action requires preconditions, budgets, postcondition verification, and a rollback path.

## Later milestones

Evaluate server mode, authenticated multi-user access, read replicas, explicit consistency policies, export/import compatibility, and optional synchronization. Distributed writes and multi-region conflict resolution require a separate design and failure-testing program.

## Release gates

A release cannot be considered stable until it passes unit tests, property tests for document operations, API contract tests, crash recovery tests, restore verification, malformed-input tests, authorization tests, dependency checks, and Steward prompt-injection/action-validation tests.

## Commit policy

Commits should be small and coherent. Recommended sequence:

1. Documentation and repository foundation.
2. Package scaffold and test harness.
3. Storage adapter and document model.
4. CRUD and query limits.
5. CLI/API surface.
6. Health diagnostics.
7. Recovery and backup verification.
8. Steward findings and policy boundary.
9. Integration tests and release documentation.

Each commit should leave the default test command runnable. Feature branches may be used for risky work, but completed work must be merged into `main`.
