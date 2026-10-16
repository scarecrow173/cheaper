param([string]$ProjectRoot = (Get-Location).Path, [switch]$Apply)

# Devin hook 登録の所有権は Rust CLI (`cheaper hook-config`) が持つ。
# この script は preview/apply UX のみを提供する thin wrapper —
# config の解析・検証・atomic write・legacy conflict gate は
# `hook-config sync|verify` が担う(ADR-034H2WBNtC4We8dyC5lQZJ、
# issue #107)。

$ErrorActionPreference = 'Stop'
$installHint = 'Install or update the CLI: cargo install --git https://github.com/scarecrow173/cheaper.git --locked --force cheaper-cli'

function Fail([string]$Message) {
    Write-Host ("cheaper: " + $Message)
    exit 1
}

try { $root = [IO.Path]::GetFullPath($ProjectRoot) } catch { Fail 'project root is invalid' }
if (-not (Test-Path -LiteralPath $root)) { Fail 'project root not found' }
try { $rootIsDir = (Get-Item -LiteralPath $root -Force).PSIsContainer } catch { Fail 'project root could not be inspected' }
if (-not $rootIsDir) { Fail 'project root is not a directory' }

$exe = $null
foreach ($candidate in @('cheaper.exe', 'cheaper')) {
    $found = Get-Command $candidate -CommandType Application -ErrorAction SilentlyContinue
    if ($found) { $exe = $found.Source; break }
}
if (-not $exe) { Fail ('cheaper CLI not found on PATH. ' + $installHint) }

# capability 事前確認 — PATH 上の binary が `hook-config` を持つこと
# (`hook targets --json` の `hookConfig` field)。旧 CLI では
# sync/verify が存在しないため誤って成功扱いしない。
$capText = $null
try {
    $capText = & $exe hook targets --json 2>$null | Out-String
} catch {
    Fail ('cheaper CLI capability check failed. ' + $installHint)
}
if ($LASTEXITCODE -ne 0) { Fail ('cheaper CLI capability check failed. ' + $installHint) }
$cap = $null
try { $cap = $capText.Trim() | ConvertFrom-Json } catch { Fail ('cheaper CLI capability response was not understood. ' + $installHint) }
$hasDevin = $false
$hasHookConfig = $false
if ($null -ne $cap) {
    if ($null -ne $cap.targets) { $hasDevin = @($cap.targets) -ccontains 'devin' }
    if ($null -ne $cap.hookConfig) { $hasHookConfig = [bool]$cap.hookConfig }
}
if (-not $hasDevin) { Fail ('cheaper CLI does not report the devin hook target. ' + $installHint) }
if (-not $hasHookConfig) {
    Fail ('cheaper CLI does not support hook-config (hookConfig missing). ' + $installHint)
}

if (-not $Apply) {
    # preview — read-only verify の結果を preview 文として要約する。
    $out = $null
    try {
        $out = & $exe hook-config verify --project-root $root 2>$null | Out-String
    } catch { }
    $own = $null
    try { $own = (($out.Trim() | ConvertFrom-Json).result).ownEntry } catch { }
    if ($own -eq 'present') {
        Write-Host 'cheaper: preview - Devin native read/grep/exec hook is already registered in .devin\config.json (no changes).'
    } else {
        Write-Host 'cheaper: preview - would add a Devin native read/grep/exec PreToolUse entry to .devin\config.json.'
        if ($own -eq 'drifted' -or $own -eq 'modified' -or $own -eq 'duplicate') {
            Write-Host ('cheaper: preview - existing registration state: ' + $own + ' (sync will converge or refuse per the ownership contract).')
        }
    }
    exit 0
}

$out = $null
# stderr を合成しない — `2>&1` 下では stderr の各行が ErrorRecord 化し
# `$ErrorActionPreference='Stop'` で第1行で throw して error envelope が
# 読めなくなる。stderr は host へそのまま流す。
try {
    $out = & $exe hook-config sync --project-root $root | Out-String
} catch {
    Fail 'hook-config sync could not be executed'
}
if ($LASTEXITCODE -ne 0) {
    Write-Host $out
    Fail 'hook-config sync failed (see the error above)'
}
try {
    $result = (($out.Trim() | ConvertFrom-Json).result)
    if ($result.changed) {
        Write-Host 'cheaper: registered Devin native read/grep/exec hook in .devin\config.json.'
    } else {
        Write-Host 'cheaper: Devin native read/grep/exec hook is already registered in .devin\config.json (no changes).'
    }
} catch {
    Write-Host $out
}
Write-Host 'cheaper: manage this registration with `cheaper hook-config verify|remove`; APM lifecycle events converge it when the consumer apm.yml wires hook-config (see docs/guides/distribution.md).'
exit 0
