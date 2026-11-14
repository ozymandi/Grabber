<#
.SYNOPSIS
Builds, signs and publishes a ripper release to GitHub (design D8 of add-installer-updater).

.DESCRIPTION
Reads the version from src-tauri/Cargo.toml, builds the NSIS installer with its updater
signature, writes latest.json for tauri-plugin-updater and publishes all three files as
release v<version> on ozymandi/Grabber.

Needs the signing key in TAURI_SIGNING_PRIVATE_KEY (path) and its password in
TAURI_SIGNING_PRIVATE_KEY_PASSWORD, as process or user environment variables.

.PARAMETER DryRun
Build and write latest.json, then stop before publishing. A dirty working tree or an
unpushed commit is reported but does not stop a dry run.

.PARAMETER Yes
Publish without the interactive confirmation (for when the go-ahead was given elsewhere).

.PARAMETER Notes
Release notes for GitHub and latest.json. Defaults to "ripper <version>".
#>
param(
    [switch]$DryRun,
    [switch]$Yes,
    [string]$Notes
)

$ErrorActionPreference = 'Stop'
$repo = 'ozymandi/Grabber'
$root = Split-Path $PSScriptRoot -Parent
$crate = Join-Path $root 'src-tauri'

function Fail($message) {
    Write-Host "release: $message" -ForegroundColor Red
    exit 1
}

# 1. Preconditions -------------------------------------------------------------------------

# Signing variables: the process may predate a user variable set in Windows, so fall back
# to the user scope and pass it on to the build. Values are never printed.
foreach ($name in 'TAURI_SIGNING_PRIVATE_KEY', 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD') {
    $value = [Environment]::GetEnvironmentVariable($name, 'Process')
    if (-not $value) { $value = [Environment]::GetEnvironmentVariable($name, 'User') }
    if (-not $value) { Fail "$name is not set. Set it in your user environment variables and try again." }
    [Environment]::SetEnvironmentVariable($name, $value, 'Process')
}
$keyPath = $env:TAURI_SIGNING_PRIVATE_KEY
if ((Test-Path -LiteralPath $keyPath -PathType Leaf) -eq $false -and $keyPath.Length -lt 200) {
    Fail "TAURI_SIGNING_PRIVATE_KEY points to a file that does not exist: $keyPath"
}

$match = Select-String -LiteralPath (Join-Path $crate 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $match) { Fail 'cannot read the version from src-tauri/Cargo.toml' }
$version = $match.Matches[0].Groups[1].Value
$tag = "v$version"
if (-not $Notes) { $Notes = "ripper $version" }

$problems = @()
if (git -C $root status --porcelain --untracked-files=no) { $problems += 'tracked files have uncommitted changes' }
git -C $root fetch --quiet origin
if (-not (git -C $root branch -r --contains HEAD)) { $problems += 'HEAD is not pushed to origin' }
if (git -C $root tag --list $tag) { $problems += "tag $tag already exists locally" }
gh release view $tag -R $repo *> $null
if ($LASTEXITCODE -eq 0) { $problems += "release $tag already exists on $repo" }
if ($problems) {
    if ($DryRun) { $problems | ForEach-Object { Write-Host "release: warning: $_" -ForegroundColor Yellow } }
    else { Fail ($problems -join '; ') }
}

# 2. Build -----------------------------------------------------------------------------------

Write-Host "release: building ripper $version"
Push-Location $crate
try {
    cargo tauri build
    if ($LASTEXITCODE -ne 0) { Fail 'cargo tauri build failed' }
    $target = (cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).target_directory
} finally {
    Pop-Location
}

# 3. Collect ---------------------------------------------------------------------------------

$bundle = Join-Path $target 'release\bundle\nsis'
$setupName = "ripper_${version}_x64-setup.exe"
$setup = Join-Path $bundle $setupName
$sig = "$setup.sig"
foreach ($f in $setup, $sig) {
    if (-not (Test-Path -LiteralPath $f)) { Fail "build output missing: $f" }
}

# 4. latest.json -----------------------------------------------------------------------------

$latest = Join-Path $bundle 'latest.json'
$doc = [ordered]@{
    version   = $version
    notes     = $Notes
    pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-dd'T'HH:mm:ss'Z'")
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = (Get-Content -LiteralPath $sig -Raw).Trim()
            url       = "https://github.com/$repo/releases/download/$tag/$setupName"
        }
    }
}
# UTF-8 without BOM: the updater parses the bytes as JSON.
[IO.File]::WriteAllText($latest, ($doc | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))

Write-Host "release: ready in $bundle"
Write-Host "  $setupName"
Write-Host "  $setupName.sig"
Write-Host '  latest.json'

if ($DryRun) {
    Write-Host 'release: dry run, nothing published'
    exit 0
}

# 5. Publish ---------------------------------------------------------------------------------

if (-not $Yes) {
    $answer = Read-Host "Publish $tag to https://github.com/$repo (public)? Type yes to continue"
    if ($answer -ne 'yes') { Fail 'not published' }
}
gh release create $tag $setup $sig $latest -R $repo --target (git -C $root rev-parse HEAD) --title "ripper $version" --notes $Notes
if ($LASTEXITCODE -ne 0) { Fail 'gh release create failed' }
git -C $root fetch --quiet --tags origin
Write-Host "release: published https://github.com/$repo/releases/tag/$tag"
Write-Host "release: updater endpoint https://github.com/$repo/releases/latest/download/latest.json"
