# VDB Documentation Index

This directory is the project’s durable knowledge base. It explains what VDB is, how it works, how to operate it safely, what is implemented, what is planned, and how future contributors should update that information. Start with this index instead of searching files randomly.

## Fast navigation

| Need | Read first | Then read |
|---|---|---|
| Understand VDB quickly | [`README.md`](../README.md) | [`product.md`](product.md), [`project-status.md`](project-status.md) |
| Install VDB on a device | [`installation.md`](installation.md) | [`operations.md`](operations.md), [`security.md`](security.md) |
| Build or modify Rust code | [`../AGENTS.md`](../AGENTS.md) | [`architecture.md`](architecture.md), [`testing.md`](testing.md), [`roadmap.md`](roadmap.md) |
| Operate a local database | [`operations.md`](operations.md) | [`data-format.md`](data-format.md), [`testing.md`](testing.md) |
| Review security | [`security.md`](security.md), [`../SECURITY.md`](../SECURITY.md) | [`production-gap-audit.md`](production-gap-audit.md), [`improvement-plan.md`](improvement-plan.md), [`steward.md`](steward.md) |
| Understand storage compatibility | [`data-format.md`](data-format.md), [`durability-matrix.md`](durability-matrix.md) | [`operations.md`](operations.md), [`testing.md`](testing.md) |
| Understand the Steward | [`steward.md`](steward.md) | [`security.md`](security.md), [`product.md`](product.md) |
| Evaluate performance | [`performance.md`](performance.md) | [`benchmarks.md`](benchmarks.md), [`improvement-plan.md`](improvement-plan.md) |
| Assess demand and product direction | [`demand-validation.md`](demand-validation.md), [`developer-research.md`](developer-research.md), [`gui-research.md`](gui-research.md) | [`product.md`](product.md), [`roadmap.md`](roadmap.md) |
| Record a new architectural decision | [`decisions/README.md`](decisions/README.md) | [`templates/decision-record.md`](templates/decision-record.md) |
| Update project knowledge | [`documentation-maintenance.md`](documentation-maintenance.md) | [`project-status.md`](project-status.md), [`templates/progress-update.md`](templates/progress-update.md) |
| Use Claude Code tooling safely | [`agent-tooling.md`](agent-tooling.md) | [`loop-engineering.md`](loop-engineering.md), `AGENTS.md` |
| Design a bounded continuation loop | [`loop-engineering.md`](loop-engineering.md) | [`agent-tooling.md`](agent-tooling.md), `production-gap-audit.md` |

## Source-of-truth map

| Information | Canonical file | Do not use as the only source |
|---|---|---|
| Product promise and scope | `docs/product.md` | README marketing copy or chat history |
| Current implementation state | `docs/project-status.md` | Roadmap or old changelog entries |
| Durable storage behavior | `docs/data-format.md` and the tested implementation | Informal examples that omit failure behavior |
| Operational procedures | `docs/operations.md` | Source comments alone |
| Security posture | `docs/security.md` and `docs/production-gap-audit.md` | Unreviewed issue or model output |
| Performance claims | `docs/performance.md` and `docs/benchmarks.md` | Unlabeled local measurements |
| Steward authority | `docs/steward.md` and `docs/security.md` | Prompt text or prototype behavior |
| Future work | `docs/roadmap.md` and `docs/improvement-plan.md` | A single task description |
| Architectural rationale | `docs/decisions/` | Commit message alone |
| Chronological user-visible changes | `CHANGELOG.md` | Status snapshot alone |

## Complete document catalog

| File | Description |
|---|---|
| [`api.md`](api.md) | MVP API and CLI contract |
| [`architecture.md`](architecture.md) | Trust zones, data flow, recovery, and MVP boundary |
| [`agent-tooling.md`](agent-tooling.md) | Verified Claude Code extensions, installation scope, privacy, and security rules |
| [`audit-research.md`](audit-research.md) | Research notes and source links from the latest audit |
| [`benchmarks.md`](benchmarks.md) | Measured baseline and benchmark limitations |
| [`data-format.md`](data-format.md) | `VDB1` header, CBOR WAL, checksums, replay, and compatibility |
| [`demand-validation.md`](demand-validation.md) | Demand signals, interview plan, and pilot thresholds |
| [`developer-research.md`](developer-research.md) | Developer adoption lessons, competing database comparisons, and product roadmap |
| [`gui-research.md`](gui-research.md) | Secure local GUI architecture, trust boundaries, and first-milestone design |
| [`crypto-research.md`](crypto-research.md) | AEAD and key-management research constraints for future encrypted storage |
| [`decisions/0003-encrypted-storage-and-key-provider.md`](decisions/0003-encrypted-storage-and-key-provider.md) | Proposed encrypted VDB3 format and external file-key provider design |
| [`durability-matrix.md`](durability-matrix.md) | Tested recovery behavior, filesystem assumptions, and release durability gaps |
| [`diagram-review.md`](diagram-review.md) | Review notes for the rendered diagrams |
| [`documentation-maintenance.md`](documentation-maintenance.md) | Required maintenance process for this knowledge base |
| [`../SECURITY.md`](../SECURITY.md) | Vulnerability reporting and security disclosure policy |
| [`improvement-plan.md`](improvement-plan.md) | Ranked research-backed technical roadmap |
| [`installation.md`](installation.md) | Cross-platform build, installation, release, and privacy guidance |
| [`loop-engineering.md`](loop-engineering.md) | Bounded automation, session handoffs, and stop conditions |
| [`operations.md`](operations.md) | Local deployment, backup, compaction, and recovery runbook |
| [`performance.md`](performance.md) | Performance architecture and measurement principles |
| [`product.md`](product.md) | Product promise, personas, workflows, and acceptance criteria |
| [`production-gap-audit.md`](production-gap-audit.md) | Honest production-readiness gaps and priorities |
| [`project-status.md`](project-status.md) | Maintained implementation and validation snapshot |
| [`roadmap.md`](roadmap.md) | Milestones and release gates |
| [`security.md`](security.md) | Security, privacy, and AI threat model |
| [`steward.md`](steward.md) | Constrained/read-only Steward specification |
| [`testing.md`](testing.md) | Reliability, adversarial, crash, and release testing |
| [`decisions/`](decisions/) | Accepted architectural decision records, including [`ADR-0002`](decisions/0002-loopback-read-only-gui.md) for the GUI boundary |
| [`templates/`](templates/) | Templates for decisions, progress updates, and research notes |

## How to use this index

When a document is added, renamed, superseded, or materially changed, update this index and the source-of-truth map in the same change. If two documents begin to disagree, choose one canonical owner, link to it from the other, and explain the transition in a decision record or changelog entry. The index is intentionally plain Markdown so it remains readable in GitHub, local editors, and future agent contexts. Run `./scripts/check-docs.sh` after adding or renaming guidance to verify the required files, diagrams, and navigation links.
