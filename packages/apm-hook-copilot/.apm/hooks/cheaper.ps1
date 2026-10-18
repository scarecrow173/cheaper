# PATH 上の Cargo install 済み cheaper を起動する thin launcher。
# 契約: ADR-034Dmb3kNAO4Ub6FxuQCtC。
$ErrorActionPreference = "Stop"
$cli = Get-Command cheaper, cheaper.exe -CommandType Application -ErrorAction SilentlyContinue |
    Select-Object -First 1
if (-not $cli -or -not $cli.Source) {
    [Console]::Error.WriteLine("cheaper launcher: 'cheaper' CLI is not installed or not on PATH.")
    [Console]::Error.WriteLine("install: cargo install --git https://github.com/scarecrow173/cheaper.git --locked cheaper-cli")
    [Console]::Error.WriteLine("then ensure %USERPROFILE%\.cargo\bin is on the IDE/agent PATH and restart it.")
    exit 1
}
function Quote-Arg([string]$a) {
    if ($a -notmatch '[\s"]') { return $a }
    return '"' + ($a -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"'
}
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $cli.Source
$psi.UseShellExecute = $false
$psi.Arguments = ($args | ForEach-Object { Quote-Arg $_ }) -join ' '
try {
    $proc = [System.Diagnostics.Process]::Start($psi)
    $proc.WaitForExit()
    exit $proc.ExitCode
} catch {
    [Console]::Error.WriteLine("cheaper launcher: failed to start cheaper: " + $_.Exception.Message)
    exit 1
}
