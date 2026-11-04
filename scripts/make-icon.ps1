# Renders assets/brand/logo.svg into assets/icon.ico (16-256 px PNG frames) and assets/brand/logo-<n>.png.
#   pwsh scripts/make-icon.ps1
# Rasterizes once at 1024 px with headless Chrome/Edge, then downsamples each size (bicubic).
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$svg  = Join-Path $root 'assets\brand\logo.svg'
$browser = @("$env:ProgramFiles\Google\Chrome\Application\chrome.exe", "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
             "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $browser) { throw "Chrome or Edge is needed to render the SVG" }

$tmp = Join-Path ([IO.Path]::GetTempPath()) ("clipdat-icon-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
$big = 1024
Copy-Item $svg "$tmp\logo.svg"
"<!doctype html><html><body style='margin:0;background:transparent'><img src='logo.svg' width='$big' height='$big' style='display:block'></body></html>" |
  Set-Content "$tmp\page.html"
$url = 'file:///' + "$tmp\page.html".Replace([char]92, '/')
& $browser --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 --default-background-color=00000000 `
  "--user-data-dir=$tmp\profile" "--window-size=$big,$big" "--screenshot=$tmp\big.png" $url 2>$null | Out-Null
if (-not (Test-Path "$tmp\big.png")) { throw "render failed" }
$src = [Drawing.Bitmap]::FromFile("$tmp\big.png")

function Scale([int]$n) {
  $bmp = New-Object Drawing.Bitmap $n, $n, ([Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [Drawing.Graphics]::FromImage($bmp)
  $g.InterpolationMode = 'HighQualityBicubic'; $g.PixelOffsetMode = 'HighQuality'; $g.CompositingQuality = 'HighQuality'
  $g.DrawImage($src, (New-Object Drawing.Rectangle 0, 0, $n, $n), 0, 0, $big, $big, 'Pixel')
  $g.Dispose()
  $ms = New-Object IO.MemoryStream
  $bmp.Save($ms, [Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
  , $ms.ToArray()
}

$sizes = 16, 20, 24, 32, 40, 48, 64, 128, 256
$frames = foreach ($n in $sizes) { , (Scale $n) }
[IO.File]::WriteAllBytes((Join-Path $root 'assets\brand\logo-256.png'), $frames[-1])
[IO.File]::WriteAllBytes((Join-Path $root 'assets\brand\logo-512.png'), (Scale 512))
$src.Dispose()

# ICO = 6-byte header, a 16-byte directory entry per frame, then the PNG payloads.
$ms = New-Object IO.MemoryStream
$w = New-Object IO.BinaryWriter $ms
$w.Write([uint16]0); $w.Write([uint16]1); $w.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
  $n = $sizes[$i]; $len = $frames[$i].Length
  $w.Write([byte]($n % 256)); $w.Write([byte]($n % 256)); $w.Write([byte]0); $w.Write([byte]0)
  $w.Write([uint16]1); $w.Write([uint16]32); $w.Write([uint32]$len); $w.Write([uint32]$offset)
  $offset += $len
}
foreach ($f in $frames) { $w.Write($f) }
[IO.File]::WriteAllBytes((Join-Path $root 'assets\icon.ico'), $ms.ToArray())
Remove-Item $tmp -Recurse -Force
"assets\icon.ico: $($sizes -join ', ') px"
