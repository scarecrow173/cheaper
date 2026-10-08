# Node-free acquisition。契約: DESIGN-034zQkHD65kAeG9xVGSoFX。
$ErrorActionPreference = "Stop"

function Invoke-Executable {
    param([string]$Path, [string[]]$Arguments, [switch]$Probe)
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $Path
    $info.UseShellExecute = $false
    # PS 5.1 native invocation の空引数欠落を避け、Windows argv を直接構成する。
    $quoted = foreach ($argument in $Arguments) {
        '"' + [regex]::Replace([regex]::Replace($argument, '(\\*)"', '$1$1\"'), '(\\+)$', '$1$1') + '"'
    }
    $info.Arguments = $quoted -join " "
    if ($Probe) {
        $info.RedirectStandardInput = $true
        $info.RedirectStandardOutput = $true
        $info.RedirectStandardError = $true
    }
    $child = New-Object System.Diagnostics.Process
    $child.StartInfo = $info
    try {
        if (-not $child.Start()) { throw "executable could not start; preseed the correct Release pair." }
        if ($Probe) {
            $child.StandardInput.Close()
            $output = $child.StandardOutput.ReadToEndAsync()
            $diagnostic = $child.StandardError.ReadToEndAsync()
            if (-not $child.WaitForExit(15000)) {
                $child.Kill()
                $child.WaitForExit()
                throw "executable version probe timed out; preseed its matching Release pair."
            }
            if ($child.ExitCode -ne 0) { throw "executable version probe failed; preseed the correct Release pair." }
            return $output.GetAwaiter().GetResult().TrimEnd([char[]]"`r`n")
        }
        $child.WaitForExit()
        return $child.ExitCode
    } finally { $child.Dispose() }
}

function Test-LfsPointer {
    param([string]$Path)
    $stream = [IO.File]::OpenRead($Path)
    try {
        $bytes = New-Object byte[] 44
        $length = $stream.Read($bytes, 0, $bytes.Length)
        return [Text.Encoding]::ASCII.GetString($bytes, 0, $length).StartsWith("version https://git-lfs")
    } finally { $stream.Dispose() }
}

function Test-ReleaseUri {
    param([Uri]$Uri, [switch]$Redirect)
    if (-not $Uri.IsAbsoluteUri -or $Uri.UserInfo -or $Uri.Query -or $Uri.Fragment) { return $false }
    if ($Uri.Scheme -eq "https") { return $true }
    if ($Redirect) { return $false }
    return $Uri.Scheme -eq "http" -and $Uri.Host -in @("localhost", "127.0.0.1", "[::1]", "::1")
}

function Get-ReleaseFile {
    param([string]$Asset, [string]$Destination)
    $uri = [Uri]("$script:releaseBase/v$script:version/$Asset")
    $curl = Get-Command curl.exe -CommandType Application -ErrorAction SilentlyContinue
    try {
        if ($curl) {
            & $curl.Source --fail --silent --location --connect-timeout 15 --max-time 120 --proto '=http,https' --proto-redir '-all,https' --output $Destination $uri.AbsoluteUri
            if ($LASTEXITCODE -ne 0) { throw "download failed" }
            return
        }
        # PS 5.1 fallback。自動 redirect を無効化し Location を毎回検証する。
        $watch = [Diagnostics.Stopwatch]::StartNew()
        for ($hop = 0; $hop -lt 10; $hop++) {
            $remaining = 120 - [int][Math]::Ceiling($watch.Elapsed.TotalSeconds)
            if ($remaining -le 0) { throw "download timed out" }
            $request = @{
                Uri = $uri.AbsoluteUri; OutFile = $Destination; PassThru = $true
                UseBasicParsing = $true; MaximumRedirection = 0
                TimeoutSec = $remaining; ErrorAction = "Stop"
            }
            if ($env:HTTPS_PROXY) { $request.Proxy = $env:HTTPS_PROXY }
            $response = $null
            try { $response = Invoke-WebRequest @request }
            catch {
                $response = $_.Exception.Response
                if (-not $response) { throw }
            }
            $status = [int]$response.StatusCode
            if ($status -ge 200 -and $status -lt 300) { return }
            if ($status -notin @(301, 302, 303, 307, 308)) { throw "HTTP download failed" }
            $location = $response.Headers["Location"]
            if (-not $location) { throw "redirect has no Location" }
            $uri = New-Object Uri($uri, [string]$location)
            if (-not (Test-ReleaseUri $uri -Redirect)) { throw "redirect must use HTTPS" }
        }
        throw "too many redirects"
    } catch {
        # origin/proxy credential や download 本文を診断へ含めない。
        throw "download of $Asset for v$script:version failed; check tagged Release/SHA256SUMS, HTTPS proxy or trusted mirror, or offline preseed the pair."
    }
}

function Confirm-Cache {
    param([string]$Directory)
    $candidate = Join-Path $Directory "cheaper.exe"
    $sums = Join-Path $Directory "SHA256SUMS"
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf) -or -not (Test-Path -LiteralPath $sums -PathType Leaf)) {
        throw "cache requires executable and SHA256SUMS; remove the incomplete key and preseed the verified pair."
    }
    if ((Get-Item -LiteralPath $sums).Length -gt 65536) { throw "checksum manifest exceeds 64KiB." }
    $manifest = [IO.File]::ReadAllText($sums)
    if ($manifest.Contains([char]13) -or $manifest -match '[^\x00-\x7f]') { throw "invalid checksum manifest; expected ASCII LF lines." }
    $lines = $manifest.Split([char]10)
    # 最後のLF1個のみ許可し、空行は拒否する。
    if ($lines[-1] -eq "") { $lines = $lines[0..($lines.Length - 2)] }
    $expected = $null
    foreach ($line in $lines) {
        if ($line -cnotmatch '^([0-9A-Fa-f]{64})  ([A-Za-z0-9._-]+)$') { throw "invalid checksum manifest format." }
        if ($Matches[2] -ceq "cheaper-windows-x64.exe") {
            if ($null -ne $expected) { throw "duplicate checksum manifest entry." }
            $expected = $Matches[1]
        }
    }
    if ($null -eq $expected) { throw "checksum manifest has no cheaper-windows-x64.exe entry." }
    $actual = (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash
    if ($actual -ine $expected) { throw "checksum mismatch; remove $script:key and preseed the correct same-tag pair." }
    try { $observed = Invoke-Executable -Path $candidate -Arguments @("--version") -Probe }
    catch { throw "executable version probe failed; remove $script:key and preseed the correct Release pair." }
    if ($observed -cne $script:version) { throw "wrong executable version (expected $script:version); remove $script:key and preseed its matching Release pair." }
}

$lock = $null
$stage = $null
$ownsLock = $false
$exitCode = 1
try {
    $exe = Join-Path $PSScriptRoot "cli\bin\win32-x64\cheaper.exe"
    if (Test-Path -LiteralPath $exe -PathType Leaf) {
        if (Test-LfsPointer $exe) { throw "$exe is a Git LFS pointer; install git-lfs and run git lfs pull or reinstall the package." }
        $exitCode = Invoke-Executable -Path $exe -Arguments $args
    } else {
        $nativeArch = $env:PROCESSOR_ARCHITEW6432
        if (-not $nativeArch) { $nativeArch = $env:PROCESSOR_ARCHITECTURE }
        if ($env:OS -ne "Windows_NT" -or $nativeArch -ine "AMD64") { throw "unsupported acquisition platform: Windows native arch $nativeArch; provide a sibling executable." }
        $versionFile = Join-Path $PSScriptRoot "cli\version.json"
        if (-not (Test-Path -LiteralPath $versionFile -PathType Leaf)) { throw "missing cli/version.json; reinstall version metadata or provide a sibling executable (Copilot JSON deploy limitation)." }
        $versionJson = [IO.File]::ReadAllText($versionFile)
        if ($versionJson -cnotmatch '^\s*\{\s*"version"\s*:\s*"([^"]+)"\s*\}\s*$') { throw "invalid version metadata; expected one version field." }
        $script:version = $Matches[1]
        if ($script:version -cnotmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?(\+[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?$') { throw "invalid version metadata; expected safe SemVer." }
        $script:releaseBase = "https://github.com/scarecrow173/cheaper/releases/download"
        if ($env:CHEAPER_RELEASE_BASE_URL) { $script:releaseBase = $env:CHEAPER_RELEASE_BASE_URL.TrimEnd("/") }
        if (-not (Test-ReleaseUri ([Uri]$script:releaseBase))) { throw "invalid Release origin; use trusted HTTPS or loopback HTTP." }
        $cacheRoot = $env:CHEAPER_CACHE_DIR
        if (-not $cacheRoot) {
            if (-not $env:LOCALAPPDATA) { throw "cache root requires LOCALAPPDATA or CHEAPER_CACHE_DIR." }
            $cacheRoot = Join-Path $env:LOCALAPPDATA "cheaper\cache"
        }
        if ($cacheRoot -notmatch '^(?:[A-Za-z]:[\\/]|\\\\)') { throw "cache root must be an absolute path." }
        $cacheRoot = [IO.Path]::GetFullPath($cacheRoot).TrimEnd("\", "/")
        $hookRoot = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd("\", "/")
        if ($cacheRoot -ieq $hookRoot -or $cacheRoot.StartsWith($hookRoot + "\", [StringComparison]::OrdinalIgnoreCase)) { throw "cache must be outside deployed hook directory; set CHEAPER_CACHE_DIR." }
        # symlink/junction の先へ cache boundary を隠す配置を拒否する。
        $ancestor = $cacheRoot
        while ($ancestor) {
            if (Test-Path -LiteralPath $ancestor) {
                $item = Get-Item -LiteralPath $ancestor -Force
                if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "cache path must not traverse a symlink or junction; choose a plain user directory." }
            }
            $ancestor = [IO.Path]::GetDirectoryName($ancestor)
        }
        $parent = Join-Path $cacheRoot $script:version
        try { [IO.Directory]::CreateDirectory($parent) | Out-Null }
        catch { throw "cannot write cache directory $cacheRoot; set CHEAPER_CACHE_DIR to a writable user directory." }
        $script:key = Join-Path $parent "win32-x64"
        $lock = Join-Path $parent ".win32-x64.lock"
        $watch = [Diagnostics.Stopwatch]::StartNew()
        while (-not $ownsLock) {
            # New-Item のディレクトリ作成は check-then-create で既存パスに対し
            # 非原子に失敗するため、一意名を作成してから同一ボリューム rename で
            # 排他 claim する(mkdir(2) 相当の原子性)。
            $claim = Join-Path $parent (".win32-x64.claim." + [Guid]::NewGuid().ToString("N"))
            try { [IO.Directory]::CreateDirectory($claim) | Out-Null }
            catch { throw "cannot create acquisition lock in cache." }
            try {
                [IO.Directory]::Move($claim, $lock)
                $ownsLock = $true
            } catch {
                Remove-Item -LiteralPath $claim -Recurse -Force -ErrorAction SilentlyContinue
                if ($watch.Elapsed.TotalSeconds -ge 120) { throw "acquisition lock timed out at $lock; verify owner has stopped, then remove its lock and staging directory." }
                Start-Sleep -Seconds 1
            }
        }
        [IO.File]::WriteAllText((Join-Path $lock "owner"), [string]$PID)
        if (Test-Path -LiteralPath $script:key) {
            if (-not (Test-Path -LiteralPath $script:key -PathType Container)) { throw "cache key is not a directory." }
            Confirm-Cache $script:key
        } else {
            $stage = Join-Path $parent (".win32-x64.staging." + [Guid]::NewGuid().ToString("N"))
            New-Item -ItemType Directory -Path $stage -ErrorAction Stop | Out-Null
            Get-ReleaseFile "SHA256SUMS" (Join-Path $stage "SHA256SUMS")
            Get-ReleaseFile "cheaper-windows-x64.exe" (Join-Path $stage "cheaper.exe")
            Confirm-Cache $stage
            if (Test-Path -LiteralPath $script:key) { throw "cache key appeared during acquisition; refusing to overwrite $script:key." }
            [IO.Directory]::Move($stage, $script:key)
            $stage = $null
        }
        Remove-Item -LiteralPath $lock -Recurse -Force
        $ownsLock = $false
        $exitCode = Invoke-Executable -Path (Join-Path $script:key "cheaper.exe") -Arguments $args
    }
} catch {
    [Console]::Error.WriteLine("cheaper launcher: " + $_.Exception.Message)
    $exitCode = 1
} finally {
    if ($stage -and (Test-Path -LiteralPath $stage)) { Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue }
    if ($ownsLock -and (Test-Path -LiteralPath $lock)) { Remove-Item -LiteralPath $lock -Recurse -Force -ErrorAction SilentlyContinue }
}
exit $exitCode
