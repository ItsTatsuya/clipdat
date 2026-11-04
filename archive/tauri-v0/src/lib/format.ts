const pad = (n: number) => String(n).padStart(2, "0");

/** 1:23 / 1:02:03 */
export function fmtS(t: number): string {
  t = Math.max(0, t || 0);
  const h = Math.floor(t / 3600), m = Math.floor((t % 3600) / 60), s = Math.floor(t % 60);
  return h ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

/** 01:23.45 (hours prefixed when needed) */
export function fmtL(t: number): string {
  t = Math.max(0, t || 0);
  const h = Math.floor(t / 3600), m = Math.floor((t % 3600) / 60), s = t % 60;
  const ss = s.toFixed(2).padStart(5, "0");
  return h ? `${h}:${pad(m)}:${ss}` : `${pad(m)}:${ss}`;
}

/** One decimal below 100 MB, truncated so 19.96 never reads as "20.0" against a 20 MB limit. */
export const fmtMB = (v: number) => (v < 100 ? (Math.floor(v * 10) / 10).toFixed(1) : String(Math.round(v)));

export const basename = (p: string) => p.split(/[\\/]/).pop() ?? p;
export const stem = (p: string) => basename(p).replace(/\.[^.]+$/, "");

/** Index of the last element <= t in a sorted array (or -1). */
export function floorIndex(arr: number[], t: number): number {
  let lo = 0, hi = arr.length - 1, r = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (arr[mid] <= t + 1e-6) { r = mid; lo = mid + 1; } else hi = mid - 1;
  }
  return r;
}

export function nearest<T extends { t: number }>(arr: T[], t: number): T | undefined {
  if (!arr.length) return undefined;
  const i = floorIndex(arr.map((x) => x.t), t);
  if (i < 0) return arr[0];
  if (i >= arr.length - 1) return arr[arr.length - 1];
  return t - arr[i].t < arr[i + 1].t - t ? arr[i] : arr[i + 1];
}
