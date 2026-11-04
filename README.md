# Cliper 9000

A native, IINA-style video player for Windows, built for clipping gaming recordings and sharing them on Discord.
You open a video, mark the moment, and export it under your Discord size limit with NVENC. The file lands on your clipboard, ready for Ctrl+V.

- **Player:** mpv (hardware decoding), a borderless window, a floating controller, and thumbnail previews on the seek bar
- **Clip mode** (`C`): a timeline with thumbnails, in/out handles, multiple segments, frame stepping and keyframe markers
- **Export modes:**
  - **Fit to size** (10/20/50/500 MB or custom): picks the resolution and fps automatically, verifies the output size, and retries once if it's over the limit
  - **Best quality**: constant quality on NVENC
  - **Lossless**: stream copy, which snaps the start to a keyframe
- **NVENC** H.264 / HEVC / AV1, with decoding, scaling and encoding all on the GPU (CUDA)
- **Advanced options:** rate control, presets, AQ, B-frames, GOP, 10-bit, crop presets, HDR→SDR, audio track picking and mixing, loudness normalization, saved presets, and a live FFmpeg command preview
- **After export:** the file is copied to the clipboard (CF_HDROP), so Ctrl+V in Discord attaches it

## Stack

**Rust + GPUI**: Zed's GPU UI framework, which renders through DirectX 11 / DirectComposition on Windows. There's no WebView or browser runtime. Text inputs come from `gpui-component`.

| Piece | How |
|---|---|
| Video | mpv (sidecar) renders into a **child window** of the GPUI window (`--wid`), controlled over its JSON IPC pipe |
| UI over video | GPUI's DirectComposition target is *topmost*, so its transparent UI is composited **above** mpv's child window. The child window is disabled for input, so every click goes to GPUI |
| Processing | FFmpeg 8.1 / ffprobe sidecars, with progress from `-progress pipe:1`, on a tokio runtime |
| One FFmpeg | mpv is built from source against the **same shared FFmpeg DLLs** that ffmpeg/ffprobe use, so FFmpeg ships once. The portable app is 206 MB, an 88 MB zip (it was 470 MB / 179 MB) |
| Font / icons | Inter (embedded) with tabular digits; a custom bold SVG icon set |

Measured during 1440p144 playback (release build): the **UI process uses 85 MB in a single process, at about 3% of one core while playing and 0.3% while paused**. mpv adds about 245 MB for the 1440p decode buffers.

The approved design is in `design/mockup.html`, and the plan is in `docs/PLAN.md`. The earlier Tauri/WebView version is kept in `archive/tauri-v0/` for reference.

## Build & run

Requirements: Rust (MSVC toolchain), the Visual Studio Build Tools with the Windows 10/11 SDK (GPUI compiles its shaders with `fxc`), and, to build mpv, MSYS2 at `C:\msys64` with:

```sh
pacman -S git mingw-w64-ucrt-x86_64-{gcc,meson,ninja,pkgconf,libplacebo,libass,shaderc,spirv-cross,ffnvcodec-headers,zlib}
```

```powershell
pwsh scripts/fetch-deps.ps1         # shared FFmpeg (DLLs + ffmpeg/ffprobe) into bin/, its SDK into build/
pwsh scripts/build-mpv.ps1          # mpv 0.41 built against that same FFmpeg, + its support DLLs into bin/
cargo run --release -- "C:\path\to\recording.mkv"
pwsh scripts/package.ps1            # dist/Cliper-Setup-<ver>.exe installer + portable zip
```

`package.ps1` uses an installed Inno Setup 6/7, or downloads one into `build/inno` the first time. `.cargo/config.toml` links the C runtime statically, so users don't need the Visual C++ Redistributable.

## Install

Run `Cliper-Setup-<version>.exe`. It installs for the current user only (no admin prompt) to `%LOCALAPPDATA%\Programs\Cliper`, and:
- adds a Start menu shortcut (a desktop shortcut is optional)
- registers Cliper as a video player, so it shows up in Windows *Default apps*. The last page can open that settings page for you
- adds a normal uninstaller under Settings → Apps. It removes the file-type registration and the thumbnail cache, but keeps your settings

Updating means running a newer setup over the old one. Windows SmartScreen warns about the setup until it's code-signed (*More info → Run anyway*).
Requirements: Windows 10 1809+ or 11, x64. Exports need an NVIDIA GPU (AV1 needs RTX 40-series).

Tests: the planner and formatting tests, plus **real NVENC exports** against any sample video:

```powershell
$env:CLIPER_BIN_DIR = "bin"; $env:CLIPER_SAMPLE = "C:\path\to\recording.mkv"
cargo test -- --test-threads=1
```

## Shortcuts

| Key | Action |
|---|---|
| `Space` | Play / pause |
| `Ctrl+O` | Open file |
| `C` | Clip mode |
| `I` / `O` | Set in / out |
| `,` / `.` | Previous / next frame |
| `←` / `→` | Seek ±5 s (with `Shift`: ±1 s) |
| `↑` / `↓` | Volume |
| `M` | Mute |
| `F` | Fullscreen (or double-click) |
| `N` | New segment |
| `Del` | Delete segment |
| `E` | Export |
| `Ctrl+E` | Quick export with last settings |
| `Ctrl+,` | Settings |
| `Esc` | Close panel / exit clip mode |

You can also drop files on the window, drag the video to move the window, and scroll over the video to change the volume.

## Default player

Settings → General → **Make default…** registers Cliper for video files under HKCU (no admin needed), then opens Windows *Default apps*, where you pick Cliper. Opening another video while Cliper is running reuses the same window.

## Known gaps

- Smart cut (re-encoding only the first GOP of a lossless cut) is not implemented yet. A lossless start snaps to the previous keyframe, and the timeline shows by how much.
- Export uses NVENC only. AMD (AMF) and Intel (QSV) encoders are detected but not used yet.
- The installer and exe aren't code-signed yet, so SmartScreen warns about them.
- Panels over the video are tinted glass, not blurred. GPUI can't blur mpv's separate video surface.
- Controls hide when idle, but the mouse cursor itself doesn't (GPUI has no hidden-cursor style yet).
