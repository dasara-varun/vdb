# Diagram Review

The rendered architecture diagram is readable as a wide system overview. It shows the client/API boundary, request validation and budgets, document store, embedded storage/WAL/snapshots, telemetry, deterministic diagnostics, redacted model context, typed plans, policy approval, restricted execution, verification, audit, and restore sandbox.

The rendered recovery-flow diagram is readable as a tall operational flow. It shows detection, evidence preservation, risk classification, snapshot verification, sandbox simulation, typed planning, policy and human approval, bounded execution, postcondition verification, and either audited commit or rollback/restore. The source diagrams remain in Mermaid format so they can be edited and regenerated.
