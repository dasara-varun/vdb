# VDB Product Requirements

## Product promise

VDB is a document database for people who want to store application data easily and receive safe, understandable help maintaining it.

## Personas

| Persona | Primary need | Success signal |
|---|---|---|
| Beginner developer | Store documents without learning database administration | Creates a working instance in minutes |
| Application developer | Integrate a predictable local database through an SDK/API | Ships a feature with bounded queries and clear errors |
| Small-team lead | Reduce backup, health, and troubleshooting burden | Uses health reports and verified restores |
| Operator | Inspect and control storage, policies, and recovery | Can audit and reproduce every privileged action |
| Privacy-sensitive builder | Keep data and AI context local | No raw data leaves the instance by default |

## Core workflows

### Create and use

A user chooses a template or custom collection, adds a document through a form, CLI, or SDK, and immediately sees the document ID, version, and storage status. The default experience should not ask the user to understand encoding, indexes, partitions, or replication.

### Diagnose

The user asks why a collection is slow or growing. VDB shows the relevant evidence, distinguishes facts from inferences, explains likely causes, and offers a recommendation. It does not silently change the database.

### Approve a change

The user inspects scope, expected benefit, estimated storage/write cost, risk, preconditions, and rollback. Approval is explicit. VDB creates a snapshot or canary where appropriate, executes a typed operation, verifies postconditions, and records the audit event.

### Recover

The user sees the last verified backup, chooses a restore point, restores to a sandbox first, compares health and checksums, and then chooses whether to replace or export the production instance. Recovery must be understandable without reading internal logs.

## Usability principles

VDB uses progressive disclosure. Beginners see guided workflows and plain-language status. Developers receive CLI, SDK, HTTP, and configuration interfaces. Experts can inspect metrics, query plans, policies, and audit events. Advanced behavior is available but never required for the first successful document.

Errors explain what happened, why it happened, and what can be done next. Technical codes, request IDs, and logs remain available under details. The product supports keyboard navigation, screen readers, high contrast, reduced motion, and non-color status indicators.

## MVP acceptance criteria

- Create, read, update, delete, and bounded equality-filter operations work with nested documents; pagination and TTL are future capabilities, not current MVP promises.
- Conditional updates return a clear version-conflict error and do not overwrite newer data.
- Query and storage limits prevent accidental unbounded reads and uncontrolled WAL growth.
- Snapshots are checksummed and restorable to a sandbox; authenticated encryption remains a production gate and is not yet implemented.
- Health diagnostics identify current schema, document, payload, and WAL growth; backup staleness and query-budget telemetry require future operational instrumentation.
- Steward findings contain evidence, severity, confidence, and a recommendation; no finding is an executable command.
- Default instances use conservative limits and start in local read-only Observe mode for Steward analysis.
- CLI behavior is covered by automated tests and examples; an application API remains future work.

## Out of scope

The MVP does not promise distributed replication, active-active writes, arbitrary migration, full MongoDB compatibility, automatic destructive changes, or remote AI inference by default.
