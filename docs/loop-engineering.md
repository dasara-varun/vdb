# VDB Loop Engineering and Session Handoffs

This guide defines how a person or AI agent may continue VDB work across many sessions without turning the project into an unbounded or unsupervised mutation loop. A loop is a repeatable cycle of discovery, one bounded change, independent verification, documentation, and a decision about the next step.

> **Safety boundary:** A request to “keep going until VDB is ready” does not authorize unlimited execution, automatic merging, destructive operations, credential use, or claims of production readiness. Every cycle must have a written scope, a bounded budget, a verifiable finish condition, and an escalation path.

## The VDB development loop

| Stage | Required action | Evidence written to the repository |
|---|---|---|
| Reconstruct | Read `AGENTS.md`, `docs/README.md`, `docs/project-status.md`, the relevant guide, recent history, and current `git status` | Current commit and working-tree state |
| Discover | Select one highest-priority gap from `docs/production-gap-audit.md` or `docs/improvement-plan.md` | Issue, task note, or progress entry |
| Design | Define scope, non-goals, threat impact, migration impact, tests, and rollback | ADR for architectural changes; plan for smaller changes |
| Implement | Make one coherent change in an isolated branch or worktree | Focused diff and small commits |
| Verify | Run targeted tests, then the full repository gate and relevant CLI/crash/security checks | Exact commands and results |
| Review | Use an independent review pass; the maker must not be the only judge | Review findings or explicit no-findings record |
| Document | Update source-of-truth guides, status, changelog, diagrams, and limitations | Synchronized documentation |
| Handoff | Commit and push to `main` only after authorization and green CI; otherwise leave a clean branch and a handoff note | Commit, CI URL, remaining risks, next bounded task |

## Recommended VDB loop shapes

### Goal-to-green loop

Use this for one issue with a clear machine-verifiable finish line. The loop may edit only the files named in its plan and must stop when the tests, review, and documentation conditions pass. A suitable VDB condition is: the targeted regression test passes, the full Rust gate passes, `scripts/check-docs.sh` passes, and the production-gap document reflects the remaining limitations.

### Watch-and-fix loop

Use this for a single CI run or a known failing test. The loop may inspect logs and prepare one fix at a time. It must stop after a bounded number of failed hypotheses, when the failure is unrelated to the current change, or when a credential, destructive command, merge conflict, or production system is encountered.

### Backlog loop

Use this only for a small, explicitly labeled queue of independent tasks. Each item must be implemented and verified in its own branch or commit. The loop must not silently broaden the queue, rewrite shared history, or close issues without evidence. Architectural, encryption, replication, authentication, and model-agent tasks require a human checkpoint before implementation.

### Session handoff loop

At the end of a session, write a handoff rather than assuming a future session will remember the conversation. Use [`templates/progress-update.md`](templates/progress-update.md) for milestones and record:

```text
Current commit and branch:
Working tree state:
Completed in this cycle:
Validation performed:
Documentation changed:
Known failures or unresolved risks:
Exact next bounded task:
Files that must be read first:
Stop conditions and human decisions required:
```

A future agent should start by reading the handoff and verifying it against the repository. It must not trust a stale “next task” if the code, branch, or project status has changed.

## Bounds and authorization

Every loop definition must include all of the following:

| Control | VDB default |
|---|---|
| Scope | One issue, one feature slice, or one clearly related hardening change |
| Time/turn budget | One session or an explicitly stated finite limit |
| Filesystem boundary | VDB checkout and an isolated temporary directory only |
| Branch boundary | Feature branch or current branch with no force-push |
| Data boundary | Synthetic test data; never production or customer databases by default |
| Network boundary | Public documentation and GitHub CI only unless explicitly authorized |
| Finish condition | Tests, review, docs, and CI evidence, not a subjective claim |
| Escalation | Stop and write a handoff for uncertainty, security findings, migration risk, or repeated failure |
| Merge policy | No automatic merge, release, deployment, or deletion |

A scheduled task or persistent worker is not a substitute for these controls. It must be configured in an environment that remains online, use a least-privilege identity, keep durable state, and expose logs and an emergency stop. The default sandbox can hibernate and therefore cannot guarantee that a loop survives a session boundary.

## What “ready” means for VDB

VDB is ready for a release only when the explicit gates in [`production-gap-audit.md`](production-gap-audit.md) are satisfied, not merely when the CLI works locally. Before a full-fledged release, the project needs reviewed authenticated encryption and key management, dependable cross-platform locking, crash and fault-injection evidence, bounded storage and query behavior, authentic dependency locking and auditing, stable API/authentication design, and a reviewed Steward threat model. Server mode, replication, and autonomous model actions are separate milestones and must not be smuggled into an MVP loop.

## Security checklist for every loop

Before starting, confirm that the loop cannot read or write secrets, real user databases, SSH keys, cloud credentials, or unrelated home-directory files. Before committing, scan the diff and logs for credentials, verify that no generated memory database or tool cache entered Git, and inspect changes to file permissions, subprocesses, network access, serialization, and path handling. After completion, report any uncertainty instead of suppressing it.

Third-party skills are advisory workflow components. They do not grant authority to bypass VDB’s `AGENTS.md`, security model, accepted ADRs, review policy, or user confirmation requirements. The maker/checker separation is mandatory for security-sensitive changes.

## References

1. [VDB agent-tooling guide](agent-tooling.md)
2. [VDB documentation maintenance](documentation-maintenance.md)
3. [VDB production-gap audit](production-gap-audit.md)
4. [VDB improvement plan](improvement-plan.md)
5. [VDB testing strategy](testing.md)
6. [Loop Engineering source repository](https://github.com/iamarvindh/claude-loop-engineering)
7. [Claude Code memory guidance](https://code.claude.com/docs/en/memory)

_Last reviewed: 2026-08-24._
_Last reviewed by: Manus AI._
