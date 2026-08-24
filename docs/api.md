# VDB MVP API Contract

The API is document-oriented and intentionally small. HTTP and SDK implementations should share these semantics.

## Document model

```json
{
  "_id": "u_123",
  "_version": 3,
  "_created_at": "2026-08-24T00:00:00Z",
  "_updated_at": "2026-08-24T00:01:00Z",
  "name": "Asha",
  "plan": "pro"
}
```

System fields are reserved. User payloads may contain nested objects and arrays. The server must reject invalid document size, reserved-field collisions, and unsupported values with stable error codes.

## Core operations

| Operation | Semantics |
|---|---|
| Create collection | Creates metadata and an empty collection; idempotent when requested |
| Put document | Inserts or replaces by `_id`; supports optional expected version |
| Get document | Returns one document or a typed not-found result |
| Query | Uses bounded filters, projections, sort, limit, and cursor pagination |
| Patch document | Applies a validated patch only when expected version matches |
| Delete document | Requires explicit id or bounded filter; destructive bulk deletes require approval |
| Create index | Creates a validated single-field equality index |
| List indexes | Reports indexed fields and document/value coverage |
| Health | Returns storage, errors, query latency, schema, and backup summaries |
| Backup | Creates a checksummed snapshot manifest |
| Backup verify | Reopens a snapshot and validates its manifest and health |
| Export | Writes portable JSON Lines records |
| Import | Validates JSON Lines records and writes them through normal document rules |

## Conditional updates

Clients should use optimistic concurrency for updates:

```http
PATCH /v1/collections/users/documents/u_123
If-Match: 3
Content-Type: application/json

{"plan":"enterprise"}
```

A conflict returns:

```json
{
  "error": {
    "code": "VERSION_CONFLICT",
    "message": "The document changed after it was read.",
    "resource": "users/u_123",
    "expected_version": 3,
    "current_version": 4,
    "retryable": true
  }
}
```

## Query limits

Every query has a maximum result count, maximum document bytes, timeout, and optional scan budget. The API should require cursor pagination for large result sets. VDB should reject unbounded destructive filters and return a clear remediation message.

## Health response

```json
{
  "status": "healthy",
  "storage": {"bytes_used": 4096000, "wal_bytes": 12000},
  "backup": {"last_snapshot": "2026-08-24T00:00:00Z", "last_verified_restore": "2026-08-23T00:00:00Z"},
  "queries": {"p95_ms": 14, "bounded_queries": 100},
  "findings": []
}
```

## Steward findings

The read-only Steward API returns findings rather than executable commands:

```json
{
  "id": "finding_01",
  "kind": "SCHEMA_DRIFT",
  "severity": "medium",
  "evidence": ["users.plan changed from string to array in 4.2% of recent writes"],
  "confidence": 0.91,
  "recommendation": "Review and approve a collection validation rule or migration plan.",
  "actions": [],
  "approval_required": true
}
```

Any future action plan must use a typed allowlist. No API accepts arbitrary SQL, shell commands, model-generated code, or unvalidated database commands.

## Error codes

`INVALID_DOCUMENT`, `DOCUMENT_NOT_FOUND`, `VERSION_CONFLICT`, `QUERY_LIMIT_EXCEEDED`, `QUERY_TIMEOUT`, `COLLECTION_NOT_FOUND`, `INDEX_CONFLICT`, `BACKUP_FAILED`, `RESTORE_VERIFICATION_FAILED`, `UNAUTHORIZED`, `FORBIDDEN`, `APPROVAL_REQUIRED`, `POLICY_DENIED`, and `STORAGE_RECOVERY_REQUIRED`.

## CLI examples

```bash
vdb --path ./app.vdb index-create users plan
vdb --path ./app.vdb index-list users
vdb --path ./app.vdb export ./users.jsonl
vdb --path ./app.vdb import ./users.jsonl
vdb --path ./app.vdb backup-verify ./backups/app.vdb
```

## Compatibility policy

API versions are explicit, beginning with `/v1`. Error codes and document semantics are stable contracts. Storage files must carry a format version and migration path. VDB should provide export/import tools so users are not trapped in a private format.
