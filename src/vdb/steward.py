from __future__ import annotations

from dataclasses import dataclass, asdict
from typing import Any

from .store import VDBStore


@dataclass(frozen=True)
class Finding:
    id: str
    kind: str
    severity: str
    evidence: list[str]
    confidence: float
    recommendation: str
    approval_required: bool = True

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


class Steward:
    """Read-only deterministic health analyst for the VDB MVP."""

    def __init__(self, store: VDBStore):
        self.store = store

    def inspect(self, collection: str | None = None) -> list[Finding]:
        findings: list[Finding] = []
        health = self.store.health()
        if health["wal_bytes"] > 64 * 1024 * 1024:
            findings.append(
                Finding(
                    id="wal-growth",
                    kind="STORAGE_PRESSURE",
                    severity="medium",
                    evidence=[f"write-ahead log is {health['wal_bytes']} bytes"],
                    confidence=0.98,
                    recommendation="Create a verified snapshot and investigate checkpoint frequency before enabling automation.",
                )
            )
        if collection:
            schema = self.store.schema_report(collection)
            fields = schema["fields"]
            mixed = [key for key, types in fields.items() if len(types) > 1]
            if mixed:
                findings.append(
                    Finding(
                        id=f"schema-drift-{collection}",
                        kind="SCHEMA_DRIFT",
                        severity="medium",
                        evidence=[f"fields with multiple observed types: {', '.join(sorted(mixed))}"],
                        confidence=0.88,
                        recommendation="Review the field contract and approve validation or a migration plan; do not reject new writes automatically.",
                    )
                )
        if health["documents"] == 0:
            findings.append(
                Finding(
                    id="empty-instance",
                    kind="INFO",
                    severity="low",
                    evidence=["the instance contains no documents"],
                    confidence=1.0,
                    recommendation="Create a collection and add a test document, or restore a verified snapshot.",
                    approval_required=False,
                )
            )
        return findings

    def explain(self, collection: str | None = None) -> dict[str, Any]:
        health = self.store.health()
        return {
            "health": health,
            "findings": [finding.to_dict() for finding in self.inspect(collection)],
            "mode": "observe",
            "actions": [],
            "message": "VDB Steward is read-only; findings are recommendations, not executable commands.",
        }
