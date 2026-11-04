import { listen } from "@tauri-apps/api/event";
import { createStore, produce } from "solid-js/store";
import { toast } from "./ui";

export type Job = {
  id: number;
  name: string;
  label: string;
  state: "running" | "done" | "error" | "cancelled";
  phase: string;
  progress: number;
  fps: number;
  outputs: string[];
  bytes: number;
  message: string;
  thumb?: string;
};

export const [jobs, setJobs] = createStore<Job[]>([]);

export function addJob(id: number, name: string, label: string, thumb?: string) {
  setJobs(produce((j) => {
    j.push({ id, name, label, thumb, state: "running", phase: "starting", progress: 0, fps: 0, outputs: [], bytes: 0, message: "" });
    while (j.length > 3) {
      const k = j.findIndex((x) => x.state !== "running");
      if (k < 0) break;
      j.splice(k, 1);
    }
  }));
}

export const dismissJob = (id: number) => setJobs((j) => j.filter((x) => x.id !== id));

export async function initJobs() {
  await listen<Partial<Job> & { id: number }>("job", ({ payload }) => {
    const i = jobs.findIndex((j) => j.id === payload.id);
    if (i < 0) return;
    setJobs(i, (j) => ({
      ...j,
      state: (payload.state as Job["state"]) ?? j.state,
      phase: payload.phase || j.phase,
      progress: payload.progress ?? j.progress,
      fps: payload.fps || j.fps,
      outputs: payload.outputs?.length ? payload.outputs : j.outputs,
      bytes: payload.bytes || j.bytes,
      message: payload.message ?? j.message,
    }));
    if (payload.state === "error") toast(payload.message ?? "Export failed");
    if (payload.state === "cancelled") setTimeout(() => dismissJob(payload.id), 1500);
  });
}
