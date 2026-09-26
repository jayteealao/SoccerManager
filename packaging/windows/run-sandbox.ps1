# Runs the clean-machine check in Windows Sandbox: a fresh Windows 11 image at every start.
#
# Run after build.ps1:  pwsh packaging/windows/run-sandbox.ps1
#
# It maps dist/ read-only into the sandbox, maps a new evidence folder
# dist/evidence/windows/<utc>/ writable, starts smoke.ps1 at logon with networking off, waits
# up to ten minutes for results.json, prints the verdict, and exits with the check's code
# (0 pass, 1 fail, 2 no results in time, 3 Windows Sandbox not available).

param(
    [int]$TimeoutMinutes = 10
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$sandbox = Join-Path $env:SystemRoot 'System32\WindowsSandbox.exe'
if (-not (Test-Path $sandbox)) {
    Write-Error 'Windows Sandbox is not enabled; see packaging/README.md for the Hyper-V fallback' -ErrorAction Continue
    exit 3
}

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$dist = Join-Path $repo 'dist'
$setup = Get-ChildItem -Path $dist -Filter 'SoccerManager-*-windows-x64-setup.exe' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $setup) { throw "no setup file in $dist; run packaging/windows/build.ps1 first" }
if (-not (Test-Path (Join-Path $dist 'smoke.ps1'))) { throw "no smoke.ps1 in $dist; run packaging/windows/build.ps1 first" }

$stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ')
$evidence = Join-Path $dist "evidence\windows\$stamp"
New-Item -ItemType Directory -Force -Path $evidence | Out-Null

# The sandbox reads absolute host paths only.
$command = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\smoke\in\smoke.ps1 " +
    "-Setup C:\smoke\in\$($setup.Name) -Evidence C:\smoke\out"
$wsb = @"
<Configuration>
  <Networking>Disable</Networking>
  <MemoryInMB>4096</MemoryInMB>
  <MappedFolders>
    <MappedFolder>
      <HostFolder>$dist</HostFolder>
      <SandboxFolder>C:\smoke\in</SandboxFolder>
      <ReadOnly>true</ReadOnly>
    </MappedFolder>
    <MappedFolder>
      <HostFolder>$evidence</HostFolder>
      <SandboxFolder>C:\smoke\out</SandboxFolder>
      <ReadOnly>false</ReadOnly>
    </MappedFolder>
  </MappedFolders>
  <LogonCommand>
    <Command>$command</Command>
  </LogonCommand>
</Configuration>
"@
$wsbFile = Join-Path $dist 'smoke.wsb'
Set-Content -Encoding utf8 -Path $wsbFile -Value $wsb

Write-Output "starting Windows Sandbox; evidence goes to $evidence"
Start-Process -FilePath $wsbFile

$results = Join-Path $evidence 'results.json'
$deadline = (Get-Date).AddMinutes($TimeoutMinutes)
while ((Get-Date) -lt $deadline -and -not (Test-Path $results)) { Start-Sleep -Seconds 5 }
if (-not (Test-Path $results)) {
    Write-Error "no results.json in $evidence after $TimeoutMinutes minutes" -ErrorAction Continue
    exit 2
}
# The file is written in one go at the end of the run; give the writer a moment to close it.
Start-Sleep -Seconds 2
$report = Get-Content -Raw $results | ConvertFrom-Json
foreach ($check in $report.checks) {
    $word = if ($check.PSObject.Properties['fact']) { 'FACT' } elseif ($check.pass) { 'PASS' } else { 'FAIL' }
    Write-Output "[$word] $($check.name) - $($check.detail)"
}
Write-Output "overall: $(if ($report.pass) { 'PASS' } else { 'FAIL' }) ($results)"
Write-Output 'close the Windows Sandbox window to discard the image'
if ($report.pass) { exit 0 } else { exit 1 }
