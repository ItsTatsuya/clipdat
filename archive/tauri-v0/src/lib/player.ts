import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { batch, createSignal } from "solid-js";
import { basename } from "./format";
import { setSettings, settings } from "./settings";
import { toast } from "./ui";

export type AudioTrack = { index: number; title: string; codec: string; channels: number; layout: string; sampleRate: number };
export type MediaInfo = {
  path: string; duration: number; start: number; width: number; height: number; fps: number;
  vcodec: string; pixFmt: string; hdr: boolean; bitrate: number; size: number; container: string;
  audio: AudioTrack[];
};
export type Thumb = { t: number; path: string; url?: string };

export const mpv = (...args: unknown[]) => invoke("mpv_command", { args });
export const mpvq = (...args: unknown[]) => invoke("mpv_nowait", { args });

export const [path, setPath] = createSignal<string | null>(null);
export const [info, setInfo] = createSignal<MediaInfo | null>(null);
export const [time, setTime] = createSignal(0);
export const [duration, setDuration] = createSignal(0);
export const [paused, setPaused] = createSignal(true);
export const [volume, setVolumeSig] = createSignal(100);
export const [muted, setMuted] = createSignal(false);
export const [keyframes, setKeyframes] = createSignal<number[]>([]);
export const [thumbs, setThumbs] = createSignal<Thumb[]>([]);
export const [loading, setLoading] = createSignal(false);

let scrubbing = false;
let pendingSeek: number | null = null;
let seekRaf = 0;
let resumeAt: number | null = null;

export function setScrubbing(v: boolean) { scrubbing = v; }

/** Seeks are coalesced to one per frame; only the latest position matters. */
export function seek(t: number, exact = true) {
  const d = duration();
  t = Math.max(0, d ? Math.min(t, d) : t);
  setTime(t);
  pendingSeek = t;
  if (!seekRaf) {
    seekRaf = requestAnimationFrame(() => {
      seekRaf = 0;
      if (pendingSeek != null) mpvq("seek", pendingSeek, exact ? "absolute+exact" : "absolute+keyframes");
      pendingSeek = null;
    });
  }
}

export const togglePause = () => mpvq("cycle", "pause");
export const setPause = (p: boolean) => mpvq("set_property", "pause", p);
export const frameStep = (dir: 1 | -1) => { mpvq(dir > 0 ? "frame-step" : "frame-back-step"); };
export function setVolume(v: number) {
  v = Math.max(0, Math.min(150, Math.round(v)));
  setVolumeSig(v);
  mpvq("set_property", "volume", v);
  setSettings("volume", v);
}
export const toggleMute = () => mpvq("cycle", "mute");

function rememberPosition() {
  const p = path();
  if (!p || !settings.resume) return;
  const t = time(), d = duration();
  setSettings("positions", (pos) => {
    const next = { ...pos };
    if (t > 5 && t < d - 5) next[p] = Math.round(t * 10) / 10; else delete next[p];
    const keys = Object.keys(next);
    if (keys.length > 300) delete next[keys[0]];
    return next;
  });
}

export async function openFile(p: string) {
  rememberPosition();
  batch(() => {
    setPath(p); setInfo(null); setKeyframes([]); setThumbs([]); setTime(0); setDuration(0); setLoading(true);
  });
  resumeAt = settings.resume ? settings.positions[p] ?? null : null;
  getCurrentWindow().setTitle(`${basename(p)} — Cliper`).catch(() => {});
  try {
    await mpv("loadfile", p);
    mpvq("set_property", "pause", false);
  } catch (e) {
    toast(`Couldn't open ${basename(p)}: ${e}`);
  }
  const i = await invoke<MediaInfo>("probe", { path: p }).catch((e) => { toast(`Probe failed: ${e}`); return null; });
  if (path() !== p) return;
  if (i) {
    setInfo(i);
    if (!duration()) setDuration(i.duration);
    invoke<number[]>("keyframes", { path: p }).then((k) => {
      if (path() === p) setKeyframes(k.map((x) => x - (i.start || 0)));
    }).catch(() => {});
    const { convertFileSrc } = await import("@tauri-apps/api/core");
    invoke<Thumb[]>("thumbnails", { path: p, duration: i.duration, count: Math.min(240, Math.max(40, Math.round(i.duration / 2))) })
      .then((th) => {
        if (path() === p) setThumbs(th.map((x) => ({ ...x, t: x.t - (i.start || 0), url: convertFileSrc(x.path) })));
      })
      .catch(() => {});
  }
  setLoading(false);
}

export async function initPlayer() {
  setVolumeSig(settings.volume);
  mpvq("set_property", "volume", settings.volume);
  mpvq("set_property", "hwdec", settings.hwdec);
  await listen<{ name: string; data: any }>("mpv-prop", ({ payload: { name, data } }) => {
    switch (name) {
      case "time-pos": if (!scrubbing && pendingSeek == null && typeof data === "number") setTime(data); break;
      case "duration": if (typeof data === "number") setDuration(data); break;
      case "pause": setPaused(!!data); break;
      case "volume": if (typeof data === "number") setVolumeSig(Math.round(data)); break;
      case "mute": setMuted(!!data); break;
    }
  });
  await listen<any>("mpv-event", ({ payload }) => {
    if (payload.event === "file-loaded" && resumeAt != null) {
      const t = resumeAt; resumeAt = null;
      mpvq("seek", t, "absolute+exact");
      toast(`Resumed at ${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, "0")}`, "info");
    }
    if (payload.event === "mpv-exited") toast("The video engine stopped. Restart Cliper.");
  });
  await listen<string>("open-file", ({ payload }) => openFile(payload));
  setInterval(() => { if (!paused()) rememberPosition(); }, 5000);
  window.addEventListener("beforeunload", rememberPosition);
}
