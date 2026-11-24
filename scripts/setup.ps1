$ErrorActionPreference = "Stop"

$launcher = "D:\Program Files\Horizon Launcher\horizon-launcher.exe"
if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) {
    throw "Horizon Launcher not found: $launcher"
}

& $launcher rust install
exit $LASTEXITCODE
