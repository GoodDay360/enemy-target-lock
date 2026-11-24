param(
    [string] $Instance = "Mod Tester",
    [string] $Launcher = "D:\Program Files\Horizon Launcher\horizon-launcher.exe",
    [switch] $TrustNativeCode
)

$ErrorActionPreference = "Stop"
Set-Location -LiteralPath $PSScriptRoot

if (-not (Test-Path -LiteralPath $Launcher -PathType Leaf)) {
    throw "Horizon Launcher not found: $Launcher"
}

$arguments = @("native-build", ".", "--instance", $Instance)
if ($TrustNativeCode) {
    $arguments += "--trust-native-code"
}

& $Launcher @arguments
exit $LASTEXITCODE
