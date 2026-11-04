import { getCurrentWindow } from "@tauri-apps/api/window";
import { createMemo, createSignal, For, Show } from "solid-js";
import { clip, setClipMode } from "../lib/clip";
import { basename, fmtL, fmtS, nearest } from "../lib/format";
import { Icon } from "../lib/icons";
import {
  duration, muted, path, seek, setScrubbing, setVolume, thumbs, time, togglePause, toggleMute, volume, paused,
} from "../lib/player";
import { flash, flashIcon, fullscreen, osdText, setSettingsOpen, toasts } from "../lib/ui";

const win = getCurrentWindow();

export function TitleBar() {
  return (
    <div class="titlebar" data-tauri-drag-region>
      <div class="app-name"><div class="logo" />Cliper</div>
      <div class="name">{path() ? basename(path()!) : ""}</div>
      <div class="caption">
        <button title="Minimize" onClick={() => win.minimize()}>
          <svg width="10" height="10"><path d="M0 5h10" stroke="currentColor" stroke-width="1.2" /></svg>
        </button>
        <button title="Maximize" onClick={() => win.toggleMaximize()}>
          <svg width="10" height="10"><rect x=".6" y=".6" width="8.8" height="8.8" rx="1.5" stroke="currentColor" stroke-width="1.2" fill="none" /></svg>
        </button>
        <button class="close" title="Close" onClick={() => win.close()}>
          <svg width="10" height="10"><path d="M.5 .5l9 9M9.5 .5l-9 9" stroke="currentColor" stroke-width="1.2" /></svg>
        </button>
      </div>
    </div>
  );
}

/** Hover preview with the nearest thumbnail and a timecode. */
export function Preview(props: { x: number; t: number; width: number }) {
  const th = () => nearest(thumbs(), props.t);
  return (
    <div class="preview" style={{ left: `${Math.min(Math.max(props.x, 90), props.width - 90)}px` }}>
      <Show when={th()}>
        <div class="img" style={{ "background-image": `url("${th()!.url}")` }} />
      </Show>
      <b class="num">{fmtL(props.t)}</b>
    </div>
  );
}

function SeekBar() {
  let el!: HTMLDivElement;
  const [hover, setHover] = createSignal<{ x: number; t: number; w: number } | null>(null);
  const d = () => duration() || 1;
  const at = (e: PointerEvent) => {
    const r = el.getBoundingClientRect();
    const x = Math.min(Math.max(e.clientX - r.left, 0), r.width);
    return { x, t: (x / r.width) * d(), w: r.width };
  };
  return (
    <div
      ref={el}
      class="seek"
      onPointerDown={(e) => {
        el.setPointerCapture(e.pointerId);
        setScrubbing(true);
        seek(at(e).t);
      }}
      onPointerMove={(e) => {
        const h = at(e);
        setHover(h);
        if (el.hasPointerCapture(e.pointerId)) seek(h.t);
      }}
      onPointerUp={() => setScrubbing(false)}
      onLostPointerCapture={() => setScrubbing(false)}
      onPointerLeave={() => setHover(null)}
    >
      <div class="track"><div class="fill" style={{ width: `${(time() / d()) * 100}%` }} /></div>
      <For each={clip.segs}>
        {(g) => <div class="clipmark" style={{ left: `${(g.a / d()) * 100}%`, width: `${((g.b - g.a) / d()) * 100}%` }} />}
      </For>
      <div class="knob" style={{ left: `${(time() / d()) * 100}%` }} />
      <Show when={hover()}>{(h) => <Preview x={h().x} t={h().t} width={h().w} />}</Show>
    </div>
  );
}

function Volume() {
  let el!: HTMLDivElement;
  const set = (e: PointerEvent) => {
    const r = el.getBoundingClientRect();
    setVolume((Math.min(Math.max(e.clientX - r.left, 0), r.width) / r.width) * 150);
  };
  return (
    <div
      ref={el}
      class="vol"
      title={`Volume ${volume()}%`}
      onPointerDown={(e) => { el.setPointerCapture(e.pointerId); set(e); }}
      onPointerMove={(e) => el.hasPointerCapture(e.pointerId) && set(e)}
      onWheel={(e) => setVolume(volume() + (e.deltaY < 0 ? 5 : -5))}
    >
      <div class="t">
        <i style={{ width: `${(Math.min(volume(), 100) / 150) * 100}%`, opacity: muted() ? 0.3 : 1 }} />
        <Show when={volume() > 100}>
          <i class="boost" style={{ left: `${(100 / 150) * 100}%`, width: `${((volume() - 100) / 150) * 100}%` }} />
        </Show>
      </div>
    </div>
  );
}

export function Osc(props: { onHover: (v: boolean) => void; onFullscreen: () => void }) {
  return (
    <div class="osc" onPointerEnter={() => props.onHover(true)} onPointerLeave={() => props.onHover(false)}>
      <div class="osc-row">
        <div class="osc-left">
          <button class="ib" title="Mute (M)" onClick={toggleMute}><Icon name={muted() ? "mute" : "vol"} size={20} /></button>
          <Volume />
        </div>
        <div class="osc-mid">
          <button class="ib" title="Back 5 s (←)" onClick={() => seek(time() - 5)}><Icon name="rew" size={20} /></button>
          <button class="ib big" title="Play / Pause (Space)" onClick={() => { flash(paused() ? "play" : "pause"); togglePause(); }}>
            <Icon name={paused() ? "play" : "pause"} size={22} />
          </button>
          <button class="ib" title="Forward 5 s (→)" onClick={() => seek(time() + 5)}><Icon name="ff" size={20} /></button>
        </div>
        <div class="osc-right">
          <button class="ib" title="Clip (C)" onClick={() => setClipMode(true)}><Icon name="scissors" size={20} /></button>
          <button class="ib" title="Settings (Ctrl+,)" onClick={() => setSettingsOpen(true)}><Icon name="gear" size={20} /></button>
          <button class="ib" title="Fullscreen (F)" onClick={props.onFullscreen}>
            <Icon name={fullscreen() ? "unfull" : "full"} size={20} />
          </button>
        </div>
      </div>
      <div class="seek-row num">
        <span>{fmtS(time())}</span>
        <SeekBar />
        <span>-{fmtS(duration() - time())}</span>
      </div>
    </div>
  );
}

export function Empty(props: { dragging: boolean; onOpen: () => void }) {
  return (
    <div class="empty" classList={{ drag: props.dragging }} data-tauri-drag-region>
      <div class="card">
        <div class="glyph"><Icon name="scissors" size={34} /></div>
        <h1>{props.dragging ? "Drop to open" : "Open a video"}</h1>
        <p>Drop a recording here, or press <kbd>Ctrl</kbd> <kbd>O</kbd></p>
        <button class="btn primary" style={{ "margin-top": "6px" }} onClick={props.onOpen}>
          <Icon name="film" size={16} />Choose file
        </button>
      </div>
    </div>
  );
}

export function Overlays() {
  const flashKey = createMemo(() => flashIcon());
  return (
    <>
      <Show when={osdText()}>
        <div class="osd"><span class="dot" /><span class="num">{osdText()}</span></div>
      </Show>
      <Show when={flashKey()} keyed>
        {(f) => <div class="flash"><Icon name={f.name} size={28} /></div>}
      </Show>
      <div class="toasts">
        <For each={toasts()}>
          {(t) => (
            <div class="toast" classList={{ info: t.kind === "info" }}>
              <Icon name={t.kind === "info" ? "info" : "warn"} size={16} />
              {t.text}
            </div>
          )}
        </For>
      </div>
    </>
  );
}
