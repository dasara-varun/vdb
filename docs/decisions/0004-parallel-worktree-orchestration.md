# ADR-0004: Parallel Worktree Multi-Agent Orchestration with Zero-CI Local Gating

- **Status:** accepted
- **Date:** 2026-09-03
- **Authors:** Dasara Varun and Antigravity
- **Supersedes:** none
- **Superseded by:** none

## Context

Developing VDB across multiple tracks (storage, query engine, platform locking, fuzzing, APIs) sequentially on a single checkout creates bottlenecks:
1. Parallel agents editing files in a single checkout experience merge collisions, git lock contention, and dirty working trees.
2. Multiple concurrent `cargo build` or `cargo test` runs against a single `target/` directory stall on process file locks.
3. Builder agents grading their own work suffer from blind spots, resulting in untested edge cases.
4. Using external CI services (like GitHub Actions runners) incurs financial cost and latency for local-first development.

## Decision

VDB development will adopt an isolated parallel worktree orchestration model running entirely on local developer hardware:

1. **One Integrator, Many Worktrees**:
   - The Integrator operates on `main`.
   - Each builder agent operates in an isolated git worktree (`../vdb-worktrees/vdb-agent-<name>`) on an explicit feature branch (`agent/<track>/<name>`).
2. **Dedicated Target Directory Isolation**:
   - Every worktree configures its own `.cargo/config.toml` targeting `../../vdb-targets/target-<name>`, preventing build lock contention across concurrent jobs.
3. **Contract-First Interfaces**:
   - Dependent tracks (e.g., Query depending on Storage's segment layout) unblock parallelism by committing trait/type stubs first. Downstream agents build against frozen traits while the upstream builder implements the engine.
4. **Maker != Checker Discipline**:
   - Independent Tester agents write negative/adversarial tests against builder diffs without reading the builder's own test cases.
   - Verifier / Checklist agents produce an acceptance report mapped line-by-line to `docs/production-gap-audit.md` before merge consideration.
5. **Zero CI Cost Local Gating**:
   - CI / GitHub Actions compute is replaced by local gate scripts (`scripts/local-gate.ps1` and `scripts/local-gate.sh`).
   - The Integrator merges only when local gates, adversarial tests, and checklist reports are completely green.
   - Frequent commits are enforced, keeping `main` clean, tested, and releasable at all times.

## Alternatives Considered

| Alternative | Benefits | Costs or risks | Reason not chosen |
|---|---|---|---|
| Sequential single-branch development | Simple git structure | Extreme latency; tracks block each other | Defeats multi-agent speed advantage |
| Shared target directory | Saves disk space | File-lock collisions stall concurrent cargo builds | Broken builds under parallel agents |
| Builder self-testing only | Less coordination overhead | Blind spots, missing negative and tamper tests | Compromises database reliability |
| Cloud CI runners | Offloads local CPU | Incurs ongoing financial expense | Rejected due to zero-cost constraint |

## Consequences

- Disk usage increases moderately to store isolated target caches in `vdb-targets/` (managed via `scripts/teardown-worktree.ps1`).
- Merge order and dependency resolution are governed by explicit checklists and the Integrator.
- Fast, predictable, zero-cloud-cost local iteration with high confidence in correctness.

## References

1. [`docs/orchestration.md`](../orchestration.md)
2. [`docs/templates/acceptance-checklist.md`](../templates/acceptance-checklist.md)
3. [`scripts/local-gate.ps1`](../../scripts/local-gate.ps1)
4. [`docs/production-gap-audit.md`](../production-gap-audit.md)
