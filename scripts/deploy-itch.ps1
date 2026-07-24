param(
    [switch]$SkipBuild,
    [string]$Version,
    [string]$Channel = "web",
    [string]$Project = "dragonaxegaming/inventory-jam-rust-mmo"
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$DistDir = Join-Path $RepoRoot "dist"
$Target = "${Project}:${Channel}"

function Invoke-NativeChecked {
    param(
        [string]$Command,
        [string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

function Assert-ButlerAvailable {
    if ($null -eq (Get-Command butler -ErrorAction SilentlyContinue)) {
        throw "butler was not found on PATH. Install it from https://itchio.itch.io/butler and run 'butler login' first."
    }
}

Push-Location $RepoRoot
try {
    Assert-ButlerAvailable

    if (-not $SkipBuild) {
        if ($null -eq (Get-Command trunk -ErrorAction SilentlyContinue)) {
            throw "trunk was not found on PATH. Install trunk before building the web release."
        }
        Write-Host "Building web release with trunk..."
        Invoke-NativeChecked -Command "trunk" -Arguments @("build", "--release")
    }

    if (-not (Test-Path $DistDir)) {
        throw "dist directory was not found at $DistDir. Build the web release first or omit -SkipBuild."
    }

    $ButlerArgs = @("push", $DistDir, $Target)
    if (-not [string]::IsNullOrWhiteSpace($Version)) {
        $ButlerArgs += @("--userversion", $Version)
    }

    Write-Host "Uploading dist/ to $Target with butler..."
    Invoke-NativeChecked -Command "butler" -Arguments $ButlerArgs
    Write-Host "Deployed dist/ to $Target"
}
finally {
    Pop-Location
}
