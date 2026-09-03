# Acceptance Checklist & Verification Report

**Branch:** `agent/<track>/<name>`  
**Maker/Builder Agent:** `<agent-name>`  
**Tester Agent:** `<test-agent-name>`  
**Verifier / Checklist Agent:** `<verifier-name>`  
**Date:** `YYYY-MM-DD`  
**Target Milestone:** `v0.X.0`  

---

## 1. Scope & Gap Audit Mapping

Identify which items from [`docs/production-gap-audit.md`](../production-gap-audit.md) and [`docs/improvement-plan.md`](../improvement-plan.md) this branch addresses:

| Gap ID / Area | Gap Description | Priority | Claimed State (`PASS` / `N/A`) |
|---|---|---|---|
| GAP-P0-DUR | Durability proof across filesystems (directory sync, replace semantics, crash matrix) | P0 | [ ] |
| GAP-P0-ENC | Authenticated encryption at rest (AES-256-GCM, key provider, tamper tests, backup drills) | P0 | [ ] |
| GAP-P0-LCK | OS-level advisory locking (Windows `LockFileEx` / Unix rustix advisory lock) | P0 | [ ] |
| GAP-P1-MEM | Memory-bounded storage (disk segments, checkpoints, bounded LRU caches) | P1 | [ ] |
| GAP-P1-QRY | Query planning & richer filtering (explain plan, range queries, compound indexes) | P1 | [ ] |
| GAP-P1-API | Narrow local application API / IPC (`vdb-core` crate API, local IPC) | P1 | [ ] |
| GAP-P2-STW | Model-backed Steward adapter (read-only, schema-validated, prompt-injection hardened) | P2 | [ ] |
| GAP-P2-FUZ | Property, fuzz, and crash testing (cargo-fuzz suites, fault injection) | P2 | [ ] |
| GAP-BASE-WAL | WAL storage quota and bounded growth | Baseline | [ ] |
| GAP-BASE-INP | Input and replay hardening (fail-closed, streaming, semantic validation) | Baseline | [ ] |
| GAP-BASE-PRV | Private local artifacts (Unix 0600 modes, overwrite rejection) | Baseline | [ ] |
| GAP-BASE-BAK | Verified backup and restore-to-new-path isolation | Baseline | [ ] |
| GAP-BASE-GUI | Loopback read-only GUI (127.0.0.1 only, HTML-escaped, no mutation) | Baseline | [ ] |
| GAP-BASE-DEP | Reproducible dependency supply chain (locked build, cargo-deny, RustSec) | Baseline | [ ] |

---

## 2. Independent Tester Evidence (Maker != Checker)

*The Tester agent evaluates the diff independently without reading the Maker agent's test file.*

### 2.1 Adversarial & Negative Test Results
- [ ] **Corrupted / Tampered Inputs**: Tested with bit-flipped headers, truncated records, and invalid lengths.
- [ ] **Security & Key Boundary**: Tested with wrong keys, missing keys, expired keys, or unauthorized paths.
- [ ] **Concurrency & Contention**: Tested concurrent opens, lock collisions, and stale lock recovery.
- [ ] **Boundary Conditions**: Tested zero values, maximum budgets, and buffer overflow attempts.

```text
Evidence log / Test command output:
<Paste exact command and results here>
```

---

## 3. Local Gate Verification (Zero CI Cost)

Run locally before submitting for merge consideration:

- [ ] **Formatting**: `cargo fmt --all -- --check` exited `0`
- [ ] **Clippy**: `cargo clippy --workspace --all-targets -- -D warnings` exited `0`
- [ ] **Workspace Tests**: `cargo test --workspace` passed all tests (`0` failed)
- [ ] **Release Build**: `cargo build --workspace --release` exited `0`
- [ ] **Local Smoke**: `scripts/local-gate.ps1` (or `local-gate.sh`) passed completely
- [ ] **Documentation**: `docs/check-docs.sh` passed

---

## 4. Documentation & Contract Synchronization

- [ ] Guide in `docs/` updated or created for changed subsystem
- [ ] `docs/project-status.md` updated with exact commit and capability status
- [ ] `docs/production-gap-audit.md` updated if gap status changed
- [ ] `CHANGELOG.md` updated with user-visible changes
- [ ] No secrets, credentials, or personal tokens in diff or commit messages

---

## 5. Integrator Verdict & Decision

- [ ] **APPROVED FOR FAST-FORWARD MERGE**
- [ ] **REVISE AND RESUBMIT** (Issues noted below)
- [ ] **REJECTED**

### Findings / Concerns:
```text
<Document any non-blocking observations, assumptions, or follow-up backlog items>
```

**Signed off by Integrator:** `_____________________`  
**Merge Commit:** `_____________________`
