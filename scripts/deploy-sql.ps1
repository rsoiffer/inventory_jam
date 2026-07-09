$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$AdminUrlPath = Join-Path $RepoRoot "secrets\database-admin-url.txt"
$SqlPath = Join-Path $RepoRoot "sql\server_discovery.sql"

function Get-DatabaseAdminUrl {
    if (Test-Path $AdminUrlPath) {
        foreach ($line in Get-Content $AdminUrlPath) {
            $line = $line.Trim()
            if ($line -eq "" -or $line.StartsWith("#")) { continue }
            return $line
        }
    }
    if ($env:DATABASE_ADMIN_URL) {
        return $env:DATABASE_ADMIN_URL
    }
    throw "Missing URL in secrets/database-admin-url.txt or DATABASE_ADMIN_URL (for CI)"
}

$psql = Get-Command psql -ErrorAction SilentlyContinue
if (-not $psql) {
    throw "psql was not found on PATH; install PostgreSQL client tools first"
}

if (-not (Test-Path $SqlPath)) {
    throw "SQL file not found: $SqlPath"
}

$databaseUrl = Get-DatabaseAdminUrl

Write-Host "Applying $SqlPath with doadmin credentials"
& $psql.Source $databaseUrl -v ON_ERROR_STOP=1 -f $SqlPath
Write-Host "SQL deploy complete"
