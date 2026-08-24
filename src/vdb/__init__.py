"""VDB: a simple, local-first document database."""

from .steward import Finding, Steward
from .store import (
    Document,
    NotFoundError,
    VDBError,
    VDBStore,
    ValidationError,
    VersionConflictError,
)

__all__ = [
    "Document",
    "Finding",
    "NotFoundError",
    "Steward",
    "VDBError",
    "VDBStore",
    "ValidationError",
    "VersionConflictError",
]

__version__ = "0.1.0"
