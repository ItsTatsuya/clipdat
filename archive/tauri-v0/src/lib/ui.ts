import { createSignal } from "solid-js";

export const [settingsOpen, setSettingsOpen] = createSignal(false);
export const [idle, setIdle] = createSignal(false);
export const [fullscreen, setFullscreen] = createSignal(false);

export const [osdText, setOsdText] = createSignal<string | null>(null);
let osdTimer: number | undefined;
export function osd(text: string) {
  setOsdText(text);
  clearTimeout(osdTimer);
  osdTimer = window.setTimeout(() => setOsdText(null), 1300);
}

export type Toast = { id: number; text: string; kind: "error" | "info" };
export const [toasts, setToasts] = createSignal<Toast[]>([]);
let toastId = 0;
export function toast(text: string, kind: Toast["kind"] = "error") {
  const id = ++toastId;
  setToasts((t) => [...t.slice(-2), { id, text, kind }]);
  setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), kind === "error" ? 6000 : 2500);
}

export const [flashIcon, setFlashIcon] = createSignal<{ name: string; n: number } | null>(null);
let flashN = 0;
export const flash = (name: string) => setFlashIcon({ name, n: ++flashN });
