# VDB Parallel Worktree Orchestration Guide

This guide establishes the mechanics for developing VDB concurrently using dedicated builder, tester, and verifier agents on a single machine without merge conflicts, cargo lock contention, or cloud CI costs.

---

## 1. Core Principles

1. **One Integrator on `main`**:
   The Integrator agent never writes product features directly. Its sole responsibility is orchestrating worktree creation, reviewing checklists, running the local gate, executing fast-forward merges, and managing releases.
2. **One Worktree per Agent**:
   Every builder and tester operates in its own directory under `../vdb-worktrees/vdb-agent-<name>` on its own branch `agent/<track>/<name>`. Agents never modify the primary checkout simultaneously.
3. **Dedicated Target Directories**:
   Every worktree configures an isolated `target-dir = "../../vdb-targets/target-<name>"` in `.cargo/config.toml`. This allows concurrent `cargo build` and `cargo test` processes to run in parallel without file-lock contention.
4. **Maker != Checker Separation**:
   A builder agent never grades its own work. An independent Tester agent writes adversarial tests against the builder's diff without reading the builder's unit tests. A Checklist agent verifies line-by-line compliance with [`docs/production-gap-audit.md`](production-gap-audit.md).
5. **Zero CI Cost**:
   No cloud CI or GitHub Actions minutes are consumed. All verification occurs through local gate scripts ([`scripts/local-gate.ps1`](../scripts/local-gate.ps1) / [`scripts/local-gate.sh`](../scripts/local-gate.sh)).
6. **Frequent Commits & Clean `main`**:
   Work is committed incrementally in small, clear commits. The `main` branch remains 100% green and deployable at all times.

---

## 2. Directory Layout

```text
E:\
├── vdb\                               # Primary repository (Integrator runs here on main)
│   ├── .cargo\
│   ├── crates\
│   ├── docs\
│   ├── scripts\
│   └── target\                        # Primary target directory
│
├── vdb-worktrees\                     # Active worktrees for parallel agents
│   ├── vdb-agent-storage\             # Track A: Storage / Encryption builder
│   ├── vdb-agent-platform\            # Track B: Windows Locking builder
│   ├── vdb-agent-test-storage\        # Independent tester for Track A
│   └── vdb-agent-test-platform\       # Independent tester for Track B
│
└── vdb-targets\                       # Isolated build artifacts (prevents lock contention)
    ├── target-storage\
    ├── target-platform\
    ├── target-test-storage\
    └── target-test-platform\
```

---

## 3. Workflow Commands

### Provisioning a Worktree
To provision a worktree for a new track or agent:

```powershell
# PowerShell (Windows)
powershell -ExecutionPolicy Bypass -File scripts/bootstrap-worktree.ps1 -AgentName storage -Track storage

# Bash (Linux/macOS)
./scripts/bootstrap-worktree.sh storage storage
```

### Running the Local Gate (Zero CI Cost)
Run inside any worktree or on `main`:

```powershell
# PowerShell (Windows)
powershell -ExecutionPolicy Bypass -File scripts/local-gate.ps1

# Bash (Linux/macOS)
./scripts/local-gate.sh
```

### Tearing Down a Worktree
When a track is merged to `main`:

```powershell
# PowerShell (Windows)
powershell -ExecutionPolicy Bypass -File scripts/teardown-worktree.ps1 -AgentName storage

# Bash (Linux/macOS)
./scripts/teardown-worktree.sh storage
```

---

## 4. Parallel Track Roadmap

```text
Phase 0 — Foundations (Sequential)
  Integrator: local-gate scripts, worktree bootstrap, target-dir isolation, checklist templates
  ↓
Phase 1 — P0 Durability & Security (Parallel, 2 independent tracks)
  Track A: Storage finishes encrypted-vdb3 ──┐
                    + Test-A (adversarial)   ├── Checklist-A → Integrator merges A
                    + Checklist-A             │
  Track B: Platform Windows locking + crash ─┘   (A and B don't share files → run concurrently)
                    + Test-B + Checklist-B → Integrator merges B
  → Tag v0.3.0, Regression run on main
  ↓
Phase 2 — P1 Scale & APIs (Parallel, contract-first)
  Storage publishes segment/checkpoint trait stub ──┬──────────────┐
  Track C: Storage implements segments + LRU       │              │
  Track D: Query builds range/compound index       │ (parallel)   │
  Track E: API builds vdb-core crate + local IPC  ──┘              │
  each track: + Test agent + Checklist agent                      │
  → Integrator merges in dependency order: C, then D, then E
  → Tag v0.4.0, Regression run on main
  ↓
Phase 3 — P2 AI Steward & Fuzzing (Parallel, fully independent)
  Track F: Security/Fuzz (cargo-fuzz suites) ──┐
  Track G: Steward model-backed adapter      ──┤ (no shared files → fully parallel)
  each + Test agent + Checklist agent          │
  → Integrator merges both → Tag v0.5.0, Regression run on main
  ↓
Phase 4 — Demand Validation Gate (User-run)
  Execute pilot interview protocol from docs/demand-validation.md
  ↓
Phase 5 — Packaging & Hardening (Sequential)
  Documentation, licensing audit, release packaging → Tag v1.0.0
```

---

## 5. Contract-First Interface Pattern

When one track depends on another (e.g. Query engine needing the Storage segment reader):
1. **Hour 1**: Upstream builder commits the trait definitions and public types to a stub branch:
   ```rust
   pub trait SegmentReader {
       fn read_block(&self, segment_id: u64, offset: usize) -> Result<Vec<u8>, VdbError>;
   }
   ```
2. **Parallel Hour 2+**:
   * Upstream implements the physical file I/O behind the trait.
   * Downstream writes the query planner and filter loops calling `SegmentReader`.
3. **Integration**:
   * Upstream merges to `main`. Downstream rebases onto `main` and swaps the trait mock for the real engine without blocking or waiting.

---

## 6. Merge & Rollback Discipline

* **Merge Order**: Integrator merges strictly in order of checklist completeness and priority (P0 before P1).
* **Regression on Main**: After any merge into `main`, the Integrator immediately runs `scripts/local-gate.ps1`.
* **Immediate Revert**: If regression fails on `main`, the Integrator immediately reverts the merge commit (`git revert HEAD`). `main` is never allowed to remain broken.
* **Phase Tags**: Every phase boundary is tagged (`v0.3.0`, `v0.4.0`, `v0.5.0`, `v1.0.0`) providing coarse-grained rollback points in addition to fine-grained individual commits.
