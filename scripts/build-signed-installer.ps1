# Signed desktop build for Data Processing (RnD).
# Private key: %USERPROFILE%\.tauri\rnd-data-processing-updater.key
# Never commit keys, passwords, installers, or .sig files.
#
# Flow: build NSIS without updater-sign step, then sign only the exact current-version
# installer from tauri.conf.json. Never signs a stale previous-version artifact.
$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$keyPath = Join-Path $env:USERPROFILE ".tauri\rnd-data-processing-updater.key"
$bundleDir = Join-Path $repositoryRoot "backend\target\release\bundle\nsis"
$tauriConfigPath = Join-Path $repositoryRoot "backend\tauri.conf.json"

if (-not (Test-Path -LiteralPath $keyPath)) {
    throw "Missing updater private key: $keyPath"
}

$tauriConfig = Get-Content -LiteralPath $tauriConfigPath -Raw | ConvertFrom-Json
$installerName = "{0}_{1}_x64-setup.exe" -f $tauriConfig.productName, $tauriConfig.version
$installerPath = Join-Path $bundleDir $installerName
$signaturePath = "$installerPath.sig"

try {
    Remove-Item -LiteralPath $installerPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $signaturePath -Force -ErrorAction SilentlyContinue

    # Clear signing env during build so createUpdaterArtifacts does not mis-decode the key.
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue

    Push-Location $repositoryRoot
    try {
        Write-Host "Building NSIS installer (no-sign; exact version $installerName)..."
        bun run tauri build --features desktop --config backend/tauri.conf.json --bundles nsis --no-sign
        $exitCode = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }

    if ($exitCode -ne 0) {
        throw "build:desktop (no-sign) exited $exitCode; refusing to sign any existing installer"
    }
    if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
        throw "Expected current-version NSIS installer was not produced: $installerPath"
    }

    Write-Host "Signing: $installerPath"
    Push-Location $repositoryRoot
    try {
        # Explicit flags avoid a Windows shell hang seen when the same values are inherited only
        # through environment variables. The key is unencrypted, so pass an explicit empty password.
        bun tauri signer sign --private-key-path $keyPath --password= $installerPath
        if ($LASTEXITCODE -ne 0) {
            throw "tauri signer exited $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }

    if (-not (Test-Path -LiteralPath $signaturePath -PathType Leaf)) {
        throw "Expected updater signature was not produced: $signaturePath"
    }

    Write-Host "Signed artifacts:"
    Get-ChildItem -LiteralPath $bundleDir | ForEach-Object {
        Write-Host ("  {0} ({1} bytes)" -f $_.Name, $_.Length)
    }
    exit 0
}
finally {
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PATH -ErrorAction SilentlyContinue
}
