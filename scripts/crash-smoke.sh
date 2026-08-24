#!/usr/bin/env bash
set -euo pipefail

binary=${1:?usage: crash-smoke.sh PATH_TO_VDB_BINARY}
if [[ ! -x "$binary" ]]; then
  printf 'binary is not executable: %s\n' "$binary" >&2
  exit 1
fi

case "${OSTYPE:-}" in
  linux*|darwin*) ;;
  *)
    printf 'process-crash smoke is only supported on Unix-like hosts\n'
    exit 0
    ;;
esac

temporary_directory=$(mktemp -d)
database="$temporary_directory/crash-smoke.vdb"
document=$(printf '{"payload":"%0100000d"}' 0)
cleanup() {
  local process_id
  for process_id in $(jobs -pr); do
    kill -KILL "$process_id" 2>/dev/null || true
  done
  rm -rf "$temporary_directory"
}
trap cleanup EXIT

"$binary" --path "$database" init >/dev/null
"$binary" --path "$database" collections create users >/dev/null

for index in $(seq 1 64); do
  "$binary" --path "$database" put users "u-$index" "$document" >/dev/null 2>"$temporary_directory/put-$index.err" &
  process_id=$!
  sleep 0.005
  if kill -0 "$process_id" 2>/dev/null; then
    kill -KILL "$process_id" 2>/dev/null || true
  fi
  wait "$process_id" 2>/dev/null || true
  "$binary" --path "$database" health >/dev/null
 done

printf 'process-crash smoke passed for 64 interrupted CLI writes\n'
