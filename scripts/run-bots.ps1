param(
    [int]$Count = 5,
    [double]$DelaySeconds = 2
)

$ErrorActionPreference = "Stop"

if ($Count -lt 1) {
    throw "-Count must be at least 1"
}
if ($DelaySeconds -lt 0) {
    throw "-DelaySeconds must be non-negative"
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

# Same binary crate name as the server / `cargo run` client (Lightyear protocol
# hashes use type_name). Separate --target-dir so a running GUI client does not
# lock the bot output exe.
$Features = "client,netcode,webtransport"
$TargetDir = Join-Path $RepoRoot "target\bots"
$DistDir = Join-Path $RepoRoot "dist\bots"
$IsWindows = $env:OS -eq "Windows_NT"
$DistName = if ($IsWindows) { "inventory_jam.exe" } else { "inventory_jam" }
$BuiltExe = Join-Path $TargetDir "debug\$DistName"
$Exe = Join-Path $DistDir $DistName

Write-Host "Building headless client ($Features) into $TargetDir..."
cargo build --no-default-features --features $Features --bin inventory_jam --target-dir $TargetDir
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}
if (-not (Test-Path $BuiltExe)) {
    throw "Expected bot binary missing: $BuiltExe"
}

New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
Copy-Item -Path $BuiltExe -Destination $Exe -Force
Write-Host "Copied bot binary to $Exe"

$Processes = @()

try {
    for ($i = 1; $i -le $Count; $i++) {
        $User = "Bot$i"

        Write-Host "Starting bot $i/$Count as '$User'..."
        # UseShellExecute=$false inherits this console (no new window; shared stdout/stderr).
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $Exe
        $psi.Arguments = "--mode edgegap --user `"$User`""
        $psi.WorkingDirectory = $RepoRoot
        $psi.UseShellExecute = $false
        $proc = [System.Diagnostics.Process]::Start($psi)
        if ($null -eq $proc) {
            throw "Failed to start bot '$User'"
        }
        $Processes += $proc
        Write-Host "  PID $($proc.Id)"

        if ($i -lt $Count -and $DelaySeconds -gt 0) {
            Start-Sleep -Seconds $DelaySeconds
        }
    }

    Write-Host "All $Count bots running. Press Ctrl+C to stop."
    Wait-Process -Id ($Processes | ForEach-Object { $_.Id })
}
finally {
    foreach ($proc in $Processes) {
        if (-not $proc.HasExited) {
            Write-Host "Stopping PID $($proc.Id)..."
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        }
    }
}
