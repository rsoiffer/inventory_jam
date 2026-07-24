$ErrorActionPreference = "Stop"

$EdgegapScript = Join-Path $PSScriptRoot "deploy-edgegap.ps1"
$ItchScript = Join-Path $PSScriptRoot "deploy-itch.ps1"

Write-Host "Deploying Edgegap..."
& $EdgegapScript
if ($LASTEXITCODE -ne 0) {
    throw "deploy-edgegap.ps1 failed with exit code $LASTEXITCODE"
}

Write-Host "Deploying itch..."
& $ItchScript
if ($LASTEXITCODE -ne 0) {
    throw "deploy-itch.ps1 failed with exit code $LASTEXITCODE"
}

Write-Host "Edgegap and itch deploys completed."
