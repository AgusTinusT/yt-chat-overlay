# Membuat paket MSIX untuk Microsoft Store dari exe hasil build.
# Pemakaian:
#   cargo build --release
#   .\packaging\build-msix.ps1 -Exe "$env:LOCALAPPDATA\ytovl\release\yt-chat-overlay.exe"
# Hasil: dist\YT-Chat-Overlay-<versi>-x64.msix (belum ditandatangani; Store yang menandatangani).
param(
    [string]$Exe = "target\release\yt-chat-overlay.exe",
    [string]$OutDir = "dist"
)
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

# Versi dari Cargo.toml. Store mewajibkan angka ke-4 = 0 (0.1.2 -> 0.1.2.0).
$cargo = Get-Content (Join-Path $root "Cargo.toml") -Raw
if ($cargo -notmatch '(?m)^version\s*=\s*"(\d+)\.(\d+)\.(\d+)"') { throw "Versi tidak ditemukan di Cargo.toml" }
$version = "$($Matches[1]).$($Matches[2]).$($Matches[3]).0"

$id = Get-Content (Join-Path $PSScriptRoot "identity.json") -Raw | ConvertFrom-Json

if (-not (Test-Path $Exe)) { throw "Exe tidak ditemukan: $Exe (jalankan cargo build --release dulu)" }

# makeappx.exe dari Windows SDK (versi terbaru)
$makeappx = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" -ErrorAction SilentlyContinue |
    Sort-Object { [version]($_.Directory.Parent.Name) } | Select-Object -Last 1
if (-not $makeappx) { throw "makeappx.exe tidak ditemukan. Install Windows SDK." }

$stage = Join-Path ([IO.Path]::GetTempPath()) "ytovl-msix-stage"
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force (Join-Path $stage "Assets") | Out-Null

Copy-Item $Exe (Join-Path $stage "YT-Chat-Overlay.exe")
Copy-Item (Join-Path $PSScriptRoot "Assets\*.png") (Join-Path $stage "Assets")

$manifest = (Get-Content (Join-Path $PSScriptRoot "AppxManifest.xml") -Raw).
    Replace("{{IDENTITY_NAME}}", $id.IdentityName).
    Replace("{{PUBLISHER}}", [Security.SecurityElement]::Escape($id.Publisher)).
    Replace("{{PUBLISHER_DISPLAY_NAME}}", [Security.SecurityElement]::Escape($id.PublisherDisplayName)).
    Replace("{{DISPLAY_NAME}}", [Security.SecurityElement]::Escape($id.DisplayName)).
    Replace("{{VERSION}}", $version)
[IO.File]::WriteAllText((Join-Path $stage "AppxManifest.xml"), $manifest, (New-Object Text.UTF8Encoding $false))

New-Item -ItemType Directory -Force $OutDir | Out-Null
$out = Join-Path (Resolve-Path $OutDir) "YT-Chat-Overlay-$version-x64.msix"
& $makeappx.FullName pack /d $stage /p $out /o
if ($LASTEXITCODE -ne 0) { throw "makeappx gagal ($LASTEXITCODE)" }
Write-Host "MSIX: $out (versi $version)"
