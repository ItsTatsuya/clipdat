import "@fontsource-variable/inter";
import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createEffect, createRoot, on } from "solid-js";
import { render } from "solid-js/web";
import { App } from "./App";
import { initJobs } from "./lib/jobs";
import { initPlayer, openFile } from "./lib/player";
import { loadSettings, saveSettingsSoon, settings } from "./lib/settings";

async function boot() {
  await loadSettings();
  createRoot(() => {
    createEffect(on(() => JSON.stringify(settings), saveSettingsSoon, { defer: true }));
  });
  await Promise.all([initPlayer(), initJobs()]);
  render(() => <App />, document.getElementById("root")!);
  await getCurrentWindow().show();
  const f = await invoke<string | null>("take_initial_file");
  if (f) openFile(f);
}

boot();
