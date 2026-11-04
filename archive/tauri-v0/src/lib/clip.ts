import { invoke } from "@tauri-apps/api/core";
import { createMemo } from "solid-js";
import { createStore, produce, unwrap } from "solid-js/store";
import { fmtL, stem } from "./format";
import { duration, info, keyframes, path, thumbs, time } from "./player";
import { cqFor, estimateQuality, plan } from "./planner";
import { type Codec, type Mode, settings, setSettings } from "./settings";
import { osd, toast } from "./ui";
import { floorIndex, nearest } from "./format";

export type Seg = { a: number; b: number };

export const [clip, setClip] = createStore({
  on: false,
  sheet: false,
  segs: [] as Seg[],
  act: 0,
  mode: "size" as Mode,
  mb: 20,
  codec: "h264" as Codec,
  quality: 68,
  qRes: 0, // 0 = source
  qFps: 0, // 0 = source
  merge: true,
  name: "",
});

const MIN_LEN = 0.2;

export function resetClipForFile() {
  setClip({
    on: false, sheet: false, segs: [], act: 0,
    mode: settings.defaultMode, mb: settings.defaultLimit, codec: settings.defaultCodec,
    merge: settings.segments !== "separate", name: "",
  });
}

function ensureSeg() {
  if (clip.segs.length) return;
  // Start at the playhead; near the end, select the last 30 s instead ("clip what just happened").
  const d = duration() || 0, t = time();
  const a = t + 30 > d ? Math.max(0, d - 30) : t;
  setClip({ segs: [{ a, b: Math.min(d, a + 30) }], act: 0 });
}

export function setClipMode(on: boolean) {
  if (!path()) return;
  if (on) ensureSeg();
  setClip({ on, sheet: on ? clip.sheet : false });
}

export function setIn() {
  ensureSeg();
  const t = time();
  setClip("segs", clip.act, produce((g) => { g.a = Math.min(t, g.b - MIN_LEN); }));
  osd(`In  ${fmtL(clip.segs[clip.act].a)}`);
}
export function setOut() {
  ensureSeg();
  const t = time();
  setClip("segs", clip.act, produce((g) => { g.b = Math.max(t, g.a + MIN_LEN); }));
  osd(`Out  ${fmtL(clip.segs[clip.act].b)}`);
}
export function newSegment() {
  const d = duration();
  const a = Math.min(time(), Math.max(0, d - 2));
  const segs = [...unwrap(clip.segs), { a, b: Math.min(d, a + 10) }].sort((x, y) => x.a - y.a);
  setClip({ segs, act: segs.findIndex((g) => g.a === a) });
  osd("New segment");
}
export function deleteSegment(i: number) {
  if (clip.segs.length <= 1) return;
  setClip(produce((c) => { c.segs.splice(i, 1); c.act = Math.min(c.act, c.segs.length - 1); }));
}
export function moveEdge(i: number, edge: "a" | "b", t: number) {
  const d = duration();
  setClip("segs", i, produce((g) => {
    if (edge === "a") g.a = Math.max(0, Math.min(t, g.b - MIN_LEN));
    else g.b = Math.min(d, Math.max(t, g.a + MIN_LEN));
  }));
}

export const totalDuration = createMemo(() => clip.segs.reduce((s, g) => s + Math.max(0, g.b - g.a), 0));

const audioTrackCount = () => info()?.audio.length ?? 0;
export const audioKbps = createMemo(() => {
  const a = settings.adv;
  const n = Math.min(a.tracks.filter((t) => t < audioTrackCount()).length, a.mix ? 1 : 99);
  if (a.acodec === "none" || n === 0) return 0;
  if (a.acodec === "copy") return 192 * n;
  return a.abitrate * n;
});

export const sizePlan = createMemo(() => {
  const i = info();
  return plan({
    duration: clip.merge || clip.segs.length < 2 ? totalDuration() : Math.max(...clip.segs.map((g) => g.b - g.a), 0.1),
    limitMb: clip.mb,
    codec: clip.codec,
    srcW: i?.width ?? 1920,
    srcH: i?.height ?? 1080,
    srcFps: i?.fps ?? 60,
    audioKbps: audioKbps(),
  });
});

/** Estimated output size (MB) for the current mode. Per file when exporting separately. */
export const estimateMb = createMemo(() => {
  const i = info();
  if (clip.mode === "size") return sizePlan().sizeMb;
  if (clip.mode === "lossless") return (totalDuration() * (i?.bitrate ?? 40e6)) / 8 / 1e6;
  const h = clip.qRes || i?.height || 1080;
  return estimateQuality(totalDuration(), clip.quality, clip.codec, h) + (totalDuration() * audioKbps()) / 8000;
});

export function keyframeBefore(t: number) {
  const k = keyframes();
  const i = floorIndex(k, t);
  return i >= 0 ? k[i] : 0;
}

function fileName() {
  if (clip.name) return clip.name;
  const p = path();
  if (!p) return "clip";
  const name = stem(p);
  const n = (settings.counters[name] ?? 0) + 1;
  return settings.nameTemplate
    .replace("{name}", name)
    .replace("{n}", String(n).padStart(2, "0"))
    .replace("{date}", new Date().toISOString().slice(0, 10))
    .replace("{time}", new Date().toTimeString().slice(0, 8).replace(/:/g, "-"));
}
export const defaultName = createMemo(fileName);

export function buildRequest() {
  const i = info();
  const p = sizePlan();
  const a = settings.adv;
  const q = clip.mode === "quality";
  const height = clip.mode === "size" ? p.height : q ? clip.qRes || i?.height || 0 : 0;
  const fps = clip.mode === "size" ? p.fps : q ? clip.qFps || i?.fps || 0 : 0;
  // Quality mode with VBR/CBR needs a bitrate; derive one from the quality slider.
  const qKbps = Math.round((4 + clip.quality * 0.32) * 1000 * Math.pow((height || 1080) / 1080, 1.4));
  return {
    input: path() ?? "",
    segments: clip.segs.map((g) => [g.a, g.b]),
    mode: clip.mode,
    limitMb: clip.mb,
    codec: clip.codec,
    quality: clip.quality,
    height,
    fps,
    videoKbps: clip.mode === "size" ? p.videoKbps : qKbps,
    merge: clip.merge,
    outDir: settings.outputDir,
    name: fileName(),
    copyClipboard: settings.copyClipboard,
    reveal: settings.reveal,
    srcWidth: i?.width ?? 0,
    srcHeight: i?.height ?? 0,
    srcFps: i?.fps ?? 0,
    audioTracks: i?.audio.length ?? 0,
    adv: {
      ...unwrap(a),
      rc: clip.mode === "size" && (a.rc === "cq" || a.rc === "qp") ? "vbr" : a.rc,
      depth: clip.codec === "h264" ? 8 : a.depth,
    },
  };
}

export async function startExport() {
  if (!path() || !clip.segs.length) return;
  if (clip.mode === "size" && !sizePlan().ok) {
    toast(`This clip is too long to look good at ${clip.mb} MB. Trim it or pick a bigger limit.`);
    return;
  }
  const req = buildRequest();
  try {
    const id = await invoke<number>("export_start", { req });
    const { addJob } = await import("./jobs");
    addJob(id, req.name, req.mode === "lossless" ? "lossless" : `NVENC ${clip.codec.toUpperCase()}`, nearest(thumbs(), clip.segs[0].a + 1)?.url);
    const name = stem(path()!);
    if (!clip.name) setSettings("counters", name, (n) => (n ?? 0) + 1);
    setClip("sheet", false);
  } catch (e) {
    toast(`Export failed to start: ${e}`);
  }
}

export const cqLabel = () => {
  const q = clip.quality;
  return `${q > 85 ? "Visually lossless" : q > 55 ? "High" : q > 30 ? "Balanced" : "Small"} · CQ ${cqFor(q)}`;
};
