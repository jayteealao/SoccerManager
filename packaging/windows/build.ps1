# Builds the Windows release: the engine, the built viewer, a staged folder, the setup file,
# and its hash. It needs Rust, Node 22.12 or later with npm, NSIS 3, and git with the previous
# release's tag fetched (the previous engine, shipped as previous\, is built from it).
#
# Run from anywhere:  pwsh packaging/windows/build.ps1
#                     pwsh packaging/windows/build.ps1 -StageOnly   (stops after dist/stage)
# Writes:             dist/SoccerManager-<version>-windows-x64-setup.exe
#                     dist/SoccerManager-<version>-windows-x64-setup.exe.sha256
#                     dist/smoke.ps1 (the clean-machine check, for run-sandbox.ps1)
#                     dist/previous-engine.json (the pin smoke.ps1 checks previous\ against)
#
# The version is read from the built program's --version, so the setup file, the installed
# program, and the engine's hello message always name the same version.

param(
    # Stop once dist/stage holds the installed layout, before the setup file: for the browser
    # checks against the packaged folder, which need no NSIS.
    [switch]$StageOnly
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$dist = Join-Path $repo 'dist'

# The viewer build needs Node; say so before the long engine build.
if (-not (Get-Command node -ErrorAction SilentlyContinue) -or -not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw 'node or npm is not on PATH; Node 22.12 or later is needed to build the viewer'
}

Push-Location $repo
try {
    cargo build --release --locked -p engine-cli
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
} finally {
    Pop-Location
}

# The previous release's engine, built from its tag (or reused from the cache), ships beside
# this one as previous\, so a match that release saved finishes on the engine that started it.
$previous = & (Join-Path $PSScriptRoot 'previous-engine.ps1') | Select-Object -Last 1
if (-not $previous -or -not (Test-Path (Join-Path $previous 'engine-cli.exe'))) {
    throw "the previous release's engine was not built (the script printed '$previous')"
}

# The page the release carries is the viewer's release build: the viewer and the handshake
# page, with the font licence texts, the open-source notices (this program's crates, the
# previous engine's crates, the viewer's packages and the fonts), and no test page. The build
# fails when a shipped package has no licence text or no allowed licence; the check then reads
# the written file again from the sources.
$env:SM_PREVIOUS_NOTICES = Join-Path $previous 'notices-crates.json'
Push-Location (Join-Path $repo 'viewer')
try {
    npm ci --no-audit --no-fund
    if ($LASTEXITCODE -ne 0) { throw "npm ci failed with exit code $LASTEXITCODE" }
    npm run build:release
    if ($LASTEXITCODE -ne 0) { throw "the viewer build failed with exit code $LASTEXITCODE" }
    npm run notices:verify -- dist-release/notices.json
    if ($LASTEXITCODE -ne 0) { throw "the open-source notices check failed with exit code $LASTEXITCODE" }
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
Copy-Item -Recurse (Join-Path $repo 'viewer\dist-release') (Join-Path $stage 'web')
Copy-Item -Recurse $previous (Join-Path $stage 'previous')
Copy-Item (Join-Path $repo 'LICENSE-MIT') $stage
Copy-Item (Join-Path $repo 'LICENSE-APACHE') $stage
if ($StageOnly) {
    Write-Output $stage
    return
}

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
# The installer's numeric version field cannot hold a prerelease suffix such as -rc.1.
$numeric = ($version -split '-')[0]
& $makensis /V2 "/DVERSION=$version" "/DNUMERIC_VERSION=$numeric" "/DSTAGE=$stage" "/DOUTFILE=$setup" $script
if ($LASTEXITCODE -ne 0) { throw "makensis failed with exit code $LASTEXITCODE" }

$hash = (Get-FileHash -Algorithm SHA256 $setup).Hash.ToLowerInvariant()
Set-Content -NoNewline -Encoding ascii -Path "$setup.sha256" -Value "$hash  $setupName`n"
Copy-Item (Join-Path $PSScriptRoot 'smoke.ps1') $dist -Force
Copy-Item (Join-Path $repo 'packaging\previous-engine.json') $dist -Force

Write-Output $setup
