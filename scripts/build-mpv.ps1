# Builds mpv against the SAME shared FFmpeg that ships in bin/ (no second FFmpeg copy).
#   pwsh scripts/build-mpv.ps1 [-Tag v0.42.0]
# Needs: MSYS2 at C:\msys64 with the UCRT64 toolchain + libplacebo/libass (see README),
#        build/ffmpeg-sdk (headers + import libs from the same BtbN build as bin/, made by fetch-deps.ps1).
param([string]$Tag = "v0.42.0")
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$bash = 'C:\msys64\usr\bin\bash.exe'
if (-not (Test-Path $bash)) { throw "MSYS2 not found at C:\msys64" }
if (-not (Test-Path "$root\build\ffmpeg-sdk\lib\pkgconfig\libavcodec.pc")) { throw "build\ffmpeg-sdk missing: run scripts\fetch-deps.ps1 -Force" }

$src = "$root\build\mpv-src"
if (-not (Test-Path $src)) { git clone --depth 1 --branch $Tag https://github.com/mpv-player/mpv.git $src }
# An existing checkout of another tag is switched, so bumping -Tag really rebuilds that version.
$have = (git -C $src describe --tags --exact-match 2>$null)
if ($have -ne $Tag) {
  git -C $src fetch --depth 1 origin "refs/tags/$($Tag):refs/tags/$Tag"
  git -C $src checkout --force $Tag
  if ($LASTEXITCODE -ne 0) { throw "could not check out mpv $Tag" }
}

function Unix($p) { '/' + $p.Substring(0, 1).ToLower() + ($p.Substring(2) -replace '\\', '/') }
$env:MSYSTEM = 'UCRT64'
$env:CHERE_INVOKING = '1'
$script = @"
set -e
export PKG_CONFIG_PATH='$(Unix "$root\build\ffmpeg-sdk\lib\pkgconfig")'
cd '$(Unix $src)'
rm -rf ../mpv-build
meson setup ../mpv-build --buildtype=release --strip --wrap-mode=nodownload \
  -Dlibmpv=false -Dcplayer=true \
  -Dlua=disabled -Djavascript=disabled -Dlibarchive=disabled -Dlibbluray=disabled \
  -Ddvdnav=disabled -Dcdda=disabled -Duchardet=disabled -Drubberband=disabled \
  -Dvapoursynth=disabled -Dzimg=disabled -Djpeg=disabled -Dlcms2=disabled \
  -Dsdl2-audio=disabled -Dsdl2-video=disabled -Dsdl2-gamepad=disabled \
  -Dmanpage-build=disabled -Dhtml-build=disabled -Dpdf-build=disabled \
  -Dd3d11=enabled -Dd3d-hwaccel=enabled -Dgl=disabled -Dvulkan=disabled -Dshaderc=enabled -Dspirv-cross=enabled
ninja -C ../mpv-build mpv.exe
"@
& $bash -lc $script
if ($LASTEXITCODE -ne 0) { throw "mpv build failed" }

# Ship mpv.exe + only the MSYS2 DLLs it needs (FFmpeg DLLs already live in bin/).
$exe = "$root\build\mpv-build\mpv.exe"
Copy-Item $exe "$root\bin\mpv.exe" -Force
$copied = @()
# ldd resolves through bin\ once a DLL is copied there, so repeat until nothing comes from MSYS2.
for ($pass = 0; $pass -lt 6; $pass++) {
  $deps = & $bash -lc "ldd '$(Unix "$root\bin\mpv.exe")' | awk '`$3 ~ /^\/ucrt64\// {print `$3}' | sort -u"
  if (-not $deps) { break }
  foreach ($d in $deps) {
    $win = (& $bash -lc "cygpath -w '$d'").Trim()
    $name = Split-Path $win -Leaf
    if ($name -match '^(avcodec|avformat|avfilter|avutil|avdevice|swscale|swresample)-\d+\.dll$') { throw "mpv resolved FFmpeg from MSYS2 ($name): it must use bin\" }
    Copy-Item $win "$root\bin\" -Force
    $copied += $name
  }
}
$size = ($copied | Sort-Object -Unique | ForEach-Object { (Get-Item "$root\bin\$_").Length } | Measure-Object -Sum).Sum
"mpv.exe {0:N1} MB + {1} support DLLs ({2:N1} MB) in bin\" -f ((Get-Item $exe).Length / 1MB), ($copied | Sort-Object -Unique).Count, ($size / 1MB)
($copied | Sort-Object -Unique) -join ', '
