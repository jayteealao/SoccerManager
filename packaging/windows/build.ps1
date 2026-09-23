# Builds the Windows release: the engine, a staged folder, the setup file, and its hash.
#
# Run from anywhere:  pwsh packaging/windows/build.ps1
# Writes:             dist/SoccerManager-<version>-windows-x64-setup.exe
#                     dist/SoccerManager-<version>-windows-x64-setup.exe.sha256
#                     dist/smoke.ps1 (the clean-machine check, for run-sandbox.ps1)
#
# The version is read from the built program's --version, so the setup file, the installed
# program, and the engine's hello message always name the same version.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$dist = Join-Path $repo 'dist'

Push-Location $repo
try {
    cargo build --release --locked -p engine-cli
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
} finally {
    Pop-Location
}

$exe = Join-Path $repo 'target\release\engine-cli.exe'
$printed = (& $exe --version) -join ' '
$version = ($printed -split '\s+')[1]
if ([string]::IsNullOrWhiteSpace($version)) {
    throw "cannot read a version from '$exe --version' (printed '$printed')"
}

# The staged folder is exactly what the installer copies.
$stage = Join-Path $dist 'stage'
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item $exe $stage
Copy-Item -Recurse (Join-Path $repo 'content') (Join-Path $stage 'content')
Copy-Item -Recurse (Join-Path $repo 'web') (Join-Path $stage 'web')
# The page's own tests are not part of the game.
Remove-Item -Recurse -Force (Join-Path $stage 'web\tests')
Copy-Item (Join-Path $repo 'LICENSE-MIT') $stage
Copy-Item (Join-Path $repo 'LICENSE-APACHE') $stage

$makensis = $null
$found = Get-Command makensis -ErrorAction SilentlyContinue
if ($found) { $makensis = $found.Source }
if (-not $makensis) {
    $candidate = Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe'
    if (Test-Path $candidate) { $makensis = $candidate }
}
if (-not $makensis) {
    throw 'NSIS 3 is not installed: makensis is neither on PATH nor in Program Files (x86)\NSIS'
}

$setupName = "SoccerManager-$version-windows-x64-setup.exe"
$setup = Join-Path $dist $setupName
$script = Join-Path $PSScriptRoot 'installer.nsi'
& $makensis /V2 "/DVERSION=$version" "/DSTAGE=$stage" "/DOUTFILE=$setup" $script
if ($LASTEXITCODE -ne 0) { throw "makensis failed with exit code $LASTEXITCODE" }

$hash = (Get-FileHash -Algorithm SHA256 $setup).Hash.ToLowerInvariant()
Set-Content -NoNewline -Encoding ascii -Path "$setup.sha256" -Value "$hash  $setupName`n"
Copy-Item (Join-Path $PSScriptRoot 'smoke.ps1') $dist -Force

Write-Output $setup
