# Signed desktop build for Data Processing (RnD).
# Private key: %USERPROFILE%\.tauri\rnd-data-processing-updater.key
# Never commit keys, passwords, installers, or .sig files.
$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$keyPath = Join-Path $env:USERPROFILE ".tauri\rnd-data-processing-updater.key"
$bundleDir = Join-Path $repositoryRoot "backend\target\release\bundle\nsis"

if (-not (Test-Path -LiteralPath $keyPath)) {
    throw "Missing updater private key: $keyPath"
}

# Prefer key *contents* (path alone is flaky under bun/tauri on Windows).
# Empty password string is required so signer does not hang waiting on stdin.
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -LiteralPath $keyPath -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PATH -ErrorAction SilentlyContinue

try {
    Push-Location $repositoryRoot
    try {
        Write-Host "Building NSIS installer..."
        bun run build:desktop
        $exitCode = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }

    $installer = Get-ChildItem -LiteralPath $bundleDir -Filter "*_x64-setup.exe" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $installer) {
        throw "No NSIS installer found under $bundleDir"
    }

    $installerPath = $installer.FullName
    if ($exitCode -ne 0) {
        Write-Warning "build:desktop exited $exitCode (often signing at end). Installer present; signing separately..."
    }

    Write-Host "Signing: $installerPath"
    Push-Location $repositoryRoot
    try {
        bun tauri signer sign $installerPath
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
    finally {
        Pop-Location
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
