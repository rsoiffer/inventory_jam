param(
    [switch]$SkipBuild,
    [switch]$SkipPush,
    [switch]$SkipVersionUpdate,
    [switch]$SkipDeploy,
    [string]$ImageTag
)

$ErrorActionPreference = "Stop"

$AppName = "inventory-jam"
$AppVersion = "dev"
$Registry = "registry.edgegap.com/loco-motion-devs-qfpbnhdj6oju"
$ImageName = "inventory-jam-server"
$Latitude = 45.5231
$Longitude = -122.6765
$GamePortName = "gameport"
$GamePort = 5888
$ReqCpu = 1024
$ReqMemory = 2048
$DeployReadyTimeoutSec = 600
$DeployReadyPollSec = 5
$ApiToken = "token 6d12964d-c588-4e26-9810-fe5ad00b33af"
$RegistryUsername = 'robot$loco-motion-devs-qfpbnhdj6oju+client-push'
$RegistryToken = "hCHsdXULbVJQrT71TdTvGPPJNAvxc1Yv"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

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

function Assert-DockerAvailable {
    if ($null -eq (Get-Command docker -ErrorAction SilentlyContinue)) {
        throw "Docker CLI was not found. Start Docker Desktop or install Docker before deploying."
    }
    Invoke-NativeChecked -Command "docker" -Arguments @("info")
}

function Ensure-RegistryLogin {
    $RegistryToken | docker login registry.edgegap.com -u $RegistryUsername --password-stdin | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "docker login failed with exit code $LASTEXITCODE"
    }
}

function Invoke-EdgegapApi {
    param(
        [string]$Method,
        [string]$Uri,
        [object]$Body
    )

    $params = @{
        Method = $Method
        Uri = $Uri
        Headers = @{ Authorization = $ApiToken }
    }

    if ($null -ne $Body) {
        $params.ContentType = "application/json"
        $params.Body = ($Body | ConvertTo-Json -Depth 16)
    }

    return Invoke-RestMethod @params
}

function Get-AppVersion {
    param(
        [string]$AppName,
        [string]$AppVersion
    )

    try {
        return Invoke-EdgegapApi `
            -Method Get `
            -Uri "https://api.edgegap.com/v1/app/$AppName/version/$AppVersion"
    } catch {
        $statusCode = $_.Exception.Response.StatusCode.value__
        if ($statusCode -eq 404) {
            return $null
        }
        throw
    }
}

function Get-VersionImageBody {
    return @{
        docker_repository = $Registry.TrimEnd("/")
        docker_image = $ImageName.TrimStart("/")
        docker_tag = $ImageTag
        private_username = $RegistryUsername
        private_token = $RegistryToken
        verify_image = $true
        is_active = $true
    }
}

function New-AppVersion {
    param(
        [string]$AppName,
        [string]$AppVersion
    )

    $body = Get-VersionImageBody
    $body.name = $AppVersion
    $body.req_cpu = $ReqCpu
    $body.req_memory = $ReqMemory
    $body.restart_policy = "never"
    $body.ports = @(
        @{
            port = $GamePort
            protocol = "UDP"
            to_check = $false
            name = $GamePortName
        }
    )

    Write-Host "Creating Edgegap app version $AppName@$AppVersion"
    return Invoke-EdgegapApi `
        -Method Post `
        -Uri "https://api.edgegap.com/v1/app/$AppName/version" `
        -Body $body
}

function Update-AppVersion {
    param(
        [string]$AppName,
        [string]$AppVersion
    )

    Write-Host "Updating existing Edgegap app version $AppName@$AppVersion"
    return Invoke-EdgegapApi `
        -Method Patch `
        -Uri "https://api.edgegap.com/v1/app/$AppName/version/$AppVersion" `
        -Body (Get-VersionImageBody)
}

function Ensure-AppVersion {
    param(
        [string]$AppName,
        [string]$AppVersion
    )

    $existing = Get-AppVersion -AppName $AppName -AppVersion $AppVersion
    if ($null -eq $existing) {
        return New-AppVersion -AppName $AppName -AppVersion $AppVersion
    }
    return Update-AppVersion -AppName $AppName -AppVersion $AppVersion
}

function Stop-ActiveDeployments {
    $response = Invoke-EdgegapApi `
        -Method Get `
        -Uri "https://api.edgegap.com/v1/deployments?limit=50"

    $deployments = @($response.data)
    if ($deployments.Count -eq 0) {
        Write-Host "No active deployments to stop"
        return
    }

    foreach ($deployment in $deployments) {
        $requestId = $deployment.request_id
        Write-Host "Stopping deployment $requestId"
        Invoke-EdgegapApi `
            -Method Delete `
            -Uri "https://api.edgegap.com/v1/stop/$requestId" | Out-Null
    }

    $deadline = (Get-Date).AddSeconds($DeployReadyTimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $response = Invoke-EdgegapApi `
            -Method Get `
            -Uri "https://api.edgegap.com/v1/deployments?limit=50"
        if (@($response.data).Count -eq 0) {
            Write-Host "Previous deployments stopped"
            return
        }
        Write-Host "Waiting for previous deployment(s) to stop..."
        Start-Sleep -Seconds $DeployReadyPollSec
    }

    throw "Timed out waiting for previous deployments to stop"
}

function Wait-EdgegapDeploymentReady {
    param(
        [string]$RequestId
    )

    $deadline = (Get-Date).AddSeconds($DeployReadyTimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $status = Invoke-EdgegapApi `
            -Method Get `
            -Uri "https://api.edgegap.com/v1/status/$RequestId"

        if ($status.error) {
            throw "Edgegap deployment $RequestId failed: $($status.error_detail)"
        }

        if ($status.running) {
            return $status
        }

        Write-Host "Waiting for deployment $RequestId (status: $($status.current_status))..."
        Start-Sleep -Seconds $DeployReadyPollSec
    }

    throw "Timed out waiting for Edgegap deployment $RequestId to become ready"
}

function New-DefaultImageTag {
    return "dev-$((Get-Date).ToUniversalTime().ToString("yyyyMMddHHmmss"))"
}

if ([string]::IsNullOrWhiteSpace($ImageTag)) {
    if ($SkipBuild -or $SkipPush) {
        throw "Pass -ImageTag when using -SkipBuild or -SkipPush so the script does not update Edgegap to an unbuilt image tag."
    }
    $ImageTag = New-DefaultImageTag
}

$image = "$Registry/$ImageName`:$ImageTag"

if (-not $SkipBuild -or -not $SkipPush) {
    Assert-DockerAvailable
}

if (-not $SkipBuild) {
    Write-Host "Building $image"
    Invoke-NativeChecked -Command "docker" -Arguments @("build", "--pull", "-t", $image, $RepoRoot)
}

if (-not $SkipPush) {
    Write-Host "Logging in to registry.edgegap.com"
    Ensure-RegistryLogin
    Write-Host "Pushing $image"
    Invoke-NativeChecked -Command "docker" -Arguments @("push", $image)
}

if (-not $SkipVersionUpdate) {
    $versionResponse = Ensure-AppVersion -AppName $AppName -AppVersion $AppVersion
    $versionResponse | ConvertTo-Json -Depth 8
}

if ($SkipDeploy) {
    Write-Host "Skipping deployment"
    return
}

$body = @{
    application = $AppName
    version = $AppVersion
    users = @(
        @{
            user_type = "geo_coordinates"
            user_data = @{
                latitude = $Latitude
                longitude = $Longitude
            }
        }
    )
}

Write-Host "Creating Edgegap deployment for $AppName@$AppVersion"

Stop-ActiveDeployments

$response = Invoke-EdgegapApi `
    -Method Post `
    -Uri "https://api.edgegap.com/v2/deployments" `
    -Body $body

$response | ConvertTo-Json -Depth 8
Write-Host "Deployment request_id: $($response.request_id)"

$ready = Wait-EdgegapDeploymentReady -RequestId $response.request_id
Write-Host "Deployment ready at $($ready.public_ip):$($ready.ports.$GamePortName.external)"
