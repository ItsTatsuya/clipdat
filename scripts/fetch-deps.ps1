# Downloads the shared FFmpeg build into bin/ (+ its SDK into build/ffmpeg-sdk). Then run build-mpv.ps1.
# Usage: pwsh scripts/fetch-deps.ps1 [-Force]
param([switch]$Force)
$ErrorActionPreference = 'Stop'
$root   = Split-Path -Parent $PSScriptRoot
$bin    = Join-Path $root 'bin'
$work   = Join-Path ([IO.Path]::GetTempPath()) ("cliper-deps-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $bin, $work | Out-Null

function Get-Asset($repo, $pattern) {
  $rel = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
  $a = $rel.assets | Where-Object { $_.name -match $pattern } | Select-Object -First 1
  if (-not $a) { throw "No asset matching $pattern in $($rel.tag_name)" }
  return $a
}

function Fetch($repo, $pattern, $exes, [switch]$WithDlls) {
  $need = $exes | Where-Object { $Force -or -not (Test-Path (Join-Path $bin "$_.exe")) }
  # A shared build is only complete if its DLLs are there too.
  if ($WithDlls -and -not (Get-ChildItem $bin -Filter 'avcodec-*.dll' -ErrorAction SilentlyContinue)) { $need = $exes }
  if (-not $need) { Write-Host "up to date: $($exes -join ', ')"; return }
  $a = Get-Asset $repo $pattern
  $dir = Join-Path $work ([IO.Path]::GetFileNameWithoutExtension($a.name))
  New-Item -ItemType Directory -Force $dir | Out-Null
  $arc = Join-Path $work $a.name
  Write-Host "downloading $($a.name) ($([math]::Round($a.size/1MB)) MB)"
  Invoke-WebRequest $a.browser_download_url -OutFile $arc
  tar -xf $arc -C $dir
  foreach ($e in $exes) {
    $src = Get-ChildItem $dir -Recurse -Filter "$e.exe" | Select-Object -First 1
    if (-not $src) { Write-Warning "$e.exe not found in $($a.name)"; continue }
    Copy-Item $src.FullName (Join-Path $bin "$e.exe") -Force
    Write-Host "  -> bin\$e.exe"
  }
  if ($WithDlls) {
    $exeDir = (Get-ChildItem $dir -Recurse -Filter "$($exes[0]).exe" | Select-Object -First 1).DirectoryName
    Get-ChildItem $bin -Filter '*.dll' | Remove-Item -Force   # drop DLLs from an older FFmpeg version
    $dlls = Get-ChildItem $exeDir -Filter '*.dll'
    $dlls | Copy-Item -Destination $bin -Force
    Write-Host "  -> bin\ ($($dlls.Count) shared libraries)"
    # Headers + import libs from the same build, so mpv links against exactly these DLLs.
    $sdk = Join-Path $root 'build\ffmpeg-sdk'
    if (Test-Path $sdk) { Remove-Item -Recurse -Force $sdk }
    New-Item -ItemType Directory -Force $sdk | Out-Null
    $pkg = Split-Path $exeDir -Parent
    Copy-Item (Join-Path $pkg 'include'), (Join-Path $pkg 'lib') $sdk -Recurse
    Write-Host "  -> build\ffmpeg-sdk (for scripts\build-mpv.ps1)"
  }
}

# mpv is built from source against this FFmpeg (scripts\build-mpv.ps1) so it doesn't embed a second copy.
# BtbN LGPL *shared* build: ffmpeg and ffprobe share one set of DLLs instead of each embedding
# all of FFmpeg (158 MB vs 316 MB). Has everything Cliper uses: NVENC/AMF/QSV, AAC, Opus,
# scale_cuda, zscale/tonemap, loudnorm. It drops only software encoders (x264/x265) we don't call.
Fetch 'BtbN/FFmpeg-Builds' '^ffmpeg-n8\.1-latest-win64-lgpl-shared-8\.1\.zip$' @('ffmpeg', 'ffprobe') -WithDlls
Remove-Item $work -Recurse -Force
