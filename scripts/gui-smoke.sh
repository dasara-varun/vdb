#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/debug/vdb}"
if [[ ! -x "$binary" ]]; then
  printf 'GUI smoke binary is not executable: %s\n' "$binary" >&2
  exit 1
fi

workdir="$(mktemp -d)"
server_pid=""
cleanup() {
  if [[ -n "$server_pid" ]] && kill -0 "$server_pid" 2>/dev/null; then
    kill -INT "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$workdir"
}
trap cleanup EXIT

database="$workdir/smoke.vdb"
log="$workdir/gui.log"

"$binary" --path "$database" init >/dev/null
"$binary" --path "$database" collections create users >/dev/null
"$binary" --path "$database" put users u1 '{"name":"Ada","html":"<safe>"}' >/dev/null
before="$($binary --path "$database" get users u1)"

"$binary" --path "$database" gui --port 0 >"$log" 2>&1 &
server_pid=$!
for _ in {1..50}; do
  if grep -q 'http://127\.0\.0\.1:[0-9][0-9]*' "$log"; then
    break
  fi
  sleep 0.1
done
url="$(sed -n 's#.*\(http://127\.0\.0\.1:[0-9][0-9]*\).*#\1#p' "$log" | head -n 1)"
if [[ -z "$url" ]]; then
  printf 'GUI did not print a loopback URL\n' >&2
  cat "$log" >&2
  exit 1
fi

curl --fail --silent --show-error -D "$workdir/root.headers" "$url/" -o "$workdir/root.html"
curl --fail --silent --show-error "$url/api/health" -o "$workdir/health.json"
curl --fail --silent --show-error "$url/api/collections" -o "$workdir/collections.json"
curl --fail --silent --show-error "$url/api/documents?collection=users&limit=1" -o "$workdir/documents.json"
curl --fail --silent --show-error "$url/api/documents?collection=users&where=%7B%22name%22%3A%22Ada%22%7D" -o "$workdir/filtered.json"
curl --fail --silent --show-error "$url/collection?collection=users" -o "$workdir/collection.html"

for header in \
  'Content-Security-Policy: default-src' \
  'X-Content-Type-Options: nosniff' \
  'Cache-Control: no-store' \
  'Referrer-Policy: no-referrer'; do
  grep -F "$header" "$workdir/root.headers" >/dev/null
done
grep -F 'Read-only mode' "$workdir/root.html" >/dev/null
grep -F '&lt;safe&gt;' "$workdir/collection.html" >/dev/null
! grep -F '<safe>' "$workdir/collection.html" >/dev/null
grep -F '"users"' "$workdir/collections.json" >/dev/null
grep -F '"name":"Ada"' "$workdir/documents.json" >/dev/null
grep -F '"name":"Ada"' "$workdir/filtered.json" >/dev/null

post_status="$(curl --silent --show-error -o "$workdir/post.json" -w '%{http_code}' -X POST "$url/")"
[[ "$post_status" == "405" ]]
grep -F 'read-only' "$workdir/post.json" >/dev/null
! grep -F "$database" "$workdir/post.json" >/dev/null

missing_status="$(curl --silent --show-error -o "$workdir/missing.json" -w '%{http_code}' "$url/collection?collection=missing")"
[[ "$missing_status" == "404" ]]
grep -F 'collection not found' "$workdir/missing.json" >/dev/null
! grep -F "$database" "$workdir/missing.json" >/dev/null

limit_status="$(curl --silent --show-error -o "$workdir/limit.json" -w '%{http_code}' "$url/api/documents?collection=users&limit=101")"
[[ "$limit_status" == "400" ]]
grep -F 'limit must be between 1 and 100' "$workdir/limit.json" >/dev/null

encoding_status="$(curl --silent --show-error -o "$workdir/encoding.json" -w '%{http_code}' "$url/api/documents?collection=%ZZ")"
[[ "$encoding_status" == "400" ]]
route_status="$(curl --silent --show-error -o "$workdir/route.json" -w '%{http_code}' "$url/not-a-route")"
[[ "$route_status" == "404" ]]
oversized="$(head -c 17000 </dev/zero | tr '\0' x)"
header_status="$(curl --silent --show-error -o "$workdir/oversized.json" -w '%{http_code}' -H "X-GUI-Fill: $oversized" "$url/")"
[[ "$header_status" == "413" ]]

kill -TERM "$server_pid" 2>/dev/null || true
wait "$server_pid" 2>/dev/null || true
server_pid=""
after="$($binary --path "$database" get users u1)"
[[ "$before" == "$after" ]]
printf 'GUI smoke passed at %s\n' "$url"
