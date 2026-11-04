import { invoke } from "@tauri-apps/api/core";
import { createEffect, createMemo, createResource, createSignal, For, on, onCleanup, Show } from "solid-js";
import {
  audioKbps, buildRequest, clip, cqLabel, defaultName, estimateMb, setClip, sizePlan, startExport, totalDuration,
} from "../lib/clip";
import { fmtMB } from "../lib/format";
import { Icon } from "../lib/icons";
import { info } from "../lib/player";
import { ADV_DEFAULT, type Adv, BUILTIN_PRESETS, type Codec, setSettings, settings } from "../lib/settings";
import { Dropdown, type Opt, Seg, Switch, Toggle } from "./controls";

export const [gpu] = createResource(() => invoke<{ adapters: string[]; encoders: string[] }>("gpu_info"));
export const gpuName = () => {
  const n = gpu()?.adapters.find((a) => /nvidia/i.test(a));
  return n ? n.replace(/^NVIDIA\s+(GeForce\s+)?/i, "") : null;
};
const hasEnc = (c: Codec) => !gpu() || gpu()!.encoders.includes(`${c}_nvenc`);

const setAdv = <K extends keyof Adv>(k: K, v: Adv[K]) => setSettings("adv", k, v as any);

const PROFILES: Record<Codec, Opt<string>[]> = {
  h264: [{ value: "high", label: "High", desc: "Recommended" }, { value: "main", label: "Main" }, { value: "baseline", label: "Baseline", desc: "Old devices" }],
  hevc: [{ value: "main", label: "Main", desc: "8-bit" }, { value: "main10", label: "Main 10", desc: "10-bit" }],
  av1: [{ value: "main", label: "Main", desc: "8 or 10-bit" }],
};

const CODEC_NOTE: Record<Codec, { icon: string; text: string }> = {
  h264: { icon: "check", text: "Plays everywhere in Discord, including the inline player on desktop, web and mobile" },
  hevc: { icon: "info", text: "About 30% smaller for the same quality, but many Discord clients won't play it inline" },
  av1: { icon: "info", text: "The best quality for the size. Inline playback works in recent Discord clients only" },
};

function highlight(cmd: string) {
  const esc = cmd.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!);
  return esc
    .replace(/^ffmpeg/, '<span class="b">ffmpeg</span>')
    .replace(/("[^"]*")/g, '<span class="q">$1</span>')
    .replace(/(^|\s)(-[a-z_:0-9-]+)/gi, '$1<span class="f">$2</span>')
    .replace(/ (?=<span class="f">-(?:i|vf|c:v|rc|multipass|filter_complex|map|c:a|movflags|map_metadata|ss)\b)/g, " ^\n  ");
}

function Advanced() {
  const a = () => settings.adv;
  const tracks = () => info()?.audio ?? [];
  const changed = createMemo(() => (Object.keys(ADV_DEFAULT) as (keyof Adv)[]).filter((k) => JSON.stringify(a()[k]) !== JSON.stringify(ADV_DEFAULT[k])).length);
  const [cmd, setCmd] = createSignal("");
  const [naming, setNaming] = createSignal(false);
  const [copied, setCopied] = createSignal(false);
  let t: number | undefined;
  createEffect(on(() => JSON.stringify(buildRequest()), (json) => {
    clearTimeout(t);
    t = window.setTimeout(() => invoke<string>("export_preview", { req: JSON.parse(json) }).then(setCmd).catch(() => {}), 120);
  }));
  onCleanup(() => clearTimeout(t));

  // Keep combinations valid.
  createEffect(() => {
    if (!PROFILES[clip.codec].some((p) => p.value === a().profile)) setAdv("profile", PROFILES[clip.codec][0].value);
    if (clip.codec === "h264" && a().depth === 10) setAdv("depth", 8);
  });

  const presetOpts = (): Opt<string>[] => [
    ...BUILTIN_PRESETS.map((p) => ({ value: `b:${p.name}`, label: p.name, desc: p.desc, group: "Built-in" })),
    ...settings.presets.map((p) => ({ value: `u:${p.name}`, label: p.name, group: "Saved" })),
    { value: "", label: "Custom", desc: "Your current settings", group: settings.presets.length ? "Saved" : "Built-in", disabled: true, reason: "Your current settings" },
  ];
  const currentPreset = () => {
    const cur = JSON.stringify(a());
    for (const p of BUILTIN_PRESETS) if (JSON.stringify({ ...ADV_DEFAULT, ...p.adv }) === cur) return `b:${p.name}`;
    for (const p of settings.presets) if (JSON.stringify(p.adv) === cur) return `u:${p.name}`;
    return "";
  };
  const applyPreset = (v: string) => {
    if (v.startsWith("b:")) {
      const p = BUILTIN_PRESETS.find((x) => `b:${x.name}` === v)!;
      setSettings("adv", { ...ADV_DEFAULT, ...p.adv });
    } else if (v.startsWith("u:")) {
      const p = settings.presets.find((x) => `u:${x.name}` === v)!;
      setSettings("adv", { ...ADV_DEFAULT, ...p.adv });
    }
  };

  const num = (o: Opt<number>[]) => o;
  return (
    <div class="sec">
      <button class="adv-h" classList={{ open: settings.advOpen }} onClick={() => setSettings("advOpen", !settings.advOpen)}>
        <Icon name="tune" size={16} />Advanced
        <span class="tag" classList={{ changed: changed() > 0 }}>{changed() ? `${changed()} changed` : "Defaults"}</span>
        <span class="chev"><Icon name="chev" size={16} /></span>
      </button>
      <Show when={settings.advOpen}>
        <div class="inrow" style={{ "margin-top": "12px" }}>
          <Show
            when={naming()}
            fallback={<>
              <Dropdown value={currentPreset()} options={presetOpts()} onChange={applyPreset} style={{ flex: "1" }} />
              <button class="btn" onClick={() => setNaming(true)}>Save preset</button>
            </>}
          >
            <input
              class="inp"
              placeholder="Preset name"
              autofocus
              ref={(el) => setTimeout(() => el.focus())}
              onKeyDown={(e) => {
                if (e.key === "Enter" && e.currentTarget.value.trim()) {
                  const name = e.currentTarget.value.trim();
                  setSettings("presets", (p) => [...p.filter((x) => x.name !== name), { name, adv: structuredClone({ ...a() }) }]);
                  setNaming(false);
                }
                if (e.key === "Escape") { e.stopPropagation(); setNaming(false); }
              }}
              onBlur={() => setNaming(false)}
            />
          </Show>
        </div>

        <Show when={clip.mode !== "lossless"}>
          <div class="sub-h">Rate control</div>
          <div class="grid2">
            <div class="field"><label>Mode</label>
              <Dropdown value={a().rc} onChange={(v) => setAdv("rc", v)} options={[
                { value: "vbr", label: "VBR", desc: "Target bitrate with capped peaks. Best for size limits" },
                { value: "cbr", label: "CBR", desc: "Flat bitrate. Predictable, less efficient" },
                { value: "cq", label: "Constant quality", desc: "Size varies", disabled: clip.mode === "size", reason: "Not available in Fit to size" },
                { value: "qp", label: "Constant QP", desc: "Fixed quantizer per frame type", disabled: clip.mode === "size", reason: "Not available in Fit to size" },
              ]} />
            </div>
            <div class="field"><label>Peak bitrate</label>
              <Dropdown value={a().peak} onChange={(v) => setAdv("peak", v)} options={num([
                { value: 1.15, label: "Target +15%", desc: "Recommended" }, { value: 1.3, label: "Target +30%" },
                { value: 1.5, label: "Target +50%" }, { value: 1, label: "Same as target" },
              ])} />
            </div>
            <div class="field"><label>Multipass</label>
              <Dropdown value={a().multipass} onChange={(v) => setAdv("multipass", v)} options={[
                { value: "disabled", label: "Off", desc: "Fastest, least accurate" },
                { value: "qres", label: "Quarter res", desc: "First pass at ¼ resolution" },
                { value: "fullres", label: "Full res", desc: "Best bitrate distribution" },
              ]} />
            </div>
            <div class="field"><label>Lookahead</label>
              <Dropdown value={a().lookahead} onChange={(v) => setAdv("lookahead", v)} options={num([
                { value: 0, label: "Off" }, { value: 8, label: "8 frames" }, { value: 20, label: "20 frames", desc: "Recommended" },
                { value: 32, label: "32 frames", desc: "Slower, better scene cuts" },
              ])} />
            </div>
          </div>

          <div class="sub-h">Encoder</div>
          <div class="grid2">
            <div class="field"><label>Preset</label>
              <Dropdown value={a().preset} onChange={(v) => setAdv("preset", v)} options={[
                { value: "p1", label: "P1 · Fastest", desc: "Fastest, lowest quality" }, { value: "p2", label: "P2 · Faster" },
                { value: "p3", label: "P3 · Fast" }, { value: "p4", label: "P4 · Medium", desc: "NVENC default" },
                { value: "p5", label: "P5 · Slow" }, { value: "p6", label: "P6 · Slower", desc: "Recommended for clips" },
                { value: "p7", label: "P7 · Slowest", desc: "Best quality, ~2× slower than P4" },
              ]} />
            </div>
            <div class="field"><label>Tune</label>
              <Dropdown value={a().tune} onChange={(v) => setAdv("tune", v)} options={[
                { value: "hq", label: "High quality", desc: "For recordings and uploads" }, { value: "ll", label: "Low latency", desc: "For streaming" },
                { value: "ull", label: "Ultra-low latency" }, { value: "lossless", label: "Lossless", desc: "Huge files" },
              ]} />
            </div>
            <div class="field"><label>Profile</label>
              <Dropdown value={a().profile} onChange={(v) => setAdv("profile", v)} options={PROFILES[clip.codec]} />
            </div>
            <div class="field"><label>Bit depth</label>
              <Dropdown value={a().depth} onChange={(v) => setAdv("depth", v)} options={num([
                { value: 8, label: "8-bit", desc: "yuv420p · plays everywhere" },
                { value: 10, label: "10-bit", desc: "p010 · less banding", disabled: clip.codec === "h264", reason: "Not supported by H.264 NVENC" },
              ])} />
            </div>
            <div class="field"><label>B-frames</label>
              <Dropdown value={a().bframes} onChange={(v) => setAdv("bframes", v)} options={num([0, 1, 2, 3, 4].map((n) => ({ value: n, label: n ? String(n) : "None", desc: n === 3 ? "Recommended" : undefined })))} />
            </div>
            <div class="field"><label>B-frame refs</label>
              <Dropdown value={a().bref} onChange={(v) => setAdv("bref", v)} options={[
                { value: "middle", label: "Middle", desc: "Recommended" }, { value: "each", label: "Each" }, { value: "disabled", label: "Off" },
              ]} />
            </div>
            <div class="field"><label>Keyframe every</label>
              <Dropdown value={a().gopSec} onChange={(v) => setAdv("gopSec", v)} options={num([
                { value: 1, label: "1 second" }, { value: 2, label: "2 seconds", desc: "Good seeking in Discord" },
                { value: 5, label: "5 seconds" }, { value: 10, label: "10 seconds", desc: "Smallest files" },
              ])} />
            </div>
            <div class="field"><label>Ref frames</label>
              <Dropdown value={a().refs} onChange={(v) => setAdv("refs", v)} options={["auto", "2", "4", "6"].map((v) => ({ value: v, label: v === "auto" ? "Auto" : v }))} />
            </div>
          </div>
          <div style={{ height: "6px" }} />
          <Toggle label="Spatial AQ" hint="Gives more bits to flat areas like skies and walls" on={a().spatialAq} onChange={(v) => setAdv("spatialAq", v)} />
          <Show when={a().spatialAq}>
            <div class="field" style={{ margin: "2px 0 6px" }}>
              <label>AQ strength <em>{a().aqStrength}</em></label>
              <input type="range" class="range-in" min="1" max="15" value={a().aqStrength} onInput={(e) => setAdv("aqStrength", +e.currentTarget.value)} />
            </div>
          </Show>
          <Toggle label="Temporal AQ" hint="Sharper static HUD and UI elements" on={a().temporalAq} onChange={(v) => setAdv("temporalAq", v)} />
          <Toggle label="Weighted prediction" hint="Helps with fades and flashes. Turns off B-frames" on={a().weightedPred} onChange={(v) => setAdv("weightedPred", v)} />

          <div class="sub-h">Picture</div>
          <div class="grid2">
            <div class="field"><label>Scaling</label>
              <Dropdown value={a().scale} onChange={(v) => setAdv("scale", v)} options={[
                { value: "lanczos", label: "Lanczos", desc: "Sharpest" }, { value: "bicubic", label: "Bicubic" },
                { value: "bilinear", label: "Bilinear" }, { value: "nearest", label: "Nearest", desc: "Pixel art" },
              ]} />
            </div>
            <div class="field"><label>Crop</label>
              <Dropdown value={a().crop} onChange={(v) => setAdv("crop", v)} options={[
                { value: "src", label: "Source" }, { value: "916", label: "9:16 vertical", desc: "Centered, for phones" },
                { value: "11", label: "1:1 square" }, { value: "43", label: "4:3" }, { value: "219", label: "21:9", desc: "Ultrawide" },
              ]} />
            </div>
            <div class="field"><label>HDR</label>
              <Dropdown value={a().hdr} onChange={(v) => setAdv("hdr", v)} options={[
                { value: "keep", label: "Keep source", desc: info()?.hdr ? "Source is HDR" : "Source is SDR" },
                { value: "tonemap", label: "Tone map to SDR", desc: "Hable, for HDR captures" },
              ]} />
            </div>
            <div class="field"><label>Color range</label>
              <Dropdown value={a().range} onChange={(v) => setAdv("range", v)} options={[
                { value: "auto", label: "Auto" }, { value: "tv", label: "Limited", desc: "16–235, standard" }, { value: "pc", label: "Full", desc: "0–255" },
              ]} />
            </div>
            <div class="field"><label>Frame timing</label>
              <Dropdown value={a().vsync} onChange={(v) => setAdv("vsync", v)} options={[
                { value: "cfr", label: "Constant", desc: "Smooth playback in Discord" }, { value: "vfr", label: "Variable", desc: "Keep source timestamps" },
              ]} />
            </div>
            <div class="field"><label>Sharpen</label>
              <Dropdown value={a().sharpen} onChange={(v) => setAdv("sharpen", v)} options={num([
                { value: 0, label: "Off" }, { value: 0.3, label: "Light" }, { value: 0.6, label: "Medium" }, { value: 1, label: "Strong" },
              ])} />
            </div>
          </div>
          <Show when={a().crop !== "src" || a().hdr === "tonemap" || a().sharpen > 0}>
            <div class="note" style={{ "margin-top": "8px", "white-space": "normal" }}>
              <Icon name="info" size={13} />Crop, tone mapping and sharpening run on the CPU, so this export will be slower
            </div>
          </Show>
        </Show>

        <div class="sub-h">Audio</div>
        <Show when={tracks().length} fallback={<div class="note">This video has no audio</div>}>
          <div class="tracks">
            <For each={tracks()}>
              {(t) => {
                const on = () => a().tracks.includes(t.index);
                return (
                  <button class="trk" classList={{ on: on() }} onClick={() => setAdv("tracks", on() ? a().tracks.filter((x) => x !== t.index) : [...a().tracks, t.index].sort())}>
                    <span class="cb"><Show when={on()}><Icon name="check" size={11} /></Show></span>
                    Track {t.index + 1}{t.title ? ` · ${t.title}` : ""}
                    <small>{t.codec.toUpperCase()} · {t.layout || `${t.channels} ch`} · {Math.round(t.sampleRate / 1000)} kHz</small>
                  </button>
                );
              }}
            </For>
          </div>
          <Show when={tracks().length > 1}>
            <Toggle label="Mix selected tracks into one" hint="Discord only plays the first audio track" on={a().mix} onChange={(v) => setAdv("mix", v)} />
          </Show>
          <div style={{ height: "6px" }} />
          <div class="grid2">
            <div class="field"><label>Codec</label>
              <Dropdown value={a().acodec} onChange={(v) => setAdv("acodec", v)} options={[
                { value: "aac", label: "AAC", desc: "Plays everywhere" }, { value: "libopus", label: "Opus", desc: "Better at low bitrates" },
                { value: "copy", label: "Copy source", disabled: a().mix && a().tracks.length > 1, reason: "Mixing needs re-encoding" },
                { value: "none", label: "No audio" },
              ]} />
            </div>
            <div class="field"><label>Bitrate</label>
              <Dropdown value={a().abitrate} onChange={(v) => setAdv("abitrate", v)} options={num([96, 128, 160, 192, 256, 320].map((b) => ({ value: b, label: `${b} kbps`, desc: b === 128 ? "Recommended" : undefined })))} />
            </div>
            <div class="field"><label>Channels</label>
              <Dropdown value={a().channels} onChange={(v) => setAdv("channels", v)} options={[{ value: "2", label: "Stereo" }, { value: "1", label: "Mono" }, { value: "src", label: "Source" }]} />
            </div>
            <div class="field"><label>Sample rate</label>
              <Dropdown value={a().sampleRate} onChange={(v) => setAdv("sampleRate", v)} options={num([{ value: 48000, label: "48 kHz" }, { value: 44100, label: "44.1 kHz" }])} />
            </div>
            <div class="field"><label>{tracks()[0]?.title || "Track 1"} volume</label>
              <Dropdown value={a().gameVol} onChange={(v) => setAdv("gameVol", v)} options={num([0, -6, -3, 3, 6].map((v) => ({ value: v, label: v ? `${v > 0 ? "+" : "−"}${Math.abs(v)} dB` : "Unchanged" })))} />
            </div>
            <Show when={tracks().length > 1}>
              <div class="field"><label>{tracks()[1]?.title || "Track 2"} volume</label>
                <Dropdown value={a().micVol} onChange={(v) => setAdv("micVol", v)} options={num([0, 3, 6, 10, -3, -6].map((v) => ({ value: v, label: v ? `${v > 0 ? "+" : "−"}${Math.abs(v)} dB` : "Unchanged" })))} />
              </div>
            </Show>
          </div>
          <div style={{ height: "6px" }} />
          <Toggle label="Normalize loudness" hint="EBU R128, −14 LUFS, so clips aren't too loud" on={a().loudnorm} onChange={(v) => setAdv("loudnorm", v)} />
        </Show>

        <div class="sub-h">Output</div>
        <Toggle label="Fast start" hint="Starts playing before the file has fully loaded" on={a().faststart} onChange={(v) => setAdv("faststart", v)} />
        <Toggle label="Keep metadata" hint="Title, recording date, encoder tags" on={a().keepMeta} onChange={(v) => setAdv("keepMeta", v)} />
        <Toggle label="Hardware decoding" hint="Decodes on NVDEC and keeps frames on the GPU" on={a().hwdec} onChange={(v) => setAdv("hwdec", v)} />

        <Show when={settings.showCommand}>
          <div class="sub-h">FFmpeg</div>
          <div class="field">
            <label>Extra arguments</label>
            <input class="inp mono" placeholder="e.g. -highbitdepth 1" value={a().extra} onInput={(e) => setAdv("extra", e.currentTarget.value)} />
          </div>
          <div class="cmd-h">
            <span>Command</span>
            <button class="linkbtn" onClick={() => { navigator.clipboard.writeText(cmd()); setCopied(true); setTimeout(() => setCopied(false), 1200); }}>
              <Icon name="copy" size={12} />{copied() ? "Copied" : "Copy"}
            </button>
          </div>
          <pre class="cmd" innerHTML={highlight(cmd())} />
        </Show>
        <div class="resetrow"><button class="linkbtn" onClick={() => setSettings("adv", { ...ADV_DEFAULT })}>Reset to defaults</button></div>
      </Show>
    </div>
  );
}

export function ExportSheet() {
  const est = () => estimateMb();
  const over = () => est() > clip.mb;
  const i = () => info();
  const resOpts = createMemo<Opt<number>[]>(() => {
    const h = i()?.height ?? 1080;
    return [{ value: 0, label: `Source · ${h}p`, desc: i() ? `${i()!.width} × ${h}` : undefined },
      ...[2160, 1440, 1080, 720, 480].filter((x) => x < h).map((x) => ({ value: x, label: `${x}p` }))];
  });
  const fpsOpts = createMemo<Opt<number>[]>(() => {
    const f = Math.round(i()?.fps ?? 60);
    return [{ value: 0, label: `Source · ${f}`, desc: "No frame-rate conversion" }, ...[120, 60, 30].filter((x) => x < f).map((x) => ({ value: x, label: String(x) }))];
  });

  return (
    <aside class="sheet">
      <div class="sheet-h">
        <div>
          <h2>Export</h2>
          <div class="sub num">{clip.segs.length} segment{clip.segs.length > 1 ? "s" : ""} · {totalDuration().toFixed(1)} s</div>
        </div>
        <div class="spacer" />
        <button class="ib sm" title="Close (Esc)" onClick={() => setClip("sheet", false)}><Icon name="x" size={18} /></button>
      </div>
      <div class="sheet-b">
        <div class="sec">
          <div class="field">
            <label>File name</label>
            <div class="inrow">
              <input class="inp" placeholder={defaultName()} value={clip.name} onInput={(e) => setClip("name", e.currentTarget.value)} />
              <span class="suffix">.mp4</span>
            </div>
          </div>
        </div>

        <div class="sec">
          <h3>Mode</h3>
          <Seg full value={clip.mode} onChange={(m) => setClip("mode", m)} options={[
            { value: "size", label: "Fit to size" }, { value: "quality", label: "Best quality" }, { value: "lossless", label: "Lossless" },
          ]} />
        </div>

        <Show when={clip.mode === "size"}>
          <div class="sec">
            <h3>Size limit</h3>
            <div class="presets">
              <For each={[{ mb: 10, l: "Discord" }, { mb: 20, l: "Discord" }, { mb: 50, l: "Nitro Basic" }, { mb: 500, l: "Nitro" }]}>
                {(p) => (
                  <button class="preset" classList={{ on: clip.mb === p.mb }} onClick={() => setClip("mb", p.mb)}>
                    <b>{p.mb} MB</b><small>{p.l}</small>
                  </button>
                )}
              </For>
            </div>
            <Show when={![10, 20, 50, 500].includes(settings.customLimit)}>
              <button class="linkbtn" style={{ "margin-top": "8px" }} classList={{}} onClick={() => setClip("mb", settings.customLimit)}>
                Use custom limit · {settings.customLimit} MB{clip.mb === settings.customLimit ? " (selected)" : ""}
              </button>
            </Show>
            <div style={{ height: "12px" }} />
            <div class="auto num">
              <Show
                when={sizePlan().ok}
                fallback={<div class="full bad"><Icon name="warn" size={14} />{totalDuration().toFixed(0)} s is too long to look good at {clip.mb} MB. Trim the clip or pick a bigger limit.</div>}
              >
                <div><small>Resolution</small><b>{sizePlan().height}p</b></div>
                <div><small>Frame rate</small><b>{sizePlan().fps} fps</b></div>
                <div><small>Bitrate</small><b>{(sizePlan().videoKbps / 1000).toFixed(1)} Mbps</b></div>
                <div class="full"><Icon name="bolt" size={12} />Picked automatically to stay under {clip.mb} MB{audioKbps() ? ` · audio ${audioKbps()}k` : " · no audio"}</div>
              </Show>
            </div>
          </div>
        </Show>

        <Show when={clip.mode === "quality"}>
          <div class="sec">
            <div class="field">
              <label>Quality <em>{cqLabel()}</em></label>
              <input type="range" class="range-in" min="0" max="100" value={clip.quality} onInput={(e) => setClip("quality", +e.currentTarget.value)} />
              <div class="scale"><span>Smaller</span><span>Balanced</span><span>Visually lossless</span></div>
            </div>
            <div class="inrow">
              <div class="field" style={{ flex: "1" }}><label>Resolution</label><Dropdown value={clip.qRes} options={resOpts()} onChange={(v) => setClip("qRes", v)} /></div>
              <div class="field" style={{ flex: "1" }}><label>Frame rate</label><Dropdown value={clip.qFps} options={fpsOpts()} onChange={(v) => setClip("qFps", v)} /></div>
            </div>
          </div>
        </Show>

        <Show when={clip.mode === "lossless"}>
          <div class="sec">
            <div class="info" classList={{ warn: over() }}>
              <Show
                when={over()}
                fallback={<><Icon name="check" size={16} /><div><b>Stream copy</b>, no quality loss, done in seconds. The start snaps to the keyframe before your in point.</div></>}
              >
                <Icon name="warn" size={16} />
                <div><b>{fmtMB(est())} MB is over your {clip.mb} MB limit.</b> A lossless cut keeps the source bitrate ({((i()?.bitrate ?? 0) / 1e6).toFixed(0)} Mbps). Use Fit to size for Discord.</div>
              </Show>
            </div>
          </div>
        </Show>

        <Show when={clip.mode !== "lossless"}>
          <div class="sec">
            <h3>Encoder</h3>
            <div class="field">
              <label>
                Codec
                <span class="gpu" classList={{ off: !gpuName() }}><Icon name="bolt" size={12} />{gpuName() ? `NVENC · ${gpuName()}` : gpu.loading ? "Detecting GPU…" : "NVENC not found"}</span>
              </label>
              <Seg full value={clip.codec} onChange={(c) => setClip("codec", c)} options={[
                { value: "h264", label: "H.264", disabled: !hasEnc("h264") },
                { value: "hevc", label: "HEVC", disabled: !hasEnc("hevc") },
                { value: "av1", label: "AV1", disabled: !hasEnc("av1"), title: hasEnc("av1") ? undefined : "Needs an RTX 40-series GPU" },
              ]} />
            </div>
            <div class="note codecnote"><Icon name={CODEC_NOTE[clip.codec].icon} size={13} />{CODEC_NOTE[clip.codec].text}</div>
          </div>
        </Show>

        <Advanced />

        <Show when={clip.segs.length > 1}>
          <div class="sec">
            <h3>Segments</h3>
            <Seg full value={clip.merge} onChange={(v) => setClip("merge", v)} options={[
              { value: true, label: "Merge into one file" }, { value: false, label: "Separate files" },
            ]} />
          </div>
        </Show>

        <div class="sec">
          <h3>After export</h3>
          <div class="tog"><div>Copy file to clipboard<small>Then just press Ctrl+V in Discord</small></div><Switch on={settings.copyClipboard} onChange={(v) => setSettings("copyClipboard", v)} /></div>
          <div class="tog"><div>Show in Explorer</div><Switch on={settings.reveal} onChange={(v) => setSettings("reveal", v)} /></div>
        </div>
      </div>
      <div class="sheet-f">
        <div class="est num">
          <b classList={{ over: over() }}>≈ {fmtMB(est())} MB</b>
          {clip.mode === "lossless" ? "stream copy" : `NVENC ${clip.codec.toUpperCase()}`}
        </div>
        <button class="btn primary" disabled={clip.mode === "size" && !sizePlan().ok} onClick={startExport}>
          <Icon name="export" size={16} />Export
        </button>
      </div>
    </aside>
  );
}
