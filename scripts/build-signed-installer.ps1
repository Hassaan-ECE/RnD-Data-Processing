# Signed desktop build for Data Processing (RnD).
# Private key: %USERPROFILE%\.tauri\rnd-data-processing-updater.key
# Never commit keys, passwords, installers, or .sig files.
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

# Prefer key *contents* (path alone is flaky under bun/tauri on Windows).
# Empty password string is required so signer does not hang waiting on stdin.
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -LiteralPath $keyPath -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PATH -ErrorAction SilentlyContinue

try {
    Remove-Item -LiteralPath $installerPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $signaturePath -Force -ErrorAction SilentlyContinue

    Push-Location $repositoryRoot
    try {
        Write-Host "Building NSIS installer..."
        bun run build:desktop
        $exitCode = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }

    if ($exitCode -ne 0) {
        throw "build:desktop exited $exitCode; refusing to sign any existing installer"
    }
    if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
        throw "Expected current-version NSIS installer was not produced: $installerPath"
    }

    if (-not (Test-Path -LiteralPath $signaturePath -PathType Leaf)) {
        Write-Host "Signing: $installerPath"
        Push-Location $repositoryRoot
        try {
            bun tauri signer sign $installerPath
            if ($LASTEXITCODE -ne 0) {
                throw "tauri signer exited $LASTEXITCODE"
            }
        }
        finally {
            Pop-Location
        }
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
