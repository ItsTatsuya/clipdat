import { invoke } from "@tauri-apps/api/core";
import { For, Match, Show, Switch } from "solid-js";
import { fmtMB } from "../lib/format";
import { Icon } from "../lib/icons";
import { dismissJob, jobs } from "../lib/jobs";
import { toast } from "../lib/ui";

export function JobCards() {
  return (
    <div class="jobs">
      <For each={jobs}>
        {(j) => (
          <div class="job">
            <div class="top">
              <div class="thumb" style={{ "background-image": j.thumb ? `url("${j.thumb}")` : undefined }}>
                <Show when={!j.thumb}><Icon name="film" size={18} /></Show>
              </div>
              <div class="meta">
                <div class="t1">{j.name}.mp4</div>
                <Switch>
                  <Match when={j.state === "running"}>
                    <div class="t2 num">
                      <Show when={j.label !== "lossless"}><Icon name="bolt" size={12} /></Show>
                      {j.phase === "re-encoding" ? "Over the limit, re-encoding smaller" : j.phase === "merging" ? "Merging segments" : j.label === "lossless" ? "Copying streams" : j.label}
                      {` · ${Math.floor(j.progress * 100)}%`}
                      <Show when={j.fps > 0}>{` · ${Math.round(j.fps)} fps`}</Show>
                    </div>
                  </Match>
                  <Match when={j.state === "done"}>
                    <div class="t2 num">
                      <Icon name="check" size={13} />
                      {fmtMB(j.bytes / 1e6)} MB
                      <Show when={j.message === "copied"} fallback={j.outputs.length > 1 ? ` · ${j.outputs.length} files` : " · saved"}>
                        {" · copied · "}<b>Ctrl+V</b>{" in Discord"}
                      </Show>
                    </div>
                  </Match>
                  <Match when={j.state === "error"}><div class="t2 bad">{j.message}</div></Match>
                  <Match when={j.state === "cancelled"}><div class="t2">Cancelled</div></Match>
                </Switch>
                <div class="pbar" classList={{ done: j.state === "done", bad: j.state === "error" }}>
                  <i style={{ width: `${(j.state === "done" ? 1 : j.progress) * 100}%` }} />
                </div>
              </div>
              <Show
                when={j.state === "running"}
                fallback={<button class="ib sm" title="Dismiss" onClick={() => dismissJob(j.id)}><Icon name="x" size={16} /></button>}
              >
                <button class="ib sm" title="Cancel" onClick={() => invoke("export_cancel", { id: j.id })}><Icon name="stop" size={14} /></button>
              </Show>
            </div>
            <Show when={j.state === "done"}>
              <div class="acts">
                <button class="btn" onClick={() => invoke("copy_files", { paths: j.outputs }).then(() => toast("Copied. Paste it in Discord", "info")).catch((e) => toast(String(e)))}>
                  <Icon name="copy" size={15} />Copy again
                </button>
                <button class="btn" onClick={() => invoke("reveal", { path: j.outputs[0] })}><Icon name="folder" size={15} />Show in folder</button>
              </div>
            </Show>
          </div>
        )}
      </For>
    </div>
  );
}
