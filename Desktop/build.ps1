param([switch]$Offline, [string]$TargetDir = "$env:LOCALAPPDATA\F87ProDesktopBuild")
$ErrorActionPreference = 'Stop'
$previousTarget = $env:CARGO_TARGET_DIR
Push-Location $PSScriptRoot
try {
    node build-licenses.mjs
    if ($LASTEXITCODE -ne 0) { throw 'Third-party license preparation failed' }
    node prepare.mjs
    if ($LASTEXITCODE -ne 0) { throw 'Asset preparation failed' }
    $env:CARGO_TARGET_DIR = $TargetDir
    $cargoArgs = @('build', '--release', '--locked', '--manifest-path', 'src-tauri/Cargo.toml')
    if ($Offline) { $cargoArgs += '--offline' }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw 'Desktop build failed' }
    New-Item -ItemType Directory -Path output -Force | Out-Null
    $desktopConfig=Get-Content -LiteralPath src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
    $desktopVersion=$desktopConfig.version
    $desktopExeName="$($desktopConfig.productName)-$desktopVersion.exe"
    Copy-Item -LiteralPath "$TargetDir\release\f87-desktop.exe" -Destination (Join-Path output $desktopExeName) -Force
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot '../LICENSE') -Destination output/LICENSE -Force
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot '../THIRD_PARTY_NOTICES.md') -Destination output/THIRD_PARTY_NOTICES.md -Force
    New-Item -ItemType Directory -Path output/third-party-licenses -Force | Out-Null
    Copy-Item -Path (Join-Path $PSScriptRoot 'licenses/generated/*') -Destination output/third-party-licenses -Recurse -Force
    Write-Host "Ready: $PSScriptRoot\output\$desktopExeName"
} finally {
    $env:CARGO_TARGET_DIR = $previousTarget
    Pop-Location
}
