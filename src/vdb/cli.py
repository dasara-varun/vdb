from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from .steward import Steward
from .store import VDBError, VDBStore


def _json(value: Any) -> None:
    print(json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="vdb", description="VDB local-first document database")
    parser.add_argument("--path", default="vdb.sqlite", help="database file path")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("init", help="initialize a database")
    collections = sub.add_parser("collections", help="manage collections")
    collections.add_argument("action", choices=["list", "create"])
    collections.add_argument("name", nargs="?")

    put = sub.add_parser("put", help="insert or replace a document")
    put.add_argument("collection")
    put.add_argument("document_id")
    put.add_argument("document", help="JSON object")
    put.add_argument("--expected-version", type=int)

    get = sub.add_parser("get", help="retrieve a document")
    get.add_argument("collection")
    get.add_argument("document_id")

    query = sub.add_parser("query", help="query documents")
    query.add_argument("collection")
    query.add_argument("--where", default="{}", help="JSON object for equality filters")
    query.add_argument("--limit", type=int, default=100)

    delete = sub.add_parser("delete", help="delete one document")
    delete.add_argument("collection")
    delete.add_argument("document_id")
    delete.add_argument("--expected-version", type=int)

    schema = sub.add_parser("schema", help="inspect observed collection field types")
    schema.add_argument("collection")

    sub.add_parser("health", help="show health metrics")
    steward = sub.add_parser("steward", help="show read-only Steward findings")
    steward.add_argument("--collection")

    backup = sub.add_parser("backup", help="create a snapshot")
    backup.add_argument("destination")
    return parser


def run(args: argparse.Namespace) -> int:
    if args.command == "init":
        with VDBStore(args.path):
            pass
        _json({"status": "initialized", "path": str(Path(args.path))})
        return 0

    with VDBStore(args.path) as store:
        if args.command == "collections":
            if args.action == "list":
                _json(store.list_collections())
            else:
                if not args.name:
                    raise VDBError("collection name is required")
                store.create_collection(args.name)
                _json({"created": args.name})
        elif args.command == "put":
            document = json.loads(args.document)
            _json(
                store.put(
                    args.collection,
                    document,
                    document_id=args.document_id,
                    expected_version=args.expected_version,
                ).to_dict()
            )
        elif args.command == "get":
            _json(store.get(args.collection, args.document_id).to_dict())
        elif args.command == "query":
            documents = store.query(args.collection, where=json.loads(args.where), limit=args.limit)
            _json([document.to_dict() for document in documents])
        elif args.command == "delete":
            store.delete(args.collection, args.document_id, expected_version=args.expected_version)
            _json({"deleted": f"{args.collection}/{args.document_id}"})
        elif args.command == "schema":
            _json(store.schema_report(args.collection))
        elif args.command == "health":
            _json(store.health())
        elif args.command == "steward":
            _json(Steward(store).explain(args.collection))
        elif args.command == "backup":
            _json(store.backup(args.destination))
    return 0


def main() -> None:
    try:
        raise SystemExit(run(build_parser().parse_args()))
    except (VDBError, json.JSONDecodeError, OSError) as exc:
        print(f"vdb: error: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc


if __name__ == "__main__":
    main()
