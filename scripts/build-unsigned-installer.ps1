$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$bundleDirectory = Join-Path $repositoryRoot "backend\target\release\bundle\nsis"

Push-Location $repositoryRoot
try {
    & cargo tauri build --features desktop --config backend/tauri.conf.json --no-bundle --no-sign
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    $lastExitCode = 1
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        $delaySeconds = 10 * $attempt
        Write-Host "Waiting $delaySeconds seconds before NSIS bundle attempt $attempt..."
        Start-Sleep -Seconds $delaySeconds

        if (Test-Path $bundleDirectory) {
            Remove-Item $bundleDirectory -Recurse -Force
        }

        & cargo tauri bundle --features desktop --config backend/tauri.conf.json --bundles nsis --no-sign
        $lastExitCode = $LASTEXITCODE
        if ($lastExitCode -eq 0) {
            exit 0
        }

        Write-Warning "NSIS bundle attempt $attempt failed with exit code $lastExitCode."
    }

    exit $lastExitCode
}
finally {
    Pop-Location
}
