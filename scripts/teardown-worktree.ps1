# teardown-worktree.ps1 - Safely removes an agent worktree and cleans its target cache
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$AgentName,

    [switch]$KeepBranch = $false,
    [switch]$KeepTarget = $false
)

$ErrorActionPreference = "Stop"

$repoRoot = (git rev-parse --show-toplevel 2>$null)
if (-not $repoRoot) {
    Write-Error "Must run inside a Git repository."
    exit 1
}
cd $repoRoot

$worktreesBase = Join-Path (Split-Path $repoRoot -Parent) "vdb-worktrees"
$worktreeDir = Join-Path $worktreesBase "vdb-agent-$AgentName"
$targetDir = Join-Path $repoRoot "target-$AgentName"


Write-Host "=== Tearing Down Worktree for Agent '$AgentName' ===" -ForegroundColor Cyan

if (Test-Path $worktreeDir) {
    Write-Host "  -> Removing git worktree at $worktreeDir..." -ForegroundColor Gray
    git worktree remove --force $worktreeDir
} else {
    Write-Host "  Worktree directory not found; pruning worktree state..." -ForegroundColor Gray
    git worktree prune
}

if (-not $KeepTarget -and (Test-Path $targetDir)) {
    Write-Host "  -> Removing target directory cache at $targetDir..." -ForegroundColor Gray
    Remove-Item -Recurse -Force $targetDir -ErrorAction SilentlyContinue
}

Write-Host "`n>>> Worktree '$AgentName' Teardown Complete! <<<" -ForegroundColor Green
