import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { createEffect, createSignal, on, onCleanup, onMount, Show } from "solid-js";
import { ClipDock } from "./components/ClipDock";
import { ExportSheet } from "./components/ExportSheet";
import { JobCards } from "./components/Jobs";
import { Empty, Osc, Overlays, TitleBar } from "./components/Player";
import { SettingsModal } from "./components/Settings";
import { clip, deleteSegment, newSegment, resetClipForFile, setClip, setClipMode, setIn, setOut, startExport } from "./lib/clip";
import {
  duration, frameStep, mpvq, openFile, path, paused, seek, setVolume, time, togglePause, toggleMute, volume,
} from "./lib/player";
import { settings } from "./lib/settings";
import { flash, fullscreen, idle, setFullscreen, setIdle, setSettingsOpen, settingsOpen } from "./lib/ui";

const VIDEO_EXT = ["mp4", "mkv", "mov", "webm", "avi", "m4v", "ts", "m2ts", "flv", "wmv", "mpg", "mpeg", "3gp", "ogv"];
const win = getCurrentWindow();

export async function chooseFile() {
  const f = await open({ multiple: false, filters: [{ name: "Video", extensions: VIDEO_EXT }] });
  if (typeof f === "string") openFile(f);
}

async function toggleFullscreen() {
  const fs = !(await win.isFullscreen());
  await win.setFullscreen(fs);
  setFullscreen(fs);
}

export function App() {
  const [dragging, setDragging] = createSignal(false);
  const [overControls, setOverControls] = createSignal(false);
  const [size, setSize] = createSignal({ w: innerWidth, h: innerHeight });

  // New file → fresh clip state.
  createEffect(on(path, () => resetClipForFile()));

  // Keep the picture clear of the dock and export sheet.
  createEffect(() => {
    const { w, h } = size();
    mpvq("set_property", "video-margin-ratio-bottom", clip.on ? Math.min(164 / h, 0.6) : 0);
    mpvq("set_property", "video-margin-ratio-right", clip.sheet ? Math.min(412 / w, 0.6) : 0);
  });

  // Loop the active segment while playing in clip mode.
  let prevT = 0;
  createEffect(() => {
    const t = time();
    const g = clip.on ? clip.segs[clip.act] : undefined;
    if (g && !paused() && prevT < g.b && t >= g.b && t - prevT < 1) seek(g.a);
    prevT = t;
  });

  // Auto-hide controls + cursor.
  let idleTimer: number | undefined;
  const wake = () => {
    setIdle(false);
    clearTimeout(idleTimer);
    if (settings.autoHide > 0) {
      idleTimer = window.setTimeout(() => {
        if (!paused() && !clip.on && !settingsOpen() && !overControls() && path()) setIdle(true);
      }, settings.autoHide);
    }
  };
  createEffect(on(paused, wake));

  // Single click = play/pause, double click = fullscreen, drag = move window.
  let clickTimer: number | undefined;
  let down: { x: number; y: number } | null = null;
  const hitDown = (e: PointerEvent) => {
    if (e.button !== 0) return;
    down = { x: e.clientX, y: e.clientY };
  };
  const hitMove = (e: PointerEvent) => {
    if (down && (Math.abs(e.clientX - down.x) > 4 || Math.abs(e.clientY - down.y) > 4)) {
      down = null;
      if (!fullscreen()) win.startDragging();
    }
  };
  const hitUp = (e: PointerEvent) => {
    if (!down || e.button !== 0) return;
    down = null;
    if (e.detail >= 2) {
      clearTimeout(clickTimer);
      toggleFullscreen();
      return;
    }
    clearTimeout(clickTimer);
    clickTimer = window.setTimeout(() => {
      if (!path()) return;
      flash(paused() ? "play" : "pause");
      togglePause();
    }, 220);
  };

  const onExport = (e?: { ctrlKey?: boolean }) => {
    if (!path()) return;
    if (e?.ctrlKey) { if (!clip.on) setClipMode(true); startExport(); return; }
    setClipMode(true);
    setClip("sheet", true);
  };

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target instanceof HTMLElement ? e.target : document.body;
      if (t.matches("input, textarea")) return;
      if (t.matches("button") && (e.key === " " || e.key === "Enter")) return;
      const k = e.key.toLowerCase();
      const has = !!path();
      if (e.ctrlKey && k === "o") { e.preventDefault(); chooseFile(); return; }
      if (e.ctrlKey && k === ",") { e.preventDefault(); setSettingsOpen(!settingsOpen()); return; }
      if (k === "escape") {
        if (settingsOpen()) setSettingsOpen(false);
        else if (clip.sheet) setClip("sheet", false);
        else if (clip.on) setClipMode(false);
        else if (fullscreen()) toggleFullscreen();
        return;
      }
      if (settingsOpen() || !has) return;
      if (k === " ") { e.preventDefault(); flash(paused() ? "play" : "pause"); togglePause(); }
      else if (k === "c" && !e.ctrlKey) setClipMode(!clip.on);
      else if (k === "i" && clip.on) setIn();
      else if (k === "o" && clip.on) setOut();
      else if (k === "n" && clip.on) newSegment();
      else if (k === "delete" && clip.on) deleteSegment(clip.act);
      else if (k === "e") { e.preventDefault(); onExport(e); }
      else if (k === ",") frameStep(-1);
      else if (k === ".") frameStep(1);
      else if (k === "arrowleft") seek(time() - (e.shiftKey ? 1 : 5));
      else if (k === "arrowright") seek(Math.min(time() + (e.shiftKey ? 1 : 5), duration()));
      else if (k === "arrowup") { e.preventDefault(); setVolume(volume() + 5); }
      else if (k === "arrowdown") { e.preventDefault(); setVolume(volume() - 5); }
      else if (k === "m") toggleMute();
      else if (k === "f") toggleFullscreen();
      else return;
      wake();
    };
    document.addEventListener("keydown", onKey);
    const onResize = () => setSize({ w: innerWidth, h: innerHeight });
    window.addEventListener("resize", onResize);
    const unResize = win.onResized(async () => setFullscreen(await win.isFullscreen()));
    const unDrop = getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "enter" || p.type === "over") setDragging(true);
      else if (p.type === "leave") setDragging(false);
      else if (p.type === "drop") {
        setDragging(false);
        const f = p.paths.find((x) => VIDEO_EXT.includes(x.split(".").pop()?.toLowerCase() ?? "")) ?? p.paths[0];
        if (f) openFile(f);
      }
    });
    document.addEventListener("contextmenu", (e) => e.preventDefault());
    onCleanup(() => {
      document.removeEventListener("keydown", onKey);
      window.removeEventListener("resize", onResize);
      unResize.then((f) => f());
      unDrop.then((f) => f());
    });
  });

  return (
    <div
      class="app"
      classList={{ clip: clip.on, exporting: clip.sheet, idle: idle(), fs: fullscreen() }}
      onPointerMove={wake}
    >
      <div class="stage">
        <div class="hit" onPointerDown={hitDown} onPointerMove={hitMove} onPointerUp={hitUp} />
        <Show when={!path()}><Empty dragging={dragging()} onOpen={chooseFile} /></Show>
        <TitleBar />
        <Overlays />
        <Show when={path()}>
          <Osc onHover={setOverControls} onFullscreen={toggleFullscreen} />
          <ExportSheet />
        </Show>
        <JobCards />
      </div>
      <Show when={path()} fallback={<div />}>
        <ClipDock onExport={() => onExport()} />
      </Show>
      <Show when={settingsOpen()}><SettingsModal /></Show>
    </div>
  );
}
