# VDB Security and Safety Model

## Security objective

VDB should be secure by reducing authority. The database remains the source of truth, the policy engine is deterministic, and the Steward is an untrusted analyst. VDB must fail closed when a request is malformed, over budget, unauthenticated, outside scope, or missing approval.

## Default posture

A new local instance binds to localhost only, runs as a non-root user, enables encryption when configured, limits document and query sizes, enables snapshots, and starts the Steward in Observe mode. Remote access, external model calls, and automatic changes are opt-in.

## Identity and authorization

The MVP separates application identity, administrator identity, backup identity, and Steward identity. The Steward receives read-only access to health metadata. Future action capabilities must be short-lived, scoped to an instance and collection, restricted by operation type, and recorded in the audit ledger.

The authorization model is deny-by-default. A capability must specify its actor, resource, operation, expiry, budget, and approval requirement. The executor rejects actions that do not match the capability exactly.

## AI threat model

| Threat | Example | Required control |
|---|---|---|
| Prompt injection | A document contains instructions telling the Steward to export all data | Treat documents as untrusted data; isolate policy from retrieved content; validate actions deterministically |
| Excessive agency | The model attempts a shell command or privilege change | Typed tools only; no generic shell; least privilege; approval gates |
| Sensitive-data disclosure | Raw documents enter a remote prompt or audit log | Redaction, field masking, local inference default, output scanning |
| Data poisoning | Malicious feedback changes future recommendations | Approved feedback only; versioned memory; evaluation before deployment |
| Insecure model output | Natural language is interpreted as an executable query | Require schema-validated plans and explicit operation allowlists |
| Supply-chain compromise | A plugin or model package introduces unsafe behavior | Pinned versions, signatures, provenance, SBOM, sandboxed extensions |
| Audit tampering | An attacker deletes evidence of an AI action | Append-only events, hash chaining or remote export, restricted deletion |

## Data minimization

The Steward should receive metrics, query fingerprints, schema summaries, and redacted samples rather than the complete database. Sensitive fields should be classified using explicit user rules plus conservative heuristics. Raw data access should be a separate, visible diagnostic mode with approval and automatic expiration.

## Audit events

Every privileged event records the timestamp, actor, capability, request hash, target, policy decision, approval, before/after configuration, result, error, and rollback reference. Audit records should be exportable for incident analysis and should never contain raw secrets.

## Backup and recovery

Backup success means more than a file being written. VDB must record snapshot checksums, metadata, source instance version, encryption key identifier, and last restore verification. Restore tests use an isolated sandbox and compare checksums, document counts, and representative reads.

## Secure development requirements

The project should use dependency pinning, secret scanning, static analysis, fuzz/property tests for the document boundary, crash-injection tests for durability, and adversarial tests for prompt injection and unsafe action plans. Security-sensitive changes require review and a regression test.
