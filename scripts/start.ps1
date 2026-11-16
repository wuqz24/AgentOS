param([int]$Port = 8765)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
if (-not (Test-Path -LiteralPath 'host/local.env')) {
    Copy-Item -LiteralPath 'host/local.env.example' -Destination 'host/local.env'
}
python -m host.server --port $Port
exit $LASTEXITCODE
