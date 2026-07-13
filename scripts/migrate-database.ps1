param(
    [switch]$ForceReset
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$AdminUrlPath = Join-Path $RepoRoot "secrets/database-admin-url.txt"
$MigrationPath = Join-Path $RepoRoot "migrations/persistence.sql"

if (-not (Test-Path $AdminUrlPath)) {
    throw "Missing admin database URL at $AdminUrlPath"
}
if (-not (Test-Path $MigrationPath)) {
    throw "Missing migration file at $MigrationPath"
}
if ($null -eq (Get-Command psql -ErrorAction SilentlyContinue)) {
    throw "psql was not found. Install PostgreSQL client tools before migrating."
}

$AdminUrl = (Get-Content $AdminUrlPath -Raw).Trim()
if ([string]::IsNullOrWhiteSpace($AdminUrl)) {
    throw "Admin database URL in $AdminUrlPath must not be empty"
}

function Invoke-Psql {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Url,
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    & psql $Url @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "psql $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

if ($ForceReset) {
    Write-Host "Force-resetting persistence schema..."
    $resetSql = @"
DROP TABLE IF EXISTS _sqlx_migrations CASCADE;
DROP TABLE IF EXISTS player_transaction CASCADE;
DROP TABLE IF EXISTS player_state CASCADE;
DROP FUNCTION IF EXISTS flush_player_transactions(JSONB, TEXT[]);
DROP FUNCTION IF EXISTS get_player_states(TEXT[]);
DROP FUNCTION IF EXISTS apply_player_change(TEXT, BIGINT);
DROP FUNCTION IF EXISTS apply_persistence_transaction(UUID, JSONB);
DROP FUNCTION IF EXISTS ensure_player(TEXT);
DROP FUNCTION IF EXISTS player_state_hash(TEXT, JSONB);
"@
    Invoke-Psql -Url $AdminUrl -Arguments @("-v", "ON_ERROR_STOP=1", "-c", $resetSql)
}

Write-Host "Applying $($MigrationPath)..."
Invoke-Psql -Url $AdminUrl -Arguments @("-v", "ON_ERROR_STOP=1", "-f", $MigrationPath)
Write-Host "Migration complete."
