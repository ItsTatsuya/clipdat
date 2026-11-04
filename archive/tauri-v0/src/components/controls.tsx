import { createSignal, For, type JSX, onCleanup, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { Icon } from "../lib/icons";

export type Opt<T> = { value: T; label: string; desc?: string; disabled?: boolean; reason?: string; group?: string };

/** Themed dropdown: button + floating glass menu (no native <select>). */
export function Dropdown<const T>(props: {
  value: T;
  options: Opt<T>[];
  onChange: (v: T) => void;
  style?: JSX.CSSProperties;
}) {
  const [open, setOpen] = createSignal(false);
  const [act, setAct] = createSignal(0);
  const [pos, setPos] = createSignal({ top: 0, left: 0, minWidth: 0 });
  let btn!: HTMLButtonElement;
  let menu: HTMLDivElement | undefined;

  const current = () => props.options.find((o) => o.value === props.value);

  const close = () => {
    setOpen(false);
    document.removeEventListener("pointerdown", onOutside, true);
    document.removeEventListener("keydown", onKey, true);
    window.removeEventListener("resize", close);
    document.removeEventListener("scroll", onScroll, true);
  };
  const onOutside = (e: PointerEvent) => {
    if (!menu?.contains(e.target as Node) && !btn.contains(e.target as Node)) close();
  };
  const onScroll = (e: Event) => { if (!menu?.contains(e.target as Node)) close(); };
  const pick = (i: number) => {
    const o = props.options[i];
    if (!o || o.disabled) return;
    props.onChange(o.value);
    close();
    btn.focus();
  };
  const step = (d: number) => {
    const n = props.options.length;
    let k = act();
    for (let i = 0; i < n; i++) {
      k = (k + d + n) % n;
      if (!props.options[k].disabled) break;
    }
    setAct(k);
    menu?.querySelector(`[data-k="${k}"]`)?.scrollIntoView({ block: "nearest" });
  };
  function onKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") step(1);
    else if (e.key === "ArrowUp") step(-1);
    else if (e.key === "Enter" || e.key === " ") pick(act());
    else if (e.key === "Escape" || e.key === "Tab") { close(); btn.focus(); }
    else return;
    e.preventDefault();
    e.stopImmediatePropagation();
  }
  const show = () => {
    const r = btn.getBoundingClientRect();
    setPos({ top: r.bottom + 6, left: r.left, minWidth: r.width });
    setAct(Math.max(0, props.options.findIndex((o) => o.value === props.value)));
    setOpen(true);
    requestAnimationFrame(() => {
      if (!menu) return;
      const h = menu.offsetHeight, w = menu.offsetWidth;
      const below = innerHeight - r.bottom;
      const top = below < h + 12 && r.top > below ? r.top - h - 6 : r.bottom + 6;
      setPos({ top, left: Math.min(r.left, innerWidth - w - 8), minWidth: r.width });
      menu.querySelector(".it.act")?.scrollIntoView({ block: "nearest" });
    });
    document.addEventListener("pointerdown", onOutside, true);
    document.addEventListener("keydown", onKey, true);
    window.addEventListener("resize", close);
    document.addEventListener("scroll", onScroll, true);
  };
  onCleanup(close);

  return (
    <>
      <button
        ref={btn}
        type="button"
        class="dd"
        classList={{ open: open() }}
        style={props.style}
        onClick={() => (open() ? close() : show())}
      >
        <span class="v">{current()?.label ?? "—"}</span>
        <span class="chev"><Icon name="chev" size={15} /></span>
      </button>
      <Show when={open()}>
        <Portal>
          <div ref={menu} class="menu" style={{ top: `${pos().top}px`, left: `${pos().left}px`, "min-width": `${pos().minWidth}px` }}>
            <For each={props.options}>
              {(o, i) => (
                <>
                  <Show when={o.group && (i() === 0 || props.options[i() - 1].group !== o.group)}>
                    <div class="grp">{o.group}</div>
                  </Show>
                  <div
                    class="it"
                    data-k={i()}
                    classList={{ sel: o.value === props.value, dis: !!o.disabled, act: act() === i() }}
                    onPointerMove={() => !o.disabled && setAct(i())}
                    onClick={() => pick(i())}
                  >
                    <span class="ck"><Show when={o.value === props.value}><Icon name="check" size={14} /></Show></span>
                    <span class="tx">
                      {o.label}
                      <Show when={o.disabled ? o.reason : o.desc}><small>{o.disabled ? o.reason : o.desc}</small></Show>
                    </span>
                  </div>
                </>
              )}
            </For>
          </div>
        </Portal>
      </Show>
    </>
  );
}

export function Switch(props: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={props.on}
      class="sw"
      classList={{ on: props.on }}
      onClick={() => props.onChange(!props.on)}
    />
  );
}

export function Seg<const T>(props: {
  value: T;
  options: { value: T; label: string; disabled?: boolean; title?: string }[];
  onChange: (v: T) => void;
  full?: boolean;
}) {
  return (
    <div class="seg" classList={{ full: props.full }}>
      <For each={props.options}>
        {(o) => (
          <button
            type="button"
            classList={{ on: o.value === props.value }}
            disabled={o.disabled}
            title={o.title}
            onClick={() => props.onChange(o.value)}
          >
            {o.label}
          </button>
        )}
      </For>
    </div>
  );
}

export function Toggle(props: { label: string; hint?: string; on: boolean; onChange: (v: boolean) => void }) {
  return (
    <div class="tog">
      <div>
        {props.label}
        <Show when={props.hint}><small>{props.hint}</small></Show>
      </div>
      <Switch on={props.on} onChange={props.onChange} />
    </div>
  );
}
