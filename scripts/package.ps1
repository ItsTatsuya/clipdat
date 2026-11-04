# Builds the Cliper installer + a portable zip in dist/.
#   pwsh scripts/package.ps1 [-NoInstaller]
# Output: dist/Cliper-Setup-<version>.exe (Inno Setup, per-user), dist/Cliper-portable.zip
# Layout: dist/Cliper/cliper.exe, dist/Cliper/bin/{mpv,ffmpeg,ffprobe}.exe + DLLs
param([switch]$NoInstaller)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

foreach ($b in 'mpv', 'ffmpeg', 'ffprobe') {
  if (-not (Test-Path "bin/$b.exe")) { throw "bin/$b.exe missing: run scripts/fetch-deps.ps1, then scripts/build-mpv.ps1" }
}

cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$out = Join-Path $root 'dist/Cliper'
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force "$out/bin" | Out-Null
Copy-Item target/release/cliper.exe $out
foreach ($b in 'mpv', 'ffmpeg', 'ffprobe') { Copy-Item "bin/$b.exe" "$out/bin/" }
Get-ChildItem bin -Filter '*.dll' | Copy-Item -Destination "$out/bin/"   # shared FFmpeg + mpv support libraries
Copy-Item assets/fonts/LICENSE-Inter.txt $out -ErrorAction SilentlyContinue
@"
Cliper 9000 (portable)

Run cliper.exe. To make it your default video player:
Settings -> General -> Make default...

Bundled: mpv 0.41 (GPLv2+, mpv.io, built against the shared FFmpeg), FFmpeg (LGPL shared build by BtbN, ffmpeg.org), Inter font (OFL).
"@ | Set-Content "$out/README.txt"

$zip = Join-Path $root 'dist/Cliper-portable.zip'
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path $out -DestinationPath $zip -CompressionLevel Optimal
"{0}  ({1:N0} MB)" -f $zip, ((Get-Item $zip).Length / 1MB)
if ($NoInstaller) { return }

# Inno Setup compiler: use an installed one, else fetch it once into build/inno.
$iscc = @('7', '6' | ForEach-Object { "$env:LOCALAPPDATA\Programs\Inno Setup $_\ISCC.exe", "$env:ProgramFiles\Inno Setup $_\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup $_\ISCC.exe" }) +
        "$root\build\inno\ISCC.exe" | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) {
  $rel = Invoke-RestMethod 'https://api.github.com/repos/jrsoftware/issrc/releases/latest'
  $a = $rel.assets | Where-Object { $_.name -match '^innosetup-[\d.]+-x64\.exe$' } | Select-Object -First 1
  if (-not $a) { throw "no Inno Setup installer in $($rel.tag_name)" }
  $is = Join-Path ([IO.Path]::GetTempPath()) "innosetup-$([guid]::NewGuid()).exe"
  Write-Host "downloading $($a.name)"
  Invoke-WebRequest $a.browser_download_url -OutFile $is
  $head = [IO.File]::ReadAllBytes($is)[0..1]
  if ($head[0] -ne 0x4D -or $head[1] -ne 0x5A) { Remove-Item $is; throw "Inno Setup download is not an exe" }
  Start-Process $is -Wait -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER', '/NOICONS', "/DIR=`"$root\build\inno`""
  Remove-Item $is
  $iscc = "$root\build\inno\ISCC.exe"
  if (-not (Test-Path $iscc)) { throw "Inno Setup install failed (expected $iscc)" }
}
$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches.Groups[1].Value
& $iscc /Q "/DAppVersion=$version" "/DSrc=$out" "/DOut=$root\dist" installer\cliper.iss
if ($LASTEXITCODE -ne 0) { throw "installer build failed" }
$setup = Join-Path $root "dist/Cliper-Setup-$version.exe"
"{0}  ({1:N0} MB)" -f $setup, ((Get-Item $setup).Length / 1MB)
