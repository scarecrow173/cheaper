param([string]$ProjectRoot = (Get-Location).Path, [switch]$Apply)

$ErrorActionPreference = 'Stop'
$OutputEncoding = [Text.Encoding]::ASCII
Add-Type -AssemblyName System.Runtime.Serialization

$ownCommand = 'cheaper hook pre-tool-use --target devin'
$ownMatcher = '^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp])$'
$installHint = 'Install or update the CLI: cargo install --git https://github.com/scarecrow173/cheaper.git --locked --force cheaper-cli'
$commandKeys = @('command', 'powershell', 'bash', 'windows', 'linux', 'osx')
$conflictRegex = 'cheaper[^\r\n]*hook[^\r\n]*(pre-read-code|pre-tool-use)'
$knownConfigs = @(
    '.windsurf\hooks.json',
    '.devin\hooks.json',
    '.devin\hooks.v1.json',
    '.devin\config.local.json',
    '.claude\settings.json',
    '.claude\settings.local.json',
    '.devin\config.json'
)

function Get-JsonShape([string]$Text) {
    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    $reader = [System.Runtime.Serialization.Json.JsonReaderWriterFactory]::CreateJsonReader($bytes, [System.Xml.XmlDictionaryReaderQuotas]::Max)
    try {
        $document = New-Object System.Xml.XmlDocument
        $document.Load($reader)
        return $document.OuterXml
    } finally {
        $reader.Dispose()
    }
}

function Test-JsonObject($Value) {
    return $null -ne $Value -and $Value.GetType().FullName -eq 'System.Management.Automation.PSCustomObject'
}

function Fail([string]$Message) {
    Write-Host ("cheaper: " + $Message)
    exit 1
}

function Get-ExactKey($Obj, [string]$Name) {
    foreach ($p in $Obj.PSObject.Properties) {
        if ($p.Name -ceq $Name) { return $p }
    }
    return $null
}

function Get-ConfigKey($Obj, [string]$Name) {
    $exact = $null
    foreach ($p in $Obj.PSObject.Properties) {
        if ($p.Name -ceq $Name) { $exact = $p }
        elseif ($p.Name -ieq $Name) { Fail ('config.json contains a differently-cased key: ' + $Name) }
    }
    return $exact
}

function Test-Reparse([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return $false }
    try { $item = Get-Item -LiteralPath $Path -Force } catch { Fail 'a path component could not be inspected' }
    return [bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
}

function Assert-PathNoReparse([string]$Path) {
    $cur = $Path
    while ($true) {
        if (Test-Reparse $cur) { Fail 'a path component is a reparse point or symlink' }
        $parent = [IO.Path]::GetDirectoryName($cur)
        if ([string]::IsNullOrEmpty($parent) -or $parent -eq $cur) { break }
        $cur = $parent
    }
}

function Assert-NativeHookGroup($Group) {
    if (-not (Test-JsonObject $Group)) { Fail 'config.json hooks.PreToolUse groups must be objects' }
    $matcher = Get-ConfigKey $Group 'matcher'
    if ($null -ne $matcher -and $matcher.Value -isnot [string]) {
        Fail 'config.json hooks.PreToolUse group matcher must be a string'
    }
    $hooks = Get-ConfigKey $Group 'hooks'
    if ($null -eq $hooks) { Fail 'config.json hooks.PreToolUse group is missing hooks' }
    if ($hooks.Value -isnot [Object[]]) { Fail 'config.json hooks.PreToolUse group hooks must be an array' }
    foreach ($h in $hooks.Value) {
        if (-not (Test-JsonObject $h)) { Fail 'config.json hooks.PreToolUse hook entries must be objects' }
        $type = Get-ConfigKey $h 'type'
        if ($null -eq $type -or $type.Value -isnot [string] -or ($type.Value -cne 'command' -and $type.Value -cne 'prompt')) {
            Fail 'config.json hooks.PreToolUse hook type must be command or prompt'
        }
        if ($type.Value -ceq 'command') {
            $cmd = Get-ConfigKey $h 'command'
            if ($null -eq $cmd -or $cmd.Value -isnot [string]) {
                Fail 'config.json hooks.PreToolUse command hook requires a command string'
            }
        } else {
            $prompt = Get-ConfigKey $h 'prompt'
            if ($null -eq $prompt -or $prompt.Value -isnot [string]) {
                Fail 'config.json hooks.PreToolUse prompt hook requires a prompt string'
            }
        }
    }
}

function Collect-CommandStrings($Node, $Skip, $Out) {
    if ($null -ne $Skip -and [object]::ReferenceEquals($Node, $Skip)) { return }
    if (Test-JsonObject $Node) {
        foreach ($prop in $Node.PSObject.Properties) {
            $v = $prop.Value
            if ($script:commandKeys -ccontains $prop.Name -and $v -is [string]) {
                $Out.Add($v)
            } else {
                Collect-CommandStrings $v $Skip $Out
            }
        }
    } elseif ($Node -is [Object[]]) {
        foreach ($e in $Node) { Collect-CommandStrings $e $Skip $Out }
    }
}

try { $root = [IO.Path]::GetFullPath($ProjectRoot) } catch { Fail 'project root is invalid' }
if (-not (Test-Path -LiteralPath $root)) { Fail 'project root not found' }
try { $rootIsDir = (Get-Item -LiteralPath $root -Force).PSIsContainer } catch { Fail 'project root could not be inspected' }
if (-not $rootIsDir) { Fail 'project root is not a directory' }

$devinDir = Join-Path $root '.devin'
$configPath = Join-Path $devinDir 'config.json'
Assert-PathNoReparse $configPath
if (Test-Path -LiteralPath $devinDir) {
    try { $devinIsDir = (Get-Item -LiteralPath $devinDir -Force).PSIsContainer } catch { Fail '.devin could not be inspected' }
    if (-not $devinIsDir) { Fail '.devin is not a directory' }
}
$configExists = Test-Path -LiteralPath $configPath
if ($configExists) {
    try { $configIsDir = (Get-Item -LiteralPath $configPath -Force).PSIsContainer } catch { Fail 'config.json could not be inspected' }
    if ($configIsDir) { Fail 'config.json is not a regular file' }
}

$rawBytes = $null
$config = [PSCustomObject]@{}
if ($configExists) {
    try { $rawBytes = [IO.File]::ReadAllBytes($configPath) } catch { Fail 'config.json could not be read' }
    $rawText = [Text.Encoding]::UTF8.GetString($rawBytes)
    try { $parsed = $rawText | ConvertFrom-Json } catch { Fail 'config.json is not valid JSON' }
    if (-not (Test-JsonObject $parsed)) { Fail 'config.json root must be a JSON object' }
    $config = $parsed
}

$hooksObj = $null
$hooksProp = Get-ConfigKey $config 'hooks'
if ($null -ne $hooksProp) {
    if (-not (Test-JsonObject $hooksProp.Value)) { Fail 'config.json hooks must be an object' }
    $hooksObj = $hooksProp.Value
}
$ptu = $null
if ($null -ne $hooksObj) {
    $ptuProp = Get-ConfigKey $hooksObj 'PreToolUse'
    if ($null -ne $ptuProp) {
        if ($ptuProp.Value -isnot [Object[]]) { Fail 'config.json hooks.PreToolUse must be an array' }
        $ptu = $ptuProp.Value
        foreach ($g in $ptu) {
            Assert-NativeHookGroup $g
        }
    }
}

if ($configExists) {
    $roundtrip = $null
    try { $roundtrip = ConvertTo-Json -InputObject $config -Depth 100 -Compress } catch { Fail 'config.json could not be reserialized safely' }
    $shapeBefore = $null
    $shapeAfter = $null
    try { $shapeBefore = Get-JsonShape $rawText } catch { Fail 'config.json could not be analyzed safely' }
    try { $shapeAfter = Get-JsonShape $roundtrip } catch { Fail 'config.json could not be reserialized safely' }
    if ($shapeBefore -cne $shapeAfter) { Fail 'config.json content cannot be preserved safely' }
}

$ownGroups = @()
if ($null -ne $ptu) {
    foreach ($g in $ptu) {
        $gHooksProp = Get-ConfigKey $g 'hooks'
        if ($null -eq $gHooksProp) { continue }
        foreach ($h in @($gHooksProp.Value)) {
            $hCmd = if (Test-JsonObject $h) { Get-ConfigKey $h 'command' } else { $null }
            if ($null -ne $hCmd -and $hCmd.Value -ceq $ownCommand) {
                $ownGroups += $g
                break
            }
        }
    }
}
if ($ownGroups.Count -gt 1) { Fail 'multiple cheaper devin hook entries found; resolve them manually' }
$alreadyRegistered = $false
if ($ownGroups.Count -eq 1) {
    $g = $ownGroups[0]
    $names = @($g.PSObject.Properties.Name)
    $ok = ($names.Count -eq 2) -and ($names -ccontains 'matcher') -and ($names -ccontains 'hooks')
    if ($ok) { $ok = $g.matcher -ceq $ownMatcher }
    if ($ok) {
        $hs = @($g.hooks)
        $ok = $hs.Count -eq 1
        if ($ok) {
            $h = $hs[0]
            if (-not (Test-JsonObject $h)) {
                $ok = $false
            } else {
                $hn = @($h.PSObject.Properties.Name)
                $hType = Get-ConfigKey $h 'type'
                $hCmd = Get-ConfigKey $h 'command'
                $ok = ($hn.Count -eq 2) -and ($hn -ccontains 'type') -and ($hn -ccontains 'command') -and ($hType.Value -ceq 'command') -and ($hCmd.Value -ceq $ownCommand)
            }
        }
    }
    if (-not $ok) { Fail 'the existing cheaper devin hook entry was modified; resolve it manually' }
    $alreadyRegistered = $true
}

$conflicts = New-Object 'System.Collections.Generic.List[string]'
$primarySkip = $null
if ($alreadyRegistered) { $primarySkip = $ownGroups[0] }
foreach ($rel in $knownConfigs) {
    $p = Join-Path $root $rel
    if (-not (Test-Path -LiteralPath $p)) { continue }
    Assert-PathNoReparse $p
    try { $knownIsDir = (Get-Item -LiteralPath $p -Force).PSIsContainer } catch { Fail ('known config path could not be inspected: ' + $rel) }
    if ($knownIsDir) { Fail ('known config path is not a file: ' + $rel) }
    $doc = $null
    if ($rel -eq '.devin\config.json') {
        $doc = $config
    } else {
        try {
            $doc = [Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes($p)) | ConvertFrom-Json
        } catch {
            Fail ('known config is not parseable: ' + $rel)
        }
    }
    $skip = $null
    if ($rel -eq '.devin\config.json') { $skip = $primarySkip }
    $hits = New-Object 'System.Collections.Generic.List[string]'
    Collect-CommandStrings $doc $skip $hits
    foreach ($s in $hits) {
        if ($s -match $conflictRegex) { $conflicts.Add($rel); break }
    }
}
if ($conflicts.Count -gt 0) {
    Fail ('existing cheaper read hook registration found: ' + ($conflicts -join ', ') + '. Resolve duplicates manually, then re-run.')
}

if (-not $Apply) {
    if ($alreadyRegistered) {
        Write-Host 'cheaper: preview - Devin native read/grep hook is already registered in .devin\config.json (no changes).'
    } else {
        Write-Host 'cheaper: preview - would add a Devin native read/grep PreToolUse entry to .devin\config.json.'
    }
    exit 0
}

$exe = $null
foreach ($candidate in @('cheaper.exe', 'cheaper')) {
    $found = Get-Command $candidate -CommandType Application -ErrorAction SilentlyContinue
    if ($found) { $exe = $found.Source; break }
}
if (-not $exe) { Fail ('cheaper CLI not found on PATH. ' + $installHint) }
$capText = $null
try {
    $capText = & $exe hook targets --json 2>$null | Out-String
} catch {
    Fail ('cheaper CLI capability check failed. ' + $installHint)
}
if ($LASTEXITCODE -ne 0) { Fail ('cheaper CLI capability check failed. ' + $installHint) }
$cap = $null
try { $cap = $capText.Trim() | ConvertFrom-Json } catch { Fail ('cheaper CLI capability response was not understood. ' + $installHint) }
$capOk = $false
if (Test-JsonObject $cap) {
    $svProp = Get-ExactKey $cap 'schemaVersion'
    $targetsProp = Get-ExactKey $cap 'targets'
    $sv = $null
    $targets = $null
    if ($null -ne $svProp) { $sv = $svProp.Value }
    if ($null -ne $targetsProp) { $targets = $targetsProp.Value }
    $svOk = (($sv -is [int]) -or ($sv -is [long])) -and ($sv -eq 1)
    $tOk = $targets -is [Object[]]
    if ($tOk) {
        foreach ($t in $targets) { if ($t -isnot [string]) { $tOk = $false } }
    }
    $capOk = $svOk -and $tOk -and ($targets -ccontains 'devin')
}
if (-not $capOk) { Fail ('cheaper CLI does not report the devin hook target. ' + $installHint) }

if ($alreadyRegistered) {
    Write-Host 'cheaper: Devin native read/grep hook is already registered in .devin\config.json (no changes).'
    exit 0
}

if ($null -eq $hooksObj) {
    $hooksObj = [PSCustomObject]@{}
    $config | Add-Member -MemberType NoteProperty -Name 'hooks' -Value $hooksObj
}
if ($null -eq $ptu) {
    $ptu = @()
    $hooksObj | Add-Member -MemberType NoteProperty -Name 'PreToolUse' -Value $ptu
}
$canonical = [PSCustomObject]@{
    matcher = $ownMatcher
    hooks = @([PSCustomObject]@{ type = 'command'; command = $ownCommand })
}
$hooksObj.PreToolUse = @($ptu) + $canonical

$newText = $null
try { $newText = ConvertTo-Json -InputObject $config -Depth 100 } catch { Fail 'updated config could not be serialized' }

Assert-PathNoReparse $configPath
if ($configExists) {
    $nowBytes = $null
    try { $nowBytes = [IO.File]::ReadAllBytes($configPath) } catch { Fail 'config.json could not be read' }
    if ([Convert]::ToBase64String($nowBytes) -cne [Convert]::ToBase64String($rawBytes)) {
        Fail 'config.json changed while preparing the update'
    }
} elseif (Test-Path -LiteralPath $configPath) {
    Fail 'config.json appeared while preparing the update'
}

$utf8 = New-Object Text.UTF8Encoding($false)
if (-not (Test-Path -LiteralPath $devinDir)) {
    try { [IO.Directory]::CreateDirectory($devinDir) | Out-Null } catch { Fail '.devin directory could not be created' }
}
$tempPath = Join-Path $devinDir ('config.json.' + [Guid]::NewGuid().ToString('N') + '.tmp')
$backupPath = Join-Path $devinDir ('config.json.' + [Guid]::NewGuid().ToString('N') + '.bak')
$writeFailed = $false
try {
    [IO.File]::WriteAllText($tempPath, $newText, $utf8)
    if ($configExists) {
        [IO.File]::Replace($tempPath, $configPath, $backupPath)
    } else {
        [IO.File]::Move($tempPath, $configPath)
    }
} catch {
    $writeFailed = $true
} finally {
    foreach ($p in @($tempPath, $backupPath)) {
        if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force -ErrorAction SilentlyContinue }
    }
}
if ($writeFailed) { Fail 'config.json could not be updated safely' }

Write-Host 'cheaper: registered Devin native read/grep hook in .devin\config.json.'
Write-Host 'cheaper: this is supplemental registration, not an APM-managed hook target; APM uninstall will not remove it. To uninstall, delete only the "cheaper hook pre-tool-use --target devin" group manually.'
Write-Host 'cheaper: global and ancestor hook sources are not fully detected; confirm loaded sources with /hooks. Native shell tools are not registered.'
exit 0
