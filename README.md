# VDB

**VDB** is a simple, local-first document database with a safe maintenance assistant. It is designed to let developers store JSON-like documents without needing to become database administrators.

> VDB stores your documents simply, proves that your backups work, explains database problems, and never changes important data without your approval.

## MVP status

This repository starts with a focused MVP: a Python document API backed by a mature embedded storage layer, bounded queries, schema inspection, health diagnostics, encrypted snapshot hooks, and a read-only Steward interface. Distributed replication, destructive automation, and arbitrary AI tool execution are intentionally out of scope for the first release.

## Design principles

VDB is document-oriented at the user boundary, conservative in its storage behavior, and explicit about its limits. The AI layer is an untrusted analyst, not a database superuser. Detection begins with deterministic rules; model-assisted explanations and recommendations must pass typed validation before they can become actions.

VDB defaults to local-only access, bounded reads, explicit versions, safe errors, auditable administrative changes, and human approval for anything destructive or difficult to reverse.

## Repository map

| Path | Purpose |
|---|---|
| `src/vdb/` | Core document database, health checks, and CLI/API primitives |
| `tests/` | Unit and integration tests |
| `docs/` | Product, architecture, security, API, and roadmap documentation |
| `diagrams/` | Mermaid architecture and data-flow diagrams |
| `pyproject.toml` | Python package metadata and development tooling |

## Planned quick start

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e '.[dev]'
pytest
vdb --help
```

## Safety boundary

The first implementation does not expose a generic shell tool, unrestricted database command execution, remote model calls, or automatic destructive changes. Any future Steward action must have a typed operation, scope, precondition, budget, approval policy, audit event, and rollback or recovery path.

## License

License to be selected before the first public release. The current repository is an evaluation and development project.
