# VDB Production Gap Audit

## Verified repository state

The selected repository is `dasara-varun/vdb`, its default branch is `main`, and the local checkout matches `origin/main` at commit `ae0adcbe173a879133628820919c8262fee6e82c`. The Rust rebuild and its documentation are therefore pushed to GitHub and present on `main`.

## Current MVP status

VDB is not finished as a production database. It is a working Rust MVP with a durable append-only CBOR WAL, checksums, replay, document versions, bounded queries, schema reports, health diagnostics, backups, backup verification, read-only Steward output, CLI workflows, tests, release benchmarking, and CI.

## Highest-value next work

| Priority | Gap | Why it matters | Next action |
|---|---|---|---|
| P0 | Explicit process-level instance lock | Multiple processes writing the same file could corrupt the log or create undefined behavior | Add a cross-platform lock strategy or fail clearly when shared writes are unsupported |
| P0 | File-format version and migration marker | A durable database needs to reject incompatible future binaries safely | Add a header/format metadata record and compatibility checks |
| P0 | Export/import escape hatch | Users need a human-readable recovery and migration path | Add JSON Lines export and validated import commands |
| P1 | Secondary indexes | Full scans will become slow as collections grow | Add declared equality indexes and measure write/read trade-offs |
| P1 | Compaction/snapshots | The WAL grows forever in the current prototype | Add checkpoint snapshots and safe log rotation |
| P1 | Encryption at rest | Current files are not encrypted | Add a well-reviewed encryption design after key-management decisions |
| P1 | Local HTTP API | CLI is useful for humans but applications need a stable API | Add localhost-only API with bounded request validation |
| P2 | Authentication and remote server | Needed only once network serving exists | Design after local API and threat model are tested |
| P2 | Model-backed Steward | Deterministic findings should remain useful without a model | Add optional local/private model adapter only after tool validation |
| P2 | Replication | High distributed-systems complexity | Defer until recovery, format migration, and compaction are stable |

## Decision

Continue with P0 reliability and escape-hatch features before adding AI autonomy or distributed replication. This sequence reduces user lock-in and data-loss risk while improving day-one usability.
