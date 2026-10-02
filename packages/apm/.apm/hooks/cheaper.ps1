# cheaper platform launcher for Windows（issue #71 /
# DESIGN-034z1EgnQwax4VamK2xmdT）。sibling executable に argv を転送し、
# exit code を伝播する。

$ErrorActionPreference = "Stop"
$exe = Join-Path $PSScriptRoot "cli\bin\win32-x64\cheaper.exe"

if (-not (Test-Path $exe)) {
    Write-Error "cheaper launcher: no executable at $exe"
    exit 1
}

& $exe @args
exit $LASTEXITCODE
