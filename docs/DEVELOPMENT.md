# Developing Clipdat

## Stack

**Rust + GPUI** (Zed's GPU UI framework). It renders through DirectX 11 / DirectComposition on Windows, so there's no WebView or browser runtime. Text inputs come from `gpui-component`.

| Piece | How |
|---|---|
| Video | mpv (sidecar) renders into a **child window** of the GPUI window (`--wid`), controlled over its JSON IPC pipe |
| UI over video | GPUI's DirectComposition target is *topmost*, so its transparent UI is composited **above** mpv's child window. The child window is disabled for input, so every click goes to GPUI |
| Processing | FFmpeg 8.1 / ffprobe sidecars, with progress from `-progress pipe:1`, on a tokio runtime |
| One FFmpeg | mpv is built from source against the **same shared FFmpeg DLLs** that ffmpeg/ffprobe use, so FFmpeg ships once |
| Font / icons | Inter (embedded) with tabular digits; a custom bold SVG icon set. The palette is strictly monochrome (`src/ui/theme.rs`) |
| Brand | `assets/brand/logo.svg` is the source for `assets/icon.ico` (`scripts/make-icon.ps1`) and the in-app `logo` icon |

Measured during 1440p144 playback (release build): the UI process uses about 85 MB, at about 3% of one core while playing and 0.3% while paused. mpv adds about 245 MB for the 1440p decode buffers.

The plan and design notes are in `docs/PLAN.md`.

## Layout

```
src/
  main.rs            app bootstrap, theme, window
  planner.rs         fit-to-size planner (resolution/fps/bitrate ladder)
  fmt.rs             time/size formatting
  backend/           mpv IPC, ffprobe/thumbnails, export jobs, GPU detection,
                     settings, single instance, Windows integration
  ui/                GPUI views: player, clip dock, export sheet, settings
installer/clipdat.iss  Inno Setup script (per-user installer)
scripts/             fetch-deps, build-mpv, make-icon, package
```

## Build

Requirements:
- Rust with the MSVC toolchain
- Visual Studio Build Tools with the Windows 10/11 SDK. GPUI compiles its shaders with `fxc`.
- To build mpv: MSYS2 at `C:\msys64`, with:

```sh
pacman -S git mingw-w64-ucrt-x86_64-{gcc,meson,ninja,pkgconf,libplacebo,libass,shaderc,spirv-cross,ffnvcodec-headers,zlib}
```

```powershell
pwsh scripts/fetch-deps.ps1     # shared FFmpeg (DLLs + ffmpeg/ffprobe) into bin/, its SDK into build/
pwsh scripts/build-mpv.ps1      # mpv 0.41 built against that same FFmpeg, + its support DLLs into bin/
cargo run --release -- "C:\path\to\recording.mkv"
pwsh scripts/package.ps1        # dist/Clipdat-Setup-<ver>.exe + dist/Clipdat-portable.zip
```

`package.ps1` uses an installed copy of Inno Setup 6 or 7. If there isn't one, it downloads Inno Setup into `build/inno` the first time. `.cargo/config.toml` links the C runtime statically, so users don't need the Visual C++ Redistributable.

To change the logo, edit `assets/brand/logo.svg`, then run `pwsh scripts/make-icon.ps1`. That regenerates `assets/icon.ico` and the PNGs.

## Tests

The planner and formatting tests, plus **real NVENC exports** against any sample video:

```powershell
$env:CLIPDAT_BIN_DIR = "bin"; $env:CLIPDAT_SAMPLE = "C:\path\to\recording.mkv"
cargo test -- --test-threads=1
```

## Known gaps

- Smart cut isn't implemented yet: a lossless start snaps to the previous keyframe.
- Export uses NVENC only. AMD (AMF) and Intel (QSV) encoders are detected but not used yet.
- The installer and exe aren't code-signed, so SmartScreen warns about them.
- Panels over the video are tinted glass, not blurred. GPUI can't blur mpv's separate video surface.
- Controls hide when idle, but the mouse cursor itself doesn't.
