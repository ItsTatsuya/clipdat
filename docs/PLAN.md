# Clipdat: Plan & Architecture (v2: native GPUI)

> **Note:** an earlier prototype used Tauri/WebView2. It was replaced by **GPUI** (native Rust, DirectX/DirectComposition), which cut the UI process from ~220 MB to ~85 MB. README.md has the details of the current stack.

A lightweight, IINA-style video player for Windows that is also a clip tool built for
**sharing gaming clips on Discord**. Open a recording, mark the moment, and export it under your size
limit with NVENC. The file lands on your clipboard, ready to paste into Discord.

**Decisions so far**
- Windows only
- NVENC first (RTX 4060 Ti)
- MP4 output
- Multiple segments: a merge-or-separate option
- No YouTube upload

---

## 1. Goals

| # | Goal | Measure of success |
|---|------|--------------------|
| G1 | Default video player | Opens with a double-click, plays anything mpv can, first frame in < 400 ms |
| G2 | Clip in the same UI | `C` enters clip mode. In/out, multiple segments, frame stepping, keyframe-aware timeline |
| G3 | **Fit to size (Discord)** | Pick a limit (default **20 MB**), and the output always lands under it at the best resolution/fps the bitrate allows |
| G4 | Lossless cut | Stream copy, no quality loss, ~instant. Optional "smart cut" for a frame-accurate start |
| G5 | Fast NVENC encode | Decode → scale → encode entirely on the GPU (CUDA). Many times faster than real time |
| G6 | Share in two keystrokes | Export copies the *file* to the clipboard, so Ctrl+V in Discord attaches it. You can also drag it from the job card |
| G7 | Low overhead | Idle RAM < 150 MB, < 5 % CPU playing 1440p144 with hardware decode, installer < 40 MB |

---

## 2. Stack

| Layer | Choice | Why |
|-------|--------|-----|
| App shell + UI | **Rust + GPUI** (+ gpui-component) | Native GPU UI through DirectX 11 / DirectComposition. No WebView, a single ~85 MB process |
| Playback | **mpv** sidecar rendering into a child window (`--wid`), over JSON IPC | IINA's engine. Hardware decode (d3d11va/NVDEC), every format, exact frame stepping |
| Font | **Inter** (variable, bundled locally): 500 body, 600 controls, 700 headings. Tabular numbers for all timecodes and sizes | Clean, very legible at 11–13 px on dark backgrounds. Numbers don't jitter while scrubbing |
| Icons | Bold, mostly solid, rounded set (2.4 px strokes or filled). In the real app: **Phosphor Icons "Bold/Fill"** as inline SVG | Reads clearly over video, matches the weight of Inter 600 |
| Processing | **FFmpeg 7.1+ sidecar** (`ffmpeg.exe`/`ffprobe.exe`), progress from `-progress pipe:1` | The most reliable NVENC support. Crash-isolated from the player |
| Clipboard | Rust `windows` crate → `CF_HDROP` (copies the file, like Explorer's copy) | Ctrl+V in Discord attaches the clip |
| Settings | JSON in `%APPDATA%\Clipdat` | |
| Installer | **Inno Setup**, per-user, with ProgId + file associations | No admin prompt, a clean uninstall |

---

## 3. Export modes

### 3.1 Fit to size (the default, for Discord)
Size presets: **20 MB** (default), 10 MB, 50 MB (Nitro Basic), 500 MB (Nitro), and Custom. The labels are editable.
> Note: I had Discord's free limit as 10 MB. You said 20 MB, so 20 is the default and 10 is one click away. If Discord changes it again, it's just a setting.

**Planner** (runs live as you drag the handles; the result shows on the toolbar size meter):
1. `total_kbps = limit_MB × 8000 × 0.96 / duration`. The 4 % is headroom for the container and audio.
2. `video_kbps = total_kbps − audio_kbps`, with AAC 128k by default (96k when tight).
3. Walk a quality ladder and pick the first step whose bits-per-pixel clears the codec's floor:
   `1440p60 → 1080p60 → 720p60 → 720p30 → 480p30`.
   Floors: H.264 0.040, HEVC 0.028, AV1 0.022. These are tunable after real-footage tests.
   Gaming clips prefer 60 fps, so resolution drops before frame rate.
4. If nothing clears the floor, warn: "too long for 20 MB, trim or raise the limit."
5. Encode with NVENC in VBR with a hard cap, and two-pass inside the encoder:
   ```
   ffmpeg -hwaccel cuda -hwaccel_output_format cuda -ss A -to B -i in.mkv
     -vf scale_cuda=-2:1080,fps=60
     -c:v h264_nvenc -preset p6 -tune hq -rc vbr -multipass fullres
     -b:v {v}k -maxrate {v*1.15}k -bufsize {v*2}k -spatial-aq 1 -temporal-aq 1 -bf 3
     -c:a aac -b:a 128k -movflags +faststart out.mp4
   ```
6. **Verify.** If the file is still over the limit, re-encode once at `bitrate × limit/actual × 0.97`. Never hand over a file that's too big.

### 3.2 Best quality
Constant quality (`-rc vbr -cq N -b:v 0`) with a single quality slider, plus resolution and fps choices. A size meter shows the estimate against your limit.

### 3.3 Lossless
- `ffprobe` builds the keyframe index in the background, and the timeline shows the keyframes in this mode.
- A stream-copy cut can only start on a keyframe. The striped region shows how far the start will snap back.
- `ffmpeg -ss <kf> -to <out> -i in -map 0 -c copy -avoid_negative_ts make_zero -movflags +faststart out.mp4`
- **Smart cut** (optional): re-encode only from IN to the next keyframe with NVENC, matching the source parameters, then concat. Falls back to a plain copy if the parameters don't match.
- If the result would be over your size limit, the export sheet says so and suggests Fit to size.

### Codec guidance (shown in the UI)
| Codec | Default? | Note |
|-------|----------|------|
| **H.264** | ✅ | Plays inline in Discord everywhere (desktop, web, mobile) |
| HEVC | | ~30 % smaller, but inline playback in Discord is unreliable |
| AV1 | | The best quality per MB on a 4060 Ti, but only recent clients play it inline |

### Multiple segments
Choose **Merge into one file** (concat after encode, or one `filter_complex` trim+concat pass on the GPU) or **Separate files**. Settings has "Ask each time / Merge / Separate". In Fit to size, the limit applies to each output file.

---

### Advanced options (collapsible, for power users)
Collapsed by default, and the header shows "Defaults" or "N changed". It has **saved presets** (built-in ones plus your own), **Reset to defaults**, and a **live FFmpeg command preview** you can copy.

| Group | Options |
|-------|---------|
| Rate control | Mode (VBR / CBR / constant quality / constant QP; CQ and QP are disabled in Fit to size), peak bitrate (+15/30/50 %), multipass (off / quarter / full res), lookahead (0–32 frames) |
| Encoder | Preset P1–P7, tune (HQ / LL / ULL / lossless), profile (follows the codec), bit depth (8/10-bit; 10-bit is disabled for H.264), B-frames 0–4, B-frame ref mode, keyframe interval (s), ref frames, spatial AQ + strength, temporal AQ, weighted prediction |
| Picture | Scaling algorithm (`scale_cuda` interp_algo), crop presets (9:16, 1:1, 4:3, 21:9, done in the NVDEC decoder), HDR → SDR tone map, color range, constant/variable frame timing, sharpen |
| Audio | **Pick tracks** (game / mic / voice), **mix into one** (Discord plays only the first track), game/mic volume, codec (AAC / Opus / copy / none), bitrate, channels, sample rate, EBU R128 loudness normalize |
| Output | Fast start, keep metadata, hardware decoding on/off, extra FFmpeg arguments |

The options don't allow invalid combinations: a disabled choice says why, and switching codec refreshes the profile list.

## 4. After export
- The file is copied to the clipboard (on by default), so Ctrl+V in Discord attaches it.
- The job card offers a thumbnail you can drag straight into Discord, plus "Copy again" and "Show in folder".
- Ctrl+E is a quick export that reuses your last settings without opening the sheet.

---

## 5. UI design principles
- **The video is the UI.** Borderless window, an auto-hiding title bar, and a floating blurred control bar (IINA-style).
- **Clip mode** docks a slim timeline under the video, and the video shrinks to make room. The timeline is one clean thumbnail strip: everything outside your selection is dimmed, the selection is a single amber frame with solid grab handles, the playhead is a thin white line with a dot, and only light time labels sit underneath. Keyframes appear only in Lossless mode.
- **One toolbar row:** frame step and play · timecode · In / Out / Segment · mode switch (20 MB / Quality / Lossless) · **live size meter** · Export.
- **Typography:** Inter, tabular numbers, at most three weights. **Icons:** bold and filled.
- **No native dropdowns.** Every select is a themed button plus a floating glass menu: a check mark on the current item, an optional description line, group headers, disabled items with a reason, and arrow/Enter/Esc keys. It flips upward near the bottom of the screen.
- **Keyboard-first:** `Space` `C` `I` `O` `,` `.` `N` `E` `Ctrl+E` `Esc`.

---

## 6. Milestones

| M | Scope | Exit criteria |
|---|-------|---------------|
| M0 | Spike: GPUI + mpv child window under a transparent composition layer | 1440p144 plays smoothly with the overlay. RAM measured |
| M1 | Player: open/drag-drop, controller, seek preview, tracks, file associations, single instance | Usable as the daily default player |
| M2 | Clip mode: timeline, in/out/segments, keyframe index, lossless export | A lossless export matches the preview |
| M3 | NVENC: detection, full-GPU pipeline, **fit-to-size planner + verify**, quality mode, CPU fallback | 10 test clips (5–90 s) all land at ≤ the limit, at the expected resolution |
| M4 | Sharing: CF_HDROP clipboard, drag-out, quick export, job queue | Export → Ctrl+V in Discord works |
| M5 | Polish: smart cut, settings, updater, installer | v1.0 |
