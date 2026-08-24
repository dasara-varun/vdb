#!/usr/bin/env bash
set -euo pipefail

required_files=(
  "AGENTS.md"
  "CONTRIBUTING.md"
  "SECURITY.md"
  "README.md"
  ".github/pull_request_template.md"
  "docs/README.md"
  "docs/agent-tooling.md"
  "docs/loop-engineering.md"
  "docs/installation.md"
  "docs/documentation-maintenance.md"
  "docs/durability-matrix.md"
  ".github/dependabot.yml"
  "docs/project-status.md"
  "docs/decisions/README.md"
  "docs/decisions/0001-local-first-read-only-steward.md"
  "docs/templates/decision-record.md"
  "docs/templates/progress-update.md"
  "docs/templates/research-note.md"
  "docs/data-format.md"
  "docs/operations.md"
  "docs/security.md"
  "docs/testing.md"
  "docs/roadmap.md"
  "docs/production-gap-audit.md"
)

for path in "${required_files[@]}"; do
  if [[ ! -f "$path" ]]; then
    printf 'Missing required documentation file: %s\n' "$path" >&2
    exit 1
  fi
done

for workflow in .github/workflows/ci.yml .github/workflows/release.yml; do
  if [[ ! -f "$workflow" ]]; then
    printf 'Missing required workflow file: %s\n' "$workflow" >&2
    exit 1
  fi
done

for asset in diagrams/architecture.mmd diagrams/architecture.png diagrams/recovery-flow.mmd diagrams/recovery-flow.png; do
  if [[ ! -f "$asset" ]]; then
    printf 'Missing required diagram asset: %s\n' "$asset" >&2
    exit 1
  fi
done

if ! grep -Fq 'docs/documentation-maintenance.md' README.md; then
  printf 'README.md does not link to the documentation maintenance guide\n' >&2
  exit 1
fi

if ! grep -Fq 'docs/project-status.md' docs/README.md; then
  printf 'docs/README.md does not link to the project status snapshot\n' >&2
  exit 1
fi

if [[ -f Cargo.lock ]]; then
  printf 'Cargo.lock exists; verify it was generated from the intended crates.io registry before committing\n' >&2
  exit 1
fi

printf 'Documentation consistency check passed (%d required files, 2 workflows, and 4 diagram assets).\n' "${#required_files[@]}"
