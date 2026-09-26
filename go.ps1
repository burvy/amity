<#
    Build/run launcher.

        .\go.ps1            -> menu
        .\go.ps1 dev        -> server + client, offline, no deploy
        .\go.ps1 release    -> deploy\ containing shooter-server.exe and dist\

    dev never touches the network and release never enables dev-local, so the
    two paths cannot contaminate each other.
#>
param([ValidateSet('dev', 'release')][string] $Mode)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

$site = Join-Path (Split-Path $PSScriptRoot -Parent) 'burvy-dev'

if (-not $Mode) {
    Write-Host ''
    Write-Host '  1. dev      server + client locally, offline'
    Write-Host '  2. release  build deploy\ for the live server'
    Write-Host ''
    $Mode = switch (Read-Host 'Choose') {
        '1' { 'dev' }
        '2' { 'release' }
        default { throw 'Pick 1 or 2.' }
    }
}

function Invoke-Step($Description, [scriptblock] $Command) {
    Write-Host "`n==> $Description" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Description failed" }
}

# Ask cargo where target/ is rather than hardcoding it, so moving target-dir
# again (as we did to dodge the Windows command-line limit) doesn't break this.
$targetDir = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).target_directory

if ($Mode -eq 'dev') {
    # Build both before launching anything: `cargo run` would start the client
    # while the server is still compiling, and it would fail to connect.
    Invoke-Step 'building server' { cargo build -p shooter-server --features dev-local }
    Invoke-Step 'building client' { cargo build --features dev-local }

    # Run the built exe directly: the new window needs no cargo on PATH and
    # doesn't contend with the client's `cargo run` for the build lock.
    $server = Join-Path $targetDir 'x86_64-pc-windows-msvc\debug\shooter-server.exe'
    Write-Host "`n==> starting server in a new window" -ForegroundColor Cyan
    Start-Process pwsh -WorkingDirectory $PSScriptRoot -ArgumentList @(
        '-NoExit', '-Command', "& '$server'"
    )

    # Client runs here so Ctrl+C returns you to a prompt; the server window
    # keeps its logs and outlives the client.
    Write-Host '==> starting client (Ctrl+C to stop, server window stays open)' -ForegroundColor Cyan
    cargo run --features dev-local
    exit
}

# RELEASE
$exe = Join-Path $targetDir 'x86_64-pc-windows-msvc\release\shooter-server.exe'

Invoke-Step 'building server (release)' { cargo build --release -p shooter-server }
if (-not (Test-Path $exe)) { throw "expected the server at $exe" }

# Only shooter-wasm: build.ps1 clears just what it rebuilds, and trunk's
# copy-dir picks up life/ and game/ from assets\ as they already stand.
Invoke-Step 'building site + shooter wasm' { & (Join-Path $site 'build.ps1') shooter-wasm }

$out = Join-Path $PSScriptRoot 'deploy'
Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $out | Out-Null
Copy-Item $exe $out
Copy-Item (Join-Path $site 'dist') $out -Recurse

Write-Host "`ndeploy\ ready:" -ForegroundColor Green
Write-Host '  shooter-server.exe  -> C:\Users\Burvy\Desktop\'
Write-Host '  dist\               -> C:\Users\Burvy\Desktop\burvy-dev\dist'
Write-Host '  then restart shooter-server.exe (nginx needs no reload)'
Invoke-Item $out
