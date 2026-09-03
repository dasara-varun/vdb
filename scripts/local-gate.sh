#!/usr/bin/env bash
# local-gate.sh - VDB Local Validation Gate (Zero CI Cost)
# Enforces complete repository quality without external cloud/CI services.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
cd "$repo_root"

log_stage() {
  printf '\n=== [GATE] %s ===\n' "$1"
}

log_stage "1/5: Formatting Check (cargo fmt)"
cargo fmt --all -- --check

log_stage "2/5: Strict Clippy Linting (warnings denied)"
cargo clippy --workspace --all-targets -- -D warnings

log_stage "3/5: Workspace Tests (core + CLI)"
cargo test --workspace

log_stage "4/5: Release Binary Build"
cargo build --workspace --release

log_stage "5/5: End-to-End Local CLI Smoke Tests"
vdb_bin="$repo_root/target/release/vdb"
if [[ -f "${vdb_bin}.exe" ]]; then
  vdb_bin="${vdb_bin}.exe"
fi

if [[ ! -x "$vdb_bin" && ! -f "$vdb_bin" ]]; then
  printf 'Release binary not found at %s\n' "$vdb_bin" >&2
  exit 1
fi

smoke_dir="$(mktemp -d 2>/dev/null || mktemp -d -t 'vdb-smoke')"
trap 'rm -rf "$smoke_dir"' EXIT

db_path="$smoke_dir/test.vdb"
backup_dir="$smoke_dir/backups"
restored_db="$smoke_dir/restored.vdb"

printf '  -> Initializing database...\n'
"$vdb_bin" init --path "$db_path"

printf '  -> Creating collection "users"...\n'
"$vdb_bin" collection create --path "$db_path" --name users

printf '  -> Putting document...\n'
"$vdb_bin" doc put --path "$db_path" --collection users --id u1 --data '{"name":"Alice","role":"admin","level":10}'

printf '  -> Getting document...\n'
doc_out="$("$vdb_bin" doc get --path "$db_path" --collection users --id u1)"
if ! grep -q 'Alice' <<< "$doc_out"; then
  printf 'Document retrieval did not contain expected payload\n' >&2
  exit 1
fi

printf '  -> Creating equality index...\n'
"$vdb_bin" index create --path "$db_path" --collection users --field role

printf '  -> Querying indexed document...\n'
q_out="$("$vdb_bin" query --path "$db_path" --collection users --field role --value admin)"
if ! grep -q 'Alice' <<< "$q_out"; then
  printf 'Query did not return expected record\n' >&2
  exit 1
fi

printf '  -> Checking database health...\n'
"$vdb_bin" health --path "$db_path"

printf '  -> Running Steward audit...\n'
"$vdb_bin" steward audit --path "$db_path"

printf '  -> Creating backup...\n'
"$vdb_bin" backup --path "$db_path" --destination "$backup_dir"

printf '  -> Verifying backup...\n'
"$vdb_bin" backup verify --destination "$backup_dir"

printf '  -> Restoring backup to new path...\n'
"$vdb_bin" restore --source-backup-dir "$backup_dir" --destination-db "$restored_db"

printf '  -> Compacting database...\n'
"$vdb_bin" compact --path "$db_path"

printf '\n>>> [GATE PASSED] All checks completed successfully! <<<\n\n'
