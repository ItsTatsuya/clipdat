import type { Codec } from "./settings";

/** Minimum bits-per-pixel that still looks good, per codec. */
const BPP: Record<Codec, number> = { h264: 0.04, hevc: 0.028, av1: 0.022 };
const LADDER: [number, number][] = [
  [2160, 60], [1440, 60], [1080, 60], [720, 60], [720, 30], [480, 30],
];

export type Plan = {
  ok: boolean;
  height: number;
  fps: number;
  videoKbps: number;
  audioKbps: number;
  sizeMb: number;
};

/**
 * Picks the best resolution/fps that keeps a clip under `limitMb`.
 * Resolution drops before frame rate (gaming clips want 60 fps).
 */
export function plan(opts: {
  duration: number;
  limitMb: number;
  codec: Codec;
  srcW: number;
  srcH: number;
  srcFps: number;
  audioKbps: number;
}): Plan {
  const dur = Math.max(opts.duration, 0.1);
  const totalK = (opts.limitMb * 8 * 1000 * 0.96) / dur;
  const videoKbps = Math.round(totalK - opts.audioKbps);
  const aspect = opts.srcW && opts.srcH ? opts.srcW / opts.srcH : 16 / 9;
  const srcH = opts.srcH || 1080;
  const srcFps = opts.srcFps || 60;
  const rungs = LADDER.filter(([h]) => h <= srcH).map(([h, f]) => [h, Math.min(f, Math.round(srcFps))] as [number, number]);
  if (!rungs.length || rungs[0][0] < srcH) rungs.unshift([srcH, Math.min(60, Math.round(srcFps))]);
  let pick: [number, number] | null = null;
  for (const [h, f] of rungs) {
    const w = Math.round((h * aspect) / 2) * 2;
    if ((videoKbps * 1000) / (w * h * f) >= BPP[opts.codec]) { pick = [h, f]; break; }
  }
  return {
    ok: !!pick && videoKbps > 300,
    height: pick ? pick[0] : 480,
    fps: pick ? pick[1] : 30,
    videoKbps: Math.max(videoKbps, 100),
    audioKbps: opts.audioKbps,
    sizeMb: opts.limitMb * 0.95,
  };
}

/** Rough size for constant-quality mode (MB). */
export function estimateQuality(duration: number, quality: number, codec: Codec, height: number) {
  const scale = Math.pow(height / 1080, 1.4);
  const mbps = (4 + quality * 0.32) * { h264: 1, hevc: 0.7, av1: 0.6 }[codec] * scale;
  return (duration * mbps) / 8;
}

export const cqFor = (q: number) => Math.round(34 - q * 0.18);
