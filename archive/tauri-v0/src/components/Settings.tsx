import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { createSignal, For, Match, Show, Switch as Sw } from "solid-js";
import { Icon } from "../lib/icons";
import { mpvq } from "../lib/player";
import { setSettings, settings } from "../lib/settings";
import { setSettingsOpen } from "../lib/ui";
import { Dropdown, Seg, Switch } from "./controls";
import { gpu } from "./ExportSheet";

type Tab = "general" | "export" | "gpu" | "keys";

export function SettingsModal() {
  const [tab, setTab] = createSignal<Tab>("general");
  const nav: { id: Tab; label: string; icon: string }[] = [
    { id: "general", label: "General", icon: "gear" },
    { id: "export", label: "Export", icon: "export" },
    { id: "gpu", label: "GPU", icon: "bolt" },
    { id: "keys", label: "Shortcuts", icon: "kbd" },
  ];
  return (
    <div class="scrim" onPointerDown={(e) => e.target === e.currentTarget && setSettingsOpen(false)}>
      <div class="modal">
        <nav class="nav">
          <div class="ttl">Settings</div>
          <For each={nav}>
            {(n) => (
              <button classList={{ on: tab() === n.id }} onClick={() => setTab(n.id)}>
                <Icon name={n.icon} size={16} />{n.label}
              </button>
            )}
          </For>
        </nav>
        <div class="pane">
          <button class="ib sm closex" onClick={() => setSettingsOpen(false)}><Icon name="x" size={18} /></button>
          <Sw>
            <Match when={tab() === "general"}>
              <h2>General</h2>
              <div class="group">
                <div class="row">
                  <div class="l">Default video player<small>Windows asks you to confirm in Settings → Default apps</small></div>
                  <button class="btn" onClick={() => invoke("open_default_apps_settings")}>Make default…</button>
                </div>
                <div class="row">
                  <div class="l">Hardware decoding</div>
                  <Dropdown value={settings.hwdec} onChange={(v) => { setSettings("hwdec", v); mpvq("set_property", "hwdec", v); }} options={[
                    { value: "auto-safe", label: "Auto", desc: "D3D11 / NVDEC when safe" },
                    { value: "nvdec", label: "NVDEC" }, { value: "d3d11va", label: "D3D11VA" }, { value: "no", label: "Off" },
                  ]} />
                </div>
                <div class="row"><div class="l">Resume where I left off</div><Switch on={settings.resume} onChange={(v) => setSettings("resume", v)} /></div>
                <div class="row">
                  <div class="l">Hide controls after</div>
                  <Dropdown value={settings.autoHide} onChange={(v) => setSettings("autoHide", v)} options={[
                    { value: 1500, label: "1.5 seconds" }, { value: 2000, label: "2 seconds" }, { value: 4000, label: "4 seconds" }, { value: 0, label: "Never" },
                  ]} />
                </div>
              </div>
            </Match>

            <Match when={tab() === "export"}>
              <h2>Export</h2>
              <div class="group">
                <div class="row">
                  <div class="l">Default mode</div>
                  <Seg value={settings.defaultMode} onChange={(v) => setSettings("defaultMode", v)} options={[
                    { value: "size", label: "Fit to size" }, { value: "quality", label: "Quality" }, { value: "lossless", label: "Lossless" },
                  ]} />
                </div>
                <div class="row">
                  <div class="l">Default size limit<small>Shown on the clip toolbar</small></div>
                  <Dropdown value={settings.defaultLimit} onChange={(v) => setSettings("defaultLimit", v)} options={[
                    { value: 20, label: "20 MB · Discord" }, { value: 10, label: "10 MB · Discord" }, { value: 50, label: "50 MB · Nitro Basic" },
                    { value: 500, label: "500 MB · Nitro" },
                    ...(![10, 20, 50, 500].includes(settings.customLimit) ? [{ value: settings.customLimit, label: `${settings.customLimit} MB · Custom` }] : []),
                  ]} />
                </div>
                <div class="row">
                  <div class="l">Custom size limit<small>An extra option in the export sheet</small></div>
                  <input class="inp num" type="number" min="1" max="4000" style={{ width: "120px" }} value={settings.customLimit}
                    onChange={(e) => { const v = Math.max(1, Math.min(4000, +e.currentTarget.value || 25)); setSettings("customLimit", v); }} />
                </div>
                <div class="row">
                  <div class="l">Default codec</div>
                  <Seg value={settings.defaultCodec} onChange={(v) => setSettings("defaultCodec", v)} options={[
                    { value: "h264", label: "H.264" }, { value: "hevc", label: "HEVC" }, { value: "av1", label: "AV1" },
                  ]} />
                </div>
                <div class="row">
                  <div class="l">Multiple segments</div>
                  <Dropdown value={settings.segments} onChange={(v) => setSettings("segments", v)} options={[
                    { value: "ask", label: "Merge, change per export", desc: "Choose in the export sheet" },
                    { value: "merge", label: "Merge into one file" }, { value: "separate", label: "Separate files" },
                  ]} />
                </div>
              </div>
              <div class="group">
                <div class="row">
                  <div class="l">Output folder<small style={{ "word-break": "break-all" }}>{settings.outputDir}</small></div>
                  <button class="btn" onClick={async () => {
                    const d = await open({ directory: true, defaultPath: settings.outputDir });
                    if (typeof d === "string") setSettings("outputDir", d);
                  }}>Change…</button>
                </div>
                <div class="row">
                  <div class="l">File name<small>{"{name} {n} {date} {time}"}</small></div>
                  <input class="inp" value={settings.nameTemplate} onChange={(e) => setSettings("nameTemplate", e.currentTarget.value || "{name}_clip{n}")} />
                </div>
                <div class="row"><div class="l">Copy file to clipboard after export</div><Switch on={settings.copyClipboard} onChange={(v) => setSettings("copyClipboard", v)} /></div>
                <div class="row"><div class="l">Show advanced options by default<small>Expands Advanced in the export sheet</small></div><Switch on={settings.advOpen} onChange={(v) => setSettings("advOpen", v)} /></div>
                <div class="row"><div class="l">Show FFmpeg command<small>Shows the exact command in the Advanced section</small></div><Switch on={settings.showCommand} onChange={(v) => setSettings("showCommand", v)} /></div>
              </div>
            </Match>

            <Match when={tab() === "gpu"}>
              <h2>GPU</h2>
              <div class="group">
                <Show when={!gpu.loading} fallback={<div class="row"><div class="l">Detecting hardware…</div></div>}>
                  <For each={gpu()?.adapters ?? []}>
                    {(name) => {
                      const vendor = /nvidia/i.test(name) ? "nvenc" : /amd|radeon/i.test(name) ? "amf" : /intel/i.test(name) ? "qsv" : "";
                      const encs = () => (gpu()?.encoders ?? []).filter((e) => e.endsWith(vendor) && vendor);
                      return (
                        <div class="row">
                          <div class="l">{name}<small>{encs().length ? encs().map((e) => e.split("_")[0].toUpperCase().replace("HEVC", "HEVC")).join(" · ") : "No hardware encoder found"}</small></div>
                          <span class="gpu" classList={{ off: !encs().length }}><Icon name={encs().length ? "check" : "x"} size={13} />{encs().length ? "Ready" : "Unavailable"}</span>
                        </div>
                      );
                    }}
                  </For>
                </Show>
                <div class="row"><div class="l">Encoding uses NVENC<small>Other GPU vendors are detected but not used for export yet</small></div></div>
              </div>
            </Match>

            <Match when={tab() === "keys"}>
              <h2>Shortcuts</h2>
              <div class="kbdtable">
                <For each={[
                  ["Play / pause", ["Space"]], ["Open file", ["Ctrl", "O"]], ["Clip mode", ["C"]], ["Set in / out", ["I", "O"]],
                  ["Previous / next frame", [",", "."]], ["Seek ±5 s (Shift: ±1 s)", ["←", "→"]], ["Volume", ["↑", "↓"]],
                  ["Mute", ["M"]], ["Fullscreen", ["F"]], ["New segment", ["N"]], ["Delete segment", ["Del"]],
                  ["Export", ["E"]], ["Quick export with last settings", ["Ctrl", "E"]], ["Settings", ["Ctrl", ","]],
                  ["Close panel / exit clip mode", ["Esc"]],
                ] as [string, string[]][]}>
                  {([l, k]) => <><span>{l}</span><span><For each={k}>{(x) => <><kbd>{x}</kbd>{" "}</>}</For></span></>}
                </For>
              </div>
            </Match>
          </Sw>
        </div>
      </div>
    </div>
  );
}
