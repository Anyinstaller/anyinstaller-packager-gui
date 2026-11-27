$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Dist = Join-Path $Root "dist"
$Gui = Join-Path $Root "target/release/anyinstaller-packager-gui.exe"
$Packager = Join-Path (Split-Path -Parent $Root) "packager/target/release/packager.exe"

cargo build --release --manifest-path (Join-Path $Root "Cargo.toml")
if ($LASTEXITCODE -ne 0) { throw "GUI build failed with exit code $LASTEXITCODE" }

cargo build --release --manifest-path (Join-Path (Split-Path -Parent $Root) "packager/Cargo.toml")
if ($LASTEXITCODE -ne 0) { throw "packager build failed with exit code $LASTEXITCODE" }

New-Item -ItemType Directory -Force -Path $Dist | Out-Null
Copy-Item -LiteralPath $Gui -Destination (Join-Path $Dist "anyinstaller-packager-gui.exe") -Force
Copy-Item -LiteralPath $Packager -Destination (Join-Path $Dist "packager.exe") -Force

Write-Host "Built $Dist"
Write-Host "  anyinstaller-packager-gui.exe"
Write-Host "  packager.exe"
