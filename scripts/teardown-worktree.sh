#!/usr/bin/env bash
# teardown-worktree.sh - Safely removes an agent worktree and cleans its target cache
set -euo pipefail

if [[ $# -lt 1 ]]; then
  printf 'Usage: %s <agent-name>\n' "$0" >&2
  exit 1
fi

agent_name="$1"
repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

parent_dir="$(cd "$repo_root/.." && pwd)"
worktree_dir="$parent_dir/vdb-worktrees/vdb-agent-${agent_name}"
target_dir="$repo_root/target-${agent_name}"


printf '=== Tearing Down Worktree for Agent "%s" ===\n' "$agent_name"

if [[ -d "$worktree_dir" ]]; then
  printf '  -> Removing git worktree at %s...\n' "$worktree_dir"
  git worktree remove --force "$worktree_dir"
else
  git worktree prune
fi

if [[ -d "$target_dir" ]]; then
  printf '  -> Removing target directory at %s...\n' "$target_dir"
  rm -rf "$target_dir"
fi

printf '>>> Worktree "%s" Teardown Complete! <<<\n' "$agent_name"
