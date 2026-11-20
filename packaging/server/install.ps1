# Installs industrial-era-server (docs/server.md) on Windows: the current Industrial Era Nightly (the only build Industrial Era ships;
# GitHub's latest release while there is none), downloaded from GitHub Releases and checked against the SHA-256
# GitHub publishes for it.
#
#   irm https://raw.githubusercontent.com/industrial-era-rust/industrial-era/main/packaging/server/install.ps1 | iex
#   & ([scriptblock]::Create((irm .../install.ps1))) -Version v0.5.0 -Dir D:\Industrial Era
#
# Releases go to <Dir>\releases\<version>, <Dir>\current points at the one in use, and <Dir>\bin (added to
# your PATH) holds industrial-era-server.cmd and industrial-era-prepare.cmd. The game files you prepare from your own
# Minecraft stay in <Dir>\game-files across updates. Nothing of Mojang's is downloaded. No admin rights.
param(
    [string]$Version = "latest",
    [string]$Dir = (Join-Path $env:LOCALAPPDATA "Industrial Era")
)
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$Repo = "industrial-era-rust/industrial-era"
$Asset = "industrial-era-windows-x64.zip"
if ($env:PROCESSOR_ARCHITECTURE -ne "AMD64") {
    throw "No Industrial Era release for Windows on $env:PROCESSOR_ARCHITECTURE yet: build it from source (docs/server.md)"
}

function Get-Release([string]$Path) {
    Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/$Path" -Headers @{ Accept = "application/vnd.github+json" }
}
if ($Version -eq "latest") {
    try { $Release = Get-Release "tags/nightly" } catch { $Release = Get-Release "latest" }
} else {
    $Release = Get-Release "tags/$Version"
}
$Tag = $Release.tag_name
# A Nightly installs under its date and commit, as `industrial-era-server update` names it:
# "Nightly 2026-10-28 <middle dot> 0c02067" is nightly-2026-10-28-0c02067 ([char]0xB7: this file stays ASCII).
if (($Tag -eq "nightly" -or $Tag.StartsWith("nightly-")) -and
    $Release.name -match "^Nightly (\d{4}-\d{2}-\d{2}) $([char]0xB7) ([0-9a-f]{1,40})$") {
    $Tag = "nightly-$($Matches[1])-$($Matches[2])"
}
$Item = $Release.assets | Where-Object { $_.name -eq $Asset } | Select-Object -First 1
if (-not $Item) { throw "Release $Tag has no $Asset" }
if (-not $Item.digest -or -not $Item.digest.StartsWith("sha256:")) { throw "Release $Tag publishes no SHA-256 for $Asset" }
$Digest = $Item.digest.Substring(7)

$Tmp = Join-Path ([IO.Path]::GetTempPath()) ("industrial-era-install-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $Tmp | Out-Null
try {
    Write-Host "Downloading Industrial Era $Tag ($Asset)..."
    $Zip = Join-Path $Tmp $Asset
    Invoke-WebRequest -Uri $Item.browser_download_url -OutFile $Zip
    $Got = (Get-FileHash -Algorithm SHA256 $Zip).Hash.ToLowerInvariant()
    if ($Got -ne $Digest.ToLowerInvariant()) {
        throw "$Asset doesn't match its SHA-256 (got $Got, expected $Digest): not installed"
    }
    $Unpacked = Join-Path $Tmp "x"
    Expand-Archive -Path $Zip -DestinationPath $Unpacked
    $Server = Get-ChildItem -Path $Unpacked -Recurse -Filter "industrial-era-server.exe" | Select-Object -First 1
    if (-not $Server) { throw "$Asset holds no industrial-era-server.exe (a release from before it existed?)" }

    $Releases = Join-Path $Dir "releases"
    $GameFiles = Join-Path $Dir "game-files\industrial-era-client"
    $Bin = Join-Path $Dir "bin"
    New-Item -ItemType Directory -Force -Path $Releases, $GameFiles, $Bin | Out-Null
    $Target = Join-Path $Releases $Tag
    if (Test-Path $Target) { Remove-Item -Recurse -Force $Target }
    Move-Item -Path $Server.Directory.FullName -Destination $Target
    # Prepared game files live outside the release, so updates and rollbacks keep them. Junctions need no
    # admin rights, unlike symbolic links.
    $Local = Join-Path $Target ".local"
    New-Item -ItemType Directory -Force -Path $Local | Out-Null
    New-Item -ItemType Junction -Path (Join-Path $Local "industrial-era-client") -Target $GameFiles | Out-Null
    $Current = Join-Path $Dir "current"
    if (Test-Path $Current) { (Get-Item $Current).Delete() }
    New-Item -ItemType Junction -Path $Current -Target $Target | Out-Null
    foreach ($Tool in "industrial-era-server", "industrial-era-prepare") {
        Set-Content -Path (Join-Path $Bin "$Tool.cmd") -Encoding ASCII -Value "@`"$Current\$Tool.exe`" %*"
    }
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not (($UserPath -split ";") -contains $Bin)) {
        [Environment]::SetEnvironmentVariable("Path", ($UserPath.TrimEnd(";") + ";" + $Bin).TrimStart(";"), "User")
        Write-Host "Added $Bin to your PATH: open a new terminal to use it."
    }
    Write-Host "Installed Industrial Era $Tag in $Target."
    if (-not (Get-ChildItem -Path $GameFiles -ErrorAction SilentlyContinue)) {
        Write-Host "Next, prepare the game files from your own Minecraft 26.3 (it needs Java 25 and the game installed"
        Write-Host "once by the official launcher; see docs/server.md):"
        Write-Host "  industrial-era-prepare --prepare --server-jar C:\path\to\server.jar"
    }
    Write-Host "Then set up a server: mkdir my-server; cd my-server; industrial-era-server init"
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
