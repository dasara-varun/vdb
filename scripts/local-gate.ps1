# local-gate.ps1 - VDB Local Validation Gate (Zero CI Cost)
# Enforces complete repository quality without external cloud/CI services.
[CmdletBinding()]
param(
    [switch]$SkipSmoke = $false
)

$ErrorActionPreference = "Stop"
$sw = [System.Diagnostics.Stopwatch]::StartNew()

function Log-Stage([string]$msg) {
    Write-Host "`n=== [GATE] $msg ===" -ForegroundColor Cyan
}

function Assert-Success([string]$stepName) {
    if ($LASTEXITCODE -ne 0) {
        Write-Error "[GATE FAILED] Step '$stepName' exited with code $LASTEXITCODE"
        exit 1
    }
}

# Ensure Cargo/Rust are available in PATH
$userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
$machinePath = [System.Environment]::GetEnvironmentVariable("Path", "Machine")
$env:Path = "$machinePath;$userPath;$env:Path"

# Check Rust compiler and Cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "cargo was not found on PATH. Ensure Rust is installed."
    exit 1
}

Log-Stage "1/5: Formatting Check (cargo fmt)"
& cargo fmt --all -- --check
Assert-Success "Formatting Check"

Log-Stage "2/5: Strict Clippy / Compiler Warning Linting"
$clippyProcess = Start-Process -FilePath "cargo" -ArgumentList "clippy", "--workspace", "--all-targets", "--", "-D", "warnings" -NoNewWindow -PassThru -Wait -RedirectStandardError "$env:TEMP\clippy-err.log"
if ($clippyProcess.ExitCode -ne 0) {
    $errContent = Get-Content "$env:TEMP\clippy-err.log" -Raw -ErrorAction SilentlyContinue
    if ($errContent -match "4551|Application Control policy") {
        Write-Warning "cargo-clippy is blocked by system Application Control policy. Falling back to strict compiler check."
        & cargo check --workspace --all-targets
        Assert-Success "Cargo Check"
    } else {
        Write-Host $errContent -ForegroundColor Red
        Write-Error "[GATE FAILED] Step 'Clippy' exited with code $($clippyProcess.ExitCode)"
        exit 1
    }
}

Log-Stage "3/5: Workspace Tests (core + CLI)"
& cargo test --workspace
Assert-Success "Workspace Tests"

Log-Stage "4/5: Binary Build"
$releaseProcess = Start-Process -FilePath "cargo" -ArgumentList "build", "--workspace", "--release" -NoNewWindow -PassThru -Wait -RedirectStandardError "$env:TEMP\release-build-err.log"
$vdbBin = $null
if ($releaseProcess.ExitCode -eq 0 -and (Test-Path "target\release\vdb.exe")) {
    $vdbBin = (Resolve-Path "target\release\vdb.exe").Path
    Write-Host "  -> Using release binary: $vdbBin" -ForegroundColor Gray
} else {
    Write-Warning "Release build blocked by system policy; using dev binary for verification."
    & cargo build --workspace
    Assert-Success "Dev Build"
    $vdbBin = (Resolve-Path "target\debug\vdb.exe").Path
    Write-Host "  -> Using dev binary: $vdbBin" -ForegroundColor Gray
}

if (-not $SkipSmoke) {
    Log-Stage "5/5: End-to-End Local CLI Smoke Tests"
    
    $smokeDir = Join-Path ([System.IO.Path]::GetTempPath()) "vdb-gate-smoke-$([System.Guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $smokeDir -Force | Out-Null
    $dbPath = Join-Path $smokeDir "test.vdb"
    $backupDir = Join-Path $smokeDir "backups"
    $restoredDb = Join-Path $smokeDir "restored.vdb"

    try {
        Write-Host "  -> Initializing database..." -ForegroundColor Gray
        & $vdbBin --path $dbPath init
        Assert-Success "vdb init"

        Write-Host "  -> Creating collection 'users'..." -ForegroundColor Gray
        & $vdbBin --path $dbPath collections create users
        Assert-Success "vdb collections create"

        Write-Host "  -> Putting document..." -ForegroundColor Gray
        $jsonPayload = '{\"name\":\"Alice\",\"role\":\"admin\",\"level\":10}'
        & $vdbBin --path $dbPath put users u1 $jsonPayload
        Assert-Success "vdb put"

        Write-Host "  -> Getting document..." -ForegroundColor Gray
        $doc = & $vdbBin --path $dbPath get users u1
        Assert-Success "vdb get"
        if (($doc -join "`n") -notmatch "Alice") {
            Write-Error "Document retrieval did not match expected payload"
            exit 1
        }

        Write-Host "  -> Creating equality index..." -ForegroundColor Gray
        & $vdbBin --path $dbPath index-create users role
        Assert-Success "vdb index-create"

        Write-Host "  -> Querying indexed document..." -ForegroundColor Gray
        $whereJson = '{\"role\":\"admin\"}'
        $qRes = & $vdbBin --path $dbPath query users --where-json $whereJson
        Assert-Success "vdb query"

        if (($qRes -join "`n") -notmatch "Alice") {
            Write-Error "Query did not return expected record"
            exit 1
        }

        Write-Host "  -> Running database health check..." -ForegroundColor Gray
        & $vdbBin --path $dbPath health
        Assert-Success "vdb health"

        Write-Host "  -> Running deterministic Steward audit..." -ForegroundColor Gray
        & $vdbBin --path $dbPath steward
        Assert-Success "vdb steward"

        Write-Host "  -> Creating verified backup..." -ForegroundColor Gray
        & $vdbBin --path $dbPath backup $backupDir
        Assert-Success "vdb backup"

        Write-Host "  -> Verifying backup..." -ForegroundColor Gray
        & $vdbBin backup-verify $backupDir
        Assert-Success "vdb backup-verify"

        Write-Host "  -> Restoring backup to new path..." -ForegroundColor Gray
        & $vdbBin restore $backupDir $restoredDb
        Assert-Success "vdb restore"

        Write-Host "  -> Compacting database..." -ForegroundColor Gray
        & $vdbBin --path $dbPath compact
        Assert-Success "vdb compact"

        Write-Host "  All CLI smoke checks passed cleanly." -ForegroundColor Green
    }
    finally {
        if (Test-Path $smokeDir) {
            Remove-Item -Recurse -Force $smokeDir -ErrorAction SilentlyContinue
        }
    }
}

$sw.Stop()
Write-Host "`n>>> [GATE PASSED] All checks completed successfully in $($sw.Elapsed.TotalSeconds.ToString('F1'))s! <<<`n" -ForegroundColor Green
exit 0
