import { invoke } from "@tauri-apps/api/core";
import { createStore, unwrap } from "solid-js/store";

export type Adv = {
  rc: "vbr" | "cbr" | "cq" | "qp";
  peak: number;
  multipass: "disabled" | "qres" | "fullres";
  lookahead: number;
  preset: string;
  tune: string;
  profile: string;
  depth: number;
  bframes: number;
  bref: string;
  gopSec: number;
  refs: string;
  spatialAq: boolean;
  aqStrength: number;
  temporalAq: boolean;
  weightedPred: boolean;
  scale: string;
  crop: string;
  hdr: string;
  range: string;
  vsync: string;
  sharpen: number;
  tracks: number[];
  mix: boolean;
  acodec: string;
  abitrate: number;
  channels: string;
  sampleRate: number;
  gameVol: number;
  micVol: number;
  loudnorm: boolean;
  faststart: boolean;
  keepMeta: boolean;
  hwdec: boolean;
  extra: string;
};

export const ADV_DEFAULT: Adv = {
  rc: "vbr", peak: 1.15, multipass: "fullres", lookahead: 20, preset: "p6", tune: "hq", profile: "high",
  depth: 8, bframes: 3, bref: "middle", gopSec: 2, refs: "auto", spatialAq: true, aqStrength: 8,
  temporalAq: true, weightedPred: false, scale: "lanczos", crop: "src", hdr: "keep", range: "auto",
  vsync: "cfr", sharpen: 0, tracks: [0, 1], mix: true, acodec: "aac", abitrate: 128, channels: "2",
  sampleRate: 48000, gameVol: 0, micVol: 0, loudnorm: false, faststart: true, keepMeta: false,
  hwdec: true, extra: "",
};

export type Mode = "size" | "quality" | "lossless";
export type Codec = "h264" | "hevc" | "av1";

export type Settings = {
  outputDir: string;
  nameTemplate: string;
  defaultMode: Mode;
  defaultLimit: number;
  customLimit: number;
  defaultCodec: Codec;
  segments: "ask" | "merge" | "separate";
  copyClipboard: boolean;
  reveal: boolean;
  hwdec: string;
  resume: boolean;
  autoHide: number;
  advOpen: boolean;
  showCommand: boolean;
  volume: number;
  adv: Adv;
  presets: { name: string; adv: Adv }[];
  positions: Record<string, number>;
  counters: Record<string, number>;
};

export const DEFAULTS: Settings = {
  outputDir: "",
  nameTemplate: "{name}_clip{n}",
  defaultMode: "size",
  defaultLimit: 20,
  customLimit: 25,
  defaultCodec: "h264",
  segments: "ask",
  copyClipboard: true,
  reveal: false,
  hwdec: "auto-safe",
  resume: true,
  autoHide: 2000,
  advOpen: false,
  showCommand: true,
  volume: 100,
  adv: { ...ADV_DEFAULT },
  presets: [],
  positions: {},
  counters: {},
};

export const [settings, setSettings] = createStore<Settings>(structuredClone(DEFAULTS));

export async function loadSettings() {
  const v = (await invoke<Partial<Settings> | null>("settings_load").catch(() => null)) ?? {};
  setSettings({
    ...structuredClone(DEFAULTS),
    ...v,
    adv: { ...ADV_DEFAULT, ...(v.adv ?? {}) },
  });
  if (!settings.outputDir) setSettings("outputDir", await invoke<string>("default_output_dir"));
}

let saveTimer: number | undefined;
export function saveSettingsSoon() {
  clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    invoke("settings_save", { value: unwrap(settings) }).catch(() => {});
  }, 400);
}

/** Built-in presets for the Advanced section. */
export const BUILTIN_PRESETS: { name: string; desc: string; adv: Partial<Adv> }[] = [
  { name: "Discord · balanced", desc: "Fit to size, P6, full-res multipass", adv: {} },
  { name: "Discord · max quality", desc: "P7, 32-frame lookahead, slower", adv: { preset: "p7", lookahead: 32 } },
  { name: "Fast", desc: "P3, no multipass, quickest exports", adv: { preset: "p3", multipass: "disabled", lookahead: 8 } },
];
