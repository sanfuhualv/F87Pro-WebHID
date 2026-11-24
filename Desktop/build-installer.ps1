param(
    [switch]$Offline,
    [switch]$SkipAppBuild,
    [string]$TargetDir="$env:LOCALAPPDATA\F87ProDesktopBuild",
    [string]$ToolDir="$env:LOCALAPPDATA\F87ProDesktopBuildTools"
)
$ErrorActionPreference='Stop'
Push-Location $PSScriptRoot
try {
    if(-not $SkipAppBuild){
        & (Join-Path $PSScriptRoot 'build.ps1') -Offline:$Offline -TargetDir $TargetDir
    }
    $config=Get-Content -LiteralPath src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
    $version=$config.version
    $exe=Join-Path $PSScriptRoot "output\$($config.productName)-$version.exe"
    if(-not (Test-Path -LiteralPath $exe)){throw "Application executable is missing: $exe"}
    New-Item -ItemType Directory -Path $ToolDir -Force | Out-Null
    $nsis=Join-Path $ToolDir 'nsis-3.11\makensis.exe'
    $zip=Join-Path $ToolDir 'nsis-3.11.zip'
    if(-not (Test-Path -LiteralPath $nsis)){
        if(-not (Test-Path -LiteralPath $zip)){
            if($Offline){throw 'NSIS 3.11 is not cached. Build once without -Offline.'}
            & curl.exe --fail --location --max-time 60 --output $zip 'https://downloads.sourceforge.net/project/nsis/NSIS%203/3.11/nsis-3.11.zip'
            if($LASTEXITCODE -ne 0){throw 'NSIS download failed'}
        }
        $hash=(Get-FileHash -Algorithm SHA256 -LiteralPath $zip).Hash
        if($hash -ne 'C7D27F780DDB6CFFB4730138CD1591E841F4B7EDB155856901CDF5F214394FA1'){throw 'NSIS archive checksum does not match'}
        Expand-Archive -LiteralPath $zip -DestinationPath $ToolDir -Force
    }
    $webview=Join-Path $ToolDir 'MicrosoftEdgeWebview2Setup.exe'
    if(-not (Test-Path -LiteralPath $webview)){
        if($Offline){throw 'Microsoft WebView2 bootstrapper is not cached. Build once without -Offline.'}
        & curl.exe --fail --location --max-time 60 --output $webview 'https://go.microsoft.com/fwlink/p/?LinkId=2124703'
        if($LASTEXITCODE -ne 0){throw 'Microsoft WebView2 bootstrapper download failed'}
    }
    $signature=Get-AuthenticodeSignature -LiteralPath $webview
    if($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation(?:,|$)'){
        throw 'Microsoft WebView2 bootstrapper signature is not valid'
    }
    $output=Join-Path $PSScriptRoot "output\F87 Pro Desktop-Setup-$version.exe"
    & $nsis '/WX' '/INPUTCHARSET' 'UTF8' "/DAPP_EXE=$exe" "/DAPP_VERSION=$version" "/DOUTPUT_FILE=$output" "/DWEBVIEW2_BOOTSTRAPPER=$webview" 'installer/f87-desktop.nsi'
    if($LASTEXITCODE -ne 0){throw 'Installer compilation failed'}
    Write-Host "Installer ready: $output"
} finally {Pop-Location}
