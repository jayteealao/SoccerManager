# Builds the previous release's engine from its git tag, so a saved match from that release
# can finish on the engine that started it. The release ships the result as `previous\`:
# `previous\engine-cli.exe` and the tag's own `previous\content\`.
#
# Run from anywhere:  pwsh packaging/windows/previous-engine.ps1
# The pin (tag, commit, version) is packaging/previous-engine.json. The cache folder is
# $env:SM_PREVIOUS_CACHE, or target\previous-engine in the repository; the script prints the
# path of the finished `previous\` folder and reuses it while its program prints the pinned
# version.
#
# The tag is built in a detached git worktree with `cargo build --locked`, and the worktree
# is removed in every case; the checkout this script runs from is not touched.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$pinPath = Join-Path $repo 'packaging\previous-engine.json'
$pin = Get-Content -Raw $pinPath | ConvertFrom-Json
$tag = [string]$pin.tag
$commit = [string]$pin.commit
$version = [string]$pin.version
if (-not $tag -or -not $commit -or -not $version) {
    throw "$pinPath must name a tag, a commit and a version"
}

$cache = if ($env:SM_PREVIOUS_CACHE) { $env:SM_PREVIOUS_CACHE } else { Join-Path $repo 'target\previous-engine' }
$out = Join-Path $cache 'previous'
$program = Join-Path $out 'engine-cli.exe'

function Get-PrintedVersion([string]$exe) {
    try {
        $printed = (& $exe --version 2>$null) -join ' '
    } catch {
        return ''
    }
    return ($printed -split '\s+')[1]
}

if ((Test-Path $program) -and (Test-Path (Join-Path $out 'content')) -and ((Get-PrintedVersion $program) -eq $version)) {
    Write-Output $out
    return
}

$found = (& git -C $repo rev-parse --verify --quiet "refs/tags/$tag^{commit}" 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $found) {
    throw "the tag $tag is not in this clone; fetch it with: git fetch --no-tags origin tag $tag"
}
if ($found.Trim() -ne $commit) {
    throw "the tag $tag points at $($found.Trim()), but $pinPath pins $commit"
}

New-Item -ItemType Directory -Force -Path $cache | Out-Null
$checkout = Join-Path $cache 'checkout'
function Remove-Checkout {
    if (Test-Path $checkout) {
        & git -C $repo worktree remove --force $checkout 2>$null | Out-Null
        if (Test-Path $checkout) { Remove-Item -Recurse -Force $checkout }
    }
    & git -C $repo worktree prune 2>$null | Out-Null
    $global:LASTEXITCODE = 0
}

Remove-Checkout
try {
    & git -C $repo worktree add --detach $checkout $commit | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "cannot check out $tag ($commit) in a worktree" }
    Push-Location $checkout
    try {
        cargo build --release --locked -p engine-cli --target-dir (Join-Path $cache 'target')
        if ($LASTEXITCODE -ne 0) { throw "the build of $tag failed with exit code $LASTEXITCODE" }
    } finally {
        Pop-Location
    }

    # A build from a changed tree carries a `-dirty` build hash, and its saves would match no
    # released build.
    $changed = (& git -C $checkout status --porcelain --untracked-files=no) -join "`n"
    if ($changed) { throw "the worktree of $tag changed during the build; refusing a dirty build" }

    if (Test-Path $out) { Remove-Item -Recurse -Force $out }
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    Copy-Item (Join-Path $cache 'target\release\engine-cli.exe') $program
    Copy-Item -Recurse (Join-Path $checkout 'content') (Join-Path $out 'content')
} finally {
    Remove-Checkout
}

$got = Get-PrintedVersion $program
if ($got -ne $version) {
    throw "the program built from $tag prints version '$got', not $version"
}

Write-Output $out
