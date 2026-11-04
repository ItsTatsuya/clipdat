import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import {
  clip, deleteSegment, estimateMb, keyframeBefore, moveEdge, newSegment, setClip, setClipMode, setIn, setOut, sizePlan,
} from "../lib/clip";
import { fmtL, fmtMB, fmtS, nearest } from "../lib/format";
import { Icon } from "../lib/icons";
import {
  duration, frameStep, keyframes, paused, seek, setScrubbing, thumbs, time, togglePause,
} from "../lib/player";
import { flash } from "../lib/ui";
import { Seg } from "./controls";
import { Preview } from "./Player";

const STRIP_H = 54;
const TILE_W = (STRIP_H * 16) / 9;

function Timeline() {
  let el!: HTMLDivElement;
  let axis!: HTMLCanvasElement;
  const [w, setW] = createSignal(0);
  const [hover, setHover] = createSignal<{ x: number; t: number } | null>(null);
  const d = () => duration() || 1;
  const pct = (t: number) => `${(t / d()) * 100}%`;

  onMount(() => {
    const ro = new ResizeObserver(() => setW(el.clientWidth));
    ro.observe(el);
    onCleanup(() => ro.disconnect());
  });

  const tiles = createMemo(() => {
    const width = w(), th = thumbs();
    if (!width) return [] as (string | undefined)[];
    const n = Math.ceil(width / TILE_W);
    return Array.from({ length: n }, (_, i) => nearest(th, ((i + 0.5) * TILE_W / width) * d())?.url);
  });

  const dims = createMemo(() => {
    const sorted = [...clip.segs].sort((a, b) => a.a - b.a);
    const out: { a: number; b: number; first: boolean; last: boolean }[] = [];
    let prev = 0;
    for (const g of sorted) {
      if (g.a > prev) out.push({ a: prev, b: g.a, first: prev === 0, last: false });
      prev = Math.max(prev, g.b);
    }
    if (prev < d()) out.push({ a: prev, b: d(), first: prev === 0, last: true });
    return out;
  });

  // Axis labels + keyframe dots (lossless mode only).
  createEffect(() => {
    const width = w(), dur = d(), kf = clip.mode === "lossless" ? keyframes() : [];
    if (!width) return;
    const dpr = Math.min(devicePixelRatio, 2);
    axis.width = width * dpr;
    axis.height = 16 * dpr;
    const c = axis.getContext("2d")!;
    c.scale(dpr, dpr);
    c.font = "500 10.5px 'Inter Variable', 'Segoe UI', sans-serif";
    c.textBaseline = "top";
    if (kf.length && kf.length < width * 2) {
      c.fillStyle = "rgba(255,178,36,.6)";
      for (const t of kf) { c.beginPath(); c.arc((t / dur) * width, 2, 1.2, 0, 7); c.fill(); }
    }
    c.fillStyle = "rgba(244,244,246,.38)";
    const steps = [1, 2, 5, 10, 15, 20, 30, 60, 120, 300, 600, 900, 1800, 3600];
    const step = steps.find((s) => (s / dur) * width >= 90) ?? 3600;
    for (let t = 0; t <= dur; t += step) {
      const x = (t / dur) * width, s = fmtS(t), tw = c.measureText(s).width;
      if (x + tw / 2 < width) c.fillText(s, t === 0 ? 0 : x - tw / 2, 5);
    }
  });

  const tAt = (e: PointerEvent) => {
    const r = el.getBoundingClientRect();
    const x = Math.min(Math.max(e.clientX - r.left, 0), r.width);
    return { x, t: (x / r.width) * d() };
  };
  let drag: { seg?: number; edge?: "a" | "b" } | null = null;
  const move = (e: PointerEvent) => {
    const { t } = tAt(e);
    if (drag?.seg != null) {
      moveEdge(drag.seg, drag.edge!, t);
      const g = clip.segs[drag.seg];
      seek(drag.edge === "a" ? g.a : g.b);
    } else seek(t);
  };

  return (
    <div
      ref={el}
      class="tl"
      onPointerDown={(e) => {
        const h = (e.target as HTMLElement).closest<HTMLElement>(".h");
        el.setPointerCapture(e.pointerId);
        setScrubbing(true);
        drag = h ? { seg: +h.dataset.r!, edge: h.dataset.e as "a" | "b" } : {};
        if (h) setClip("act", +h.dataset.r!);
        move(e);
      }}
      onPointerMove={(e) => {
        setHover(tAt(e));
        if (drag) move(e);
      }}
      onPointerUp={() => { drag = null; setScrubbing(false); }}
      onLostPointerCapture={() => { drag = null; setScrubbing(false); }}
      onPointerLeave={() => setHover(null)}
    >
      <div class="strip" classList={{ loading: !thumbs().length }}>
        <For each={tiles()}>
          {(u) => <div class="tile" style={{ width: `${TILE_W}px`, "background-image": u ? `url("${u}")` : undefined }} />}
        </For>
      </div>
      <For each={dims()}>
        {(m) => <div class="dim" classList={{ first: m.first, last: m.last }} style={{ left: pct(m.a), width: pct(m.b - m.a) }} />}
      </For>
      <Show when={clip.mode === "lossless" && clip.segs[clip.act]}>
        {(g) => {
          const k = () => keyframeBefore(g().a);
          return <div class="snap" style={{ left: pct(k()), width: pct(g().a - k()) }} />;
        }}
      </Show>
      <For each={clip.segs}>
        {(g, i) => (
          <div class="sel" classList={{ inactive: i() !== clip.act }} style={{ left: pct(g.a), width: pct(g.b - g.a) }}>
            <div class="h l" data-r={i()} data-e="a" />
            <div class="h r" data-r={i()} data-e="b" />
            <Show when={((g.b - g.a) / d()) * w() > 70}>
              <span class="lbl num">{(g.b - g.a).toFixed(1)} s</span>
            </Show>
          </div>
        )}
      </For>
      <Show when={hover()}>{(h) => <div class="hoverline" style={{ left: `${h().x}px` }} />}</Show>
      <div class="playhead" style={{ left: pct(time()) }} />
      <canvas ref={axis} class="axis" />
      <Show when={hover()}>{(h) => <Preview x={h().x} t={h().t} width={w()} />}</Show>
    </div>
  );
}

function Note() {
  const s = () => clip.segs[clip.act];
  return (
    <Show
      when={clip.mode === "lossless"}
      fallback={
        <Show
          when={clip.mode !== "size" || sizePlan().ok}
          fallback={<div class="note warn"><Icon name="warn" size={13} />Too long for {clip.mb} MB. Trim it or pick a bigger limit</div>}
        >
          <div class="note num">
            <Icon name="bolt" size={13} />
            NVENC {clip.codec.toUpperCase()}
            {clip.mode === "size" ? ` · ${sizePlan().height}p${sizePlan().fps} · ${(sizePlan().videoKbps / 1000).toFixed(1)} Mbps` : " · frame-accurate"}
          </div>
        </Show>
      }
    >
      <Show when={s()}>
        {(g) => {
          const k = () => keyframeBefore(g().a);
          const off = () => g().a - k();
          return (
            <Show
              when={keyframes().length && off() > 0.01}
              fallback={<div class="note"><Icon name="check" size={13} />{keyframes().length ? "In point is on a keyframe" : "Indexing keyframes…"}</div>}
            >
              <div class="note warn num"><Icon name="warn" size={13} />Start snaps to keyframe {fmtL(k())} (−{off().toFixed(2)} s)</div>
            </Show>
          );
        }}
      </Show>
    </Show>
  );
}

export function ClipDock(props: { onExport: () => void }) {
  const est = () => estimateMb();
  return (
    <div class="dock">
      <div class="dock-inner">
        <div class="bar">
          <button class="ib sm" title="Previous frame (,)" onClick={() => frameStep(-1)}><Icon name="stepb" size={18} /></button>
          <button class="ib sm" title="Play / Pause (Space)" onClick={() => { flash(paused() ? "play" : "pause"); togglePause(); }}>
            <Icon name={paused() ? "play" : "pause"} size={18} />
          </button>
          <button class="ib sm" title="Next frame (.)" onClick={() => frameStep(1)}><Icon name="stepf" size={18} /></button>
          <div class="tc num"><b>{fmtL(time())}</b><span>/ {fmtL(duration())}</span></div>
          <div class="spacer" />
          <button class="tbtn" title="Set in (I)" onClick={setIn}><Icon name="in" size={18} /><span class="t">In</span></button>
          <button class="tbtn" title="Set out (O)" onClick={setOut}><Icon name="out" size={18} /><span class="t">Out</span></button>
          <button class="tbtn" title="New segment (N)" onClick={newSegment}><Icon name="plus" size={18} /><span class="t">Segment</span></button>
          <div class="div" />
          <Seg
            value={clip.mode}
            onChange={(m) => setClip("mode", m)}
            options={[
              { value: "size", label: `${clip.mb} MB` },
              { value: "quality", label: "Quality" },
              { value: "lossless", label: "Lossless" },
            ]}
          />
          <div class="meter num" classList={{ over: est() > clip.mb }} title="Estimated size vs your limit">
            <div class="g"><i style={{ width: `${Math.min((est() / clip.mb) * 100, 100)}%` }} /></div>
            <b>{fmtMB(est())} MB</b><span>/ {clip.mb}</span>
          </div>
          <button class="btn primary" title="Export (E) · Quick export (Ctrl+E)" onClick={props.onExport}>
            <Icon name="export" size={16} />Export
          </button>
          <button class="ib sm" title="Close clip mode (Esc)" onClick={() => setClipMode(false)}><Icon name="x" size={18} /></button>
        </div>

        <Timeline />

        <div class="foot">
          <div class="chips num">
            <For each={clip.segs}>
              {(g, i) => (
                <span class="chip" classList={{ on: i() === clip.act }} onClick={() => { setClip("act", i()); seek(g.a); }}>
                  {i() + 1}&nbsp; {fmtL(g.a)} – {fmtL(g.b)}
                  <Show when={clip.segs.length > 1} fallback={<span style={{ width: "4px" }} />}>
                    <span class="x" title="Remove segment" onClick={(e) => { e.stopPropagation(); deleteSegment(i()); }}>
                      <Icon name="x" size={11} />
                    </span>
                  </Show>
                </span>
              )}
            </For>
          </div>
          <div class="spacer" />
          <Note />
        </div>
      </div>
    </div>
  );
}
