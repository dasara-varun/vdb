from __future__ import annotations

import hashlib
import json
import re
import shutil
import sqlite3
import time
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterator

import cbor2


_COLLECTION_RE = re.compile(r"^[A-Za-z][A-Za-z0-9_-]{0,62}$")
_RESERVED = {"_id", "_version", "_created_at", "_updated_at"}


class VDBError(Exception):
    """Base error for VDB operations."""


class ValidationError(VDBError):
    """Raised when a request violates the document or query contract."""


class NotFoundError(VDBError):
    """Raised when a requested collection or document does not exist."""


class VersionConflictError(VDBError):
    """Raised when optimistic concurrency detects a newer document version."""

    def __init__(self, collection: str, document_id: str, expected: int | None, current: int):
        self.collection = collection
        self.document_id = document_id
        self.expected = expected
        self.current = current
        super().__init__(
            f"document {collection}/{document_id} changed: expected version {expected}, "
            f"current version is {current}"
        )


@dataclass(frozen=True)
class Document:
    collection: str
    id: str
    version: int
    created_at: str
    updated_at: str
    data: dict[str, Any]

    def to_dict(self) -> dict[str, Any]:
        return {
            "_id": self.id,
            "_version": self.version,
            "_created_at": self.created_at,
            "_updated_at": self.updated_at,
            **self.data,
        }


class VDBStore:
    """Small local-first document database with a stable, bounded API."""

    def __init__(self, path: str | Path, *, max_document_bytes: int = 1_048_576):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.max_document_bytes = max_document_bytes
        self._db = sqlite3.connect(self.path, isolation_level=None)
        self._db.row_factory = sqlite3.Row
        self._db.execute("PRAGMA journal_mode=WAL")
        self._db.execute("PRAGMA foreign_keys=ON")
        self._db.execute("PRAGMA synchronous=FULL")
        self._initialize()

    def close(self) -> None:
        self._db.close()

    def __enter__(self) -> "VDBStore":
        return self

    def __exit__(self, _exc_type: Any, _exc: Any, _tb: Any) -> None:
        self.close()

    def _initialize(self) -> None:
        self._db.executescript(
            """
            CREATE TABLE IF NOT EXISTS collections (
                name TEXT PRIMARY KEY,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS documents (
                collection TEXT NOT NULL REFERENCES collections(name) ON DELETE CASCADE,
                document_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                payload BLOB NOT NULL,
                payload_size INTEGER NOT NULL,
                PRIMARY KEY (collection, document_id)
            );
            CREATE INDEX IF NOT EXISTS idx_documents_updated
                ON documents(collection, updated_at DESC);
            CREATE TABLE IF NOT EXISTS audit_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL,
                event_type TEXT NOT NULL,
                collection TEXT,
                document_id TEXT,
                details TEXT NOT NULL
            );
            """
        )

    @staticmethod
    def _now() -> str:
        return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")

    @staticmethod
    def _validate_collection(name: str) -> None:
        if not isinstance(name, str) or not _COLLECTION_RE.fullmatch(name):
            raise ValidationError(
                "collection names must start with a letter and contain only letters, numbers, "
                "hyphens, or underscores"
            )

    @staticmethod
    def _validate_document(data: dict[str, Any]) -> None:
        if not isinstance(data, dict):
            raise ValidationError("documents must be JSON-like objects")
        reserved = sorted(_RESERVED.intersection(data))
        if reserved:
            raise ValidationError(f"reserved fields are not allowed in payload: {', '.join(reserved)}")
        try:
            json.dumps(data, separators=(",", ":"), ensure_ascii=False)
        except (TypeError, ValueError) as exc:
            raise ValidationError(f"document contains unsupported values: {exc}") from exc

    def _encode(self, data: dict[str, Any]) -> bytes:
        self._validate_document(data)
        payload = cbor2.dumps(data, canonical=True)
        if len(payload) > self.max_document_bytes:
            raise ValidationError(
                f"document is {len(payload)} bytes; maximum is {self.max_document_bytes} bytes"
            )
        return payload

    @staticmethod
    def _decode(payload: bytes) -> dict[str, Any]:
        value = cbor2.loads(payload)
        if not isinstance(value, dict):
            raise VDBError("stored payload is not a document object")
        return value

    def create_collection(self, name: str) -> None:
        self._validate_collection(name)
        self._db.execute(
            "INSERT OR IGNORE INTO collections(name, created_at) VALUES (?, ?)",
            (name, self._now()),
        )

    def list_collections(self) -> list[str]:
        rows = self._db.execute("SELECT name FROM collections ORDER BY name").fetchall()
        return [row["name"] for row in rows]

    def _require_collection(self, collection: str) -> None:
        self._validate_collection(collection)
        row = self._db.execute("SELECT 1 FROM collections WHERE name = ?", (collection,)).fetchone()
        if row is None:
            raise NotFoundError(f"collection not found: {collection}")

    def put(
        self,
        collection: str,
        data: dict[str, Any],
        *,
        document_id: str | None = None,
        expected_version: int | None = None,
    ) -> Document:
        self._require_collection(collection)
        payload = self._encode(data)
        document_id = document_id or str(uuid.uuid4())
        if not document_id or len(document_id) > 256:
            raise ValidationError("document id must be a non-empty string of at most 256 characters")
        now = self._now()
        with self._db:
            row = self._db.execute(
                "SELECT version, created_at FROM documents WHERE collection = ? AND document_id = ?",
                (collection, document_id),
            ).fetchone()
            if row is None:
                if expected_version not in (None, 0):
                    raise VersionConflictError(collection, document_id, expected_version, 0)
                version = 1
                created_at = now
                self._db.execute(
                    "INSERT INTO documents(collection, document_id, version, created_at, updated_at, payload, payload_size) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?)",
                    (collection, document_id, version, created_at, now, payload, len(payload)),
                )
                event = "DOCUMENT_CREATED"
            else:
                current = int(row["version"])
                if expected_version is not None and expected_version != current:
                    raise VersionConflictError(collection, document_id, expected_version, current)
                version = current + 1
                created_at = row["created_at"]
                self._db.execute(
                    "UPDATE documents SET version = ?, updated_at = ?, payload = ?, payload_size = ? "
                    "WHERE collection = ? AND document_id = ?",
                    (version, now, payload, len(payload), collection, document_id),
                )
                event = "DOCUMENT_REPLACED"
            self._audit(event, collection, document_id, {"version": version})
        return Document(collection, document_id, version, created_at, now, data)

    def get(self, collection: str, document_id: str) -> Document:
        self._require_collection(collection)
        row = self._db.execute(
            "SELECT * FROM documents WHERE collection = ? AND document_id = ?",
            (collection, document_id),
        ).fetchone()
        if row is None:
            raise NotFoundError(f"document not found: {collection}/{document_id}")
        return Document(
            collection,
            row["document_id"],
            row["version"],
            row["created_at"],
            row["updated_at"],
            self._decode(row["payload"]),
        )

    def query(
        self,
        collection: str,
        *,
        where: dict[str, Any] | None = None,
        limit: int = 100,
        cursor_updated_at: str | None = None,
    ) -> list[Document]:
        self._require_collection(collection)
        if limit < 1 or limit > 1000:
            raise ValidationError("limit must be between 1 and 1000")
        where = where or {}
        self._validate_document(where)
        sql = "SELECT * FROM documents WHERE collection = ?"
        params: list[Any] = [collection]
        if cursor_updated_at:
            sql += " AND updated_at < ?"
            params.append(cursor_updated_at)
        sql += " ORDER BY updated_at DESC, document_id DESC LIMIT ?"
        params.append(limit)
        rows = self._db.execute(sql, params).fetchall()
        result: list[Document] = []
        for row in rows:
            data = self._decode(row["payload"])
            if all(data.get(key) == value for key, value in where.items()):
                result.append(
                    Document(
                        collection,
                        row["document_id"],
                        row["version"],
                        row["created_at"],
                        row["updated_at"],
                        data,
                    )
                )
        return result

    def delete(
        self,
        collection: str,
        document_id: str,
        *,
        expected_version: int | None = None,
    ) -> None:
        self._require_collection(collection)
        with self._db:
            row = self._db.execute(
                "SELECT version FROM documents WHERE collection = ? AND document_id = ?",
                (collection, document_id),
            ).fetchone()
            if row is None:
                raise NotFoundError(f"document not found: {collection}/{document_id}")
            current = int(row["version"])
            if expected_version is not None and expected_version != current:
                raise VersionConflictError(collection, document_id, expected_version, current)
            self._db.execute(
                "DELETE FROM documents WHERE collection = ? AND document_id = ?",
                (collection, document_id),
            )
            self._audit("DOCUMENT_DELETED", collection, document_id, {"version": current})

    def schema_report(self, collection: str, *, sample_limit: int = 100) -> dict[str, Any]:
        documents = self.query(collection, limit=min(max(sample_limit, 1), 1000))
        fields: dict[str, set[str]] = {}
        for document in documents:
            for key, value in document.data.items():
                fields.setdefault(key, set()).add(type(value).__name__)
        return {
            "collection": collection,
            "sampled_documents": len(documents),
            "fields": {key: sorted(types) for key, types in sorted(fields.items())},
        }

    def health(self) -> dict[str, Any]:
        counts = self._db.execute(
            "SELECT COUNT(*) AS documents, COALESCE(SUM(payload_size), 0) AS payload_bytes FROM documents"
        ).fetchone()
        collection_count = self._db.execute("SELECT COUNT(*) AS count FROM collections").fetchone()["count"]
        wal_path = Path(f"{self.path}-wal")
        return {
            "status": "healthy",
            "path": str(self.path),
            "collections": int(collection_count),
            "documents": int(counts["documents"]),
            "payload_bytes": int(counts["payload_bytes"]),
            "wal_bytes": wal_path.stat().st_size if wal_path.exists() else 0,
            "max_document_bytes": self.max_document_bytes,
        }

    def backup(self, destination: str | Path) -> dict[str, Any]:
        destination = Path(destination)
        destination.parent.mkdir(parents=True, exist_ok=True)
        self._db.execute("PRAGMA wal_checkpoint(PASSIVE)")
        with sqlite3.connect(destination) as target:
            self._db.backup(target)
        digest = hashlib.sha256(destination.read_bytes()).hexdigest()
        manifest = {
            "source": str(self.path),
            "destination": str(destination),
            "sha256": digest,
            "created_at": self._now(),
            "bytes": destination.stat().st_size,
        }
        manifest_path = destination.with_suffix(destination.suffix + ".manifest.json")
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        self._audit("BACKUP_CREATED", None, None, manifest)
        return manifest

    def _audit(
        self,
        event_type: str,
        collection: str | None,
        document_id: str | None,
        details: dict[str, Any],
    ) -> None:
        self._db.execute(
            "INSERT INTO audit_events(created_at, event_type, collection, document_id, details) VALUES (?, ?, ?, ?, ?)",
            (self._now(), event_type, collection, document_id, json.dumps(details, sort_keys=True)),
        )

    def audit_events(self, *, limit: int = 100) -> list[dict[str, Any]]:
        if limit < 1 or limit > 1000:
            raise ValidationError("limit must be between 1 and 1000")
        rows = self._db.execute(
            "SELECT * FROM audit_events ORDER BY id DESC LIMIT ?", (limit,)
        ).fetchall()
        return [
            {
                "id": row["id"],
                "created_at": row["created_at"],
                "event_type": row["event_type"],
                "collection": row["collection"],
                "document_id": row["document_id"],
                "details": json.loads(row["details"]),
            }
            for row in rows
        ]
