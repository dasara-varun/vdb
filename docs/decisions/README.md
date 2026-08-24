# VDB Architectural Decision Records

Architectural Decision Records (ADRs) preserve decisions that future contributors should not have to rediscover from code or chat history. An ADR records context, alternatives, consequences, evidence, and validation. Accepted decisions should be treated as constraints until a superseding ADR is accepted.

## Decision index

| ADR | Title | Status |
|---|---|---|
| [ADR-0001](0001-local-first-read-only-steward.md) | Local-first storage with a read-only Steward by default | Accepted |
| [ADR-0002](0002-loopback-read-only-gui.md) | Opt-in loopback read-only GUI | Accepted |

## When to create an ADR

Create an ADR when a change affects the storage format, durability or recovery contract, trust boundaries, encryption, networking, authentication, replication, Steward authority, dependency policy, public API compatibility, or another decision that will shape future work. Do not create an ADR for routine refactoring unless it changes a project constraint.

## ADR rules

Use [`docs/templates/decision-record.md`](../templates/decision-record.md). Number records sequentially. Keep accepted records immutable except for status metadata and links to superseding decisions. If a decision changes, create a new ADR that explicitly supersedes the old one. Link the ADR from affected technical guides, the project-status snapshot, and the changelog when user-visible behavior changes.
