$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$bundleDirectory = Join-Path $repositoryRoot "backend\target\release\bundle\nsis"
$buildTimeoutSeconds = 600
$bundleTimeoutSeconds = 180

function Write-CommandOutput {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (Test-Path -LiteralPath $Path -PathType Leaf) {
        Get-Content -LiteralPath $Path | ForEach-Object { Write-Host $_ }
    }
}

function Invoke-LocalTauri {
    param(
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds
    )

    $commandLabel = "bun run tauri " + ($Arguments -join " ")
    $stdoutPath = Join-Path ([System.IO.Path]::GetTempPath()) "rnd-tauri-$([guid]::NewGuid()).stdout.log"
    $stderrPath = Join-Path ([System.IO.Path]::GetTempPath()) "rnd-tauri-$([guid]::NewGuid()).stderr.log"
    $process = $null
    $exitCode = 1

    Write-Host "`$ $commandLabel"
    try {
        $process = Start-Process `
            -FilePath "bun" `
            -ArgumentList (@("run", "tauri") + $Arguments) `
            -WorkingDirectory $repositoryRoot `
            -RedirectStandardOutput $stdoutPath `
            -RedirectStandardError $stderrPath `
            -PassThru

        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            Write-Warning "Tauri command timed out after $TimeoutSeconds seconds: $commandLabel"
            & taskkill.exe /PID $process.Id /T /F 2>$null | Out-Null
            $process.WaitForExit(10000) | Out-Null
            $exitCode = 124
        }
        else {
            $process.WaitForExit()
            $process.Refresh()
            $exitCode = [int]$process.ExitCode
        }
    }
    finally {
        Write-CommandOutput -Path $stdoutPath
        Write-CommandOutput -Path $stderrPath
        Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
    }

    return $exitCode
}

Push-Location $repositoryRoot
try {
    $buildExitCode = Invoke-LocalTauri `
        -Arguments @("build", "--features", "desktop", "--config", "backend/tauri.conf.json", "--no-bundle", "--no-sign") `
        -TimeoutSeconds $buildTimeoutSeconds
    if ($buildExitCode -ne 0) {
        exit $buildExitCode
    }

    $lastExitCode = 1
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        $delaySeconds = 10 * $attempt
        Write-Host "Waiting $delaySeconds seconds before NSIS bundle attempt $attempt..."
        Start-Sleep -Seconds $delaySeconds

        if (Test-Path $bundleDirectory) {
            Remove-Item $bundleDirectory -Recurse -Force
        }

        $lastExitCode = Invoke-LocalTauri `
            -Arguments @("bundle", "--features", "desktop", "--config", "backend/tauri.conf.json", "--bundles", "nsis", "--no-sign") `
            -TimeoutSeconds $bundleTimeoutSeconds
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
