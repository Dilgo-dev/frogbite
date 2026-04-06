# frogbite installer (Windows PowerShell)
# Usage:
#   irm https://raw.githubusercontent.com/Dilgo-dev/frogbite/main/scripts/install.ps1 | iex
#   $env:FROGBITE_VERSION='v0.1.0'; irm ... | iex

$ErrorActionPreference = 'Stop'

$Repo = 'Dilgo-dev/frogbite'
$Bin  = 'frogbite.exe'
$InstallDir = if ($env:FROGBITE_INSTALL_DIR) {
  $env:FROGBITE_INSTALL_DIR
} else {
  Join-Path $env:LOCALAPPDATA 'Programs\frogbite'
}

function Info($msg) { Write-Host "==> $msg" }

# Detect architecture
$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
  'AMD64' { 'x86_64' }
  'ARM64' { 'aarch64' }
  default { throw "unsupported arch: $($env:PROCESSOR_ARCHITECTURE)" }
}
$target = "$arch-pc-windows-msvc"

# Resolve version
$version = $env:FROGBITE_VERSION
if (-not $version) {
  Info 'resolving latest release'
  $resp = Invoke-WebRequest -UseBasicParsing -MaximumRedirection 0 `
    -ErrorAction SilentlyContinue "https://github.com/$Repo/releases/latest"
  $loc = $resp.Headers.Location
  if (-not $loc) { throw 'failed to resolve latest version' }
  $version = $loc -replace '.*/tag/',''
}
Info "installing frogbite $version ($target)"

$archive = "frogbite-$version-$target.zip"
$url     = "https://github.com/$Repo/releases/download/$version/$archive"
$sumUrl  = "$url.sha256"

$tmp = Join-Path ([IO.Path]::GetTempPath()) ([IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Info "downloading $archive"
  Invoke-WebRequest -UseBasicParsing -Uri $url    -OutFile (Join-Path $tmp $archive)
  Invoke-WebRequest -UseBasicParsing -Uri $sumUrl -OutFile (Join-Path $tmp "$archive.sha256")

  Info 'verifying checksum'
  $expected = (Get-Content (Join-Path $tmp "$archive.sha256") -Raw).Split()[0].ToLower()
  $actual   = (Get-FileHash (Join-Path $tmp $archive) -Algorithm SHA256).Hash.ToLower()
  if ($expected -ne $actual) { throw "checksum mismatch: expected $expected got $actual" }

  Info 'extracting'
  Expand-Archive -Force -Path (Join-Path $tmp $archive) -DestinationPath $tmp

  if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir | Out-Null
  }
  $src = Join-Path $tmp "frogbite-$version-$target\$Bin"
  Move-Item -Force $src (Join-Path $InstallDir $Bin)

  Info "installed $Bin -> $InstallDir\$Bin"

  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  if (($userPath -split ';') -notcontains $InstallDir) {
    Write-Host ''
    Write-Host "Note: $InstallDir is not in your PATH."
    Write-Host 'Add it permanently with:'
    Write-Host "  [Environment]::SetEnvironmentVariable('Path', `"$userPath;$InstallDir`", 'User')"
  }
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
