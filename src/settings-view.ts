// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

import { api, type Settings, type Status } from "./api";
import { bytes, escapeHtml } from "./format";
import { icons, mark } from "./icons";

const layer = document.getElementById("settings") as HTMLElement;
const app = document.getElementById("app") as HTMLElement;

type Page = "general" | "display" | "shortcuts" | "history" | "privacy" | "about";

const PAGES: { id: Page; label: string; icon: string }[] = [
  { id: "general", label: "General", icon: "sliders" },
  { id: "display", label: "Display", icon: "display" },
  { id: "shortcuts", label: "Shortcuts", icon: "keyboard" },
  { id: "history", label: "History", icon: "history" },
  { id: "privacy", label: "Privacy", icon: "shield" },
  { id: "about", label: "About", icon: "info" },
];

let current: Settings;
let status: Status;
let page: Page = "general";
let onSaved: (s: Settings) => void = () => {};
let saveTimer = 0;
let clearArmed = false;

export const settingsOpen = () => !layer.hidden;

export function closeSettings(): void {
  if (layer.hidden || layer.classList.contains("closing")) return;
  layer.classList.add("closing");
  app.classList.remove("covered");
  window.setTimeout(() => {
    layer.hidden = true;
    layer.classList.remove("closing");
    layer.innerHTML = "";
  }, 170);
}

export function openSettings(settings: Settings, st: Status, saved: (s: Settings) => void, start: Page = "general"): void {
  current = structuredClone(settings);
  status = st;
  onSaved = saved;
  page = start;
  clearArmed = false;
  layer.classList.remove("closing");
  layer.innerHTML = shell();
  layer.hidden = false;
  app.classList.add("covered");
  renderPage();
}

function shell(): string {
  return `
  <div class="sheet-backdrop" data-close></div>
  <section class="sheet" role="dialog" aria-label="Settings">
    <header class="sheet-head">
      <h2>Settings</h2>
      <span class="ver">v${escapeHtml(status.version)}</span>
      <button class="tool" type="button" data-close title="Done (Esc)" aria-label="Close settings">${icons.close}</button>
    </header>
    <nav class="sheet-nav" aria-label="Settings sections">
      <span class="nav-indicator" aria-hidden="true"></span>
      ${PAGES.map((p) => `<button type="button" data-page="${p.id}">${icons[p.icon]}${p.label}</button>`).join("")}
    </nav>
    <div class="sheet-page"></div>
  </section>`;
}

function save(): void {
  clearTimeout(saveTimer);
  saveTimer = window.setTimeout(async () => {
    try {
      current = await api.saveSettings(current);
      onSaved(current);
    } catch (err) {
      console.error("save settings", err);
    }
  }, 160);
}

// ---------------------------------------------------------------- controls

const set = (key: keyof Settings, value: unknown) => {
  (current as unknown as Record<string, unknown>)[key] = value;
  save();
};

const hint = (text: string) => (text ? `<small>${text}</small>` : "");

const toggle = (key: keyof Settings, label: string, help = "") => `
  <label class="setting">
    <span class="setting-text"><span>${label}</span>${hint(help)}</span>
    <input type="checkbox" class="switch" data-key="${key}" ${current[key] ? "checked" : ""}/>
  </label>`;

const segmented = (key: keyof Settings, label: string, options: [string, string, string?][], help = "") => `
  <div class="setting">
    <span class="setting-text"><span>${label}</span>${hint(help)}</span>
    <div class="segmented" data-key="${key}" role="radiogroup" aria-label="${label}">
      ${options
        .map(
          ([v, l, icon]) =>
            `<button type="button" role="radio" aria-checked="${current[key] === v}" data-value="${v}" class="${current[key] === v ? "active" : ""}">${
              icon ? icons[icon] : ""
            }${l}</button>`,
        )
        .join("")}
    </div>
  </div>`;

const stepper = (key: keyof Settings, label: string, min: number, max: number, step: number, unit: string, help = "") => `
  <div class="setting">
    <span class="setting-text"><span>${label}</span>${hint(help)}</span>
    <span class="stepper" data-key="${key}" data-min="${min}" data-max="${max}" data-step="${step}">
      <button type="button" data-delta="-1" aria-label="Decrease">−</button>
      <input type="number" min="${min}" max="${max}" value="${current[key]}" aria-label="${label}"/>
      <button type="button" data-delta="1" aria-label="Increase">+</button>
      ${unit ? `<em>${unit}</em>` : ""}
    </span>
  </div>`;

const keycaps = (combo: string) =>
  combo
    ? combo
        .split("+")
        .map((k) => `<kbd class="key">${escapeHtml(k)}</kbd>`)
        .join('<span class="plus">+</span>')
    : `<span>Off</span>`;

const recorder = (key: keyof Settings, label: string, help = "") => `
  <div class="setting">
    <span class="setting-text"><span>${label}</span>${hint(help)}</span>
    <button type="button" class="recorder" data-hotkey="${key}" aria-label="${label}: ${escapeHtml(String(current[key]))}. Click to change">${keycaps(
      String(current[key]),
    )}</button>
  </div>`;

// ---------------------------------------------------------------- pages

function pageHtml(): string {
  switch (page) {
    case "general":
      return `
        ${toggle("autostart", "Start with Windows")}
        ${toggle("pasteOnSelect", "Paste into the previous app", "Off: choosing a clip only copies it")}
        ${toggle("restoreClipboard", "Restore clipboard after pasting", "Puts back whatever you had copied before")}
        ${toggle("keepDashboardWarm", "Keep the deck ready", "Opens instantly, keeps ~120 MB in memory")}`;
    case "display":
      return `
        ${segmented(
          "displayMode",
          "Show clips",
          [
            ["fullscreen", "Full screen", "maximize"],
            ["window", "Window", "windowed"],
          ],
          "Full screen covers the display like Mission Control. Window can be dragged by its top bar and resized from any edge.",
        )}
        ${segmented("reduceMotion", "Reduce motion", [
          ["system", "System"],
          ["on", "On"],
          ["off", "Off"],
        ])}
        <p class="note">Double-click the window's top bar to go back to full screen. The window remembers where you left it.</p>`;
    case "shortcuts": {
      const errors = status.hotkeyErrors.length
        ? `<p class="note warn">${status.hotkeyErrors.map(escapeHtml).join("<br/>")}</p>`
        : "";
      return `
        ${recorder("hotkeyDashboard", "Open the deck")}
        ${recorder("hotkeyQueue", "Toggle paste queue", "While on, the deck shortcut pastes queued clips in order")}
        ${segmented("slotModifiers", "Paste slots 1–9", [
          ["Ctrl+Alt", "Ctrl Alt"],
          ["Ctrl+Shift", "Ctrl ⇧"],
          ["Alt+Shift", "Alt ⇧"],
          ["", "Off"],
        ])}
        ${errors}
        <p class="note">Click a shortcut, then press the new keys. Plain Ctrl+C and Ctrl+V are never touched.</p>`;
    }
    case "history":
      return `
        ${stepper("historyLimit", "Clips to keep", 10, 200, 10, "", "Pinned clips are extra and never removed")}
        ${stepper("maxDiskMb", "Disk budget", 50, 10000, 50, "MB", `${bytes(status.diskBytes)} in use`)}
        ${stepper("maxItemMb", "Largest clip", 1, 512, 8, "MB")}
        ${toggle("captureImages", "Capture images")}
        ${toggle("captureFiles", "Capture copied files")}
        ${toggle("captureRich", "Keep rich text formatting")}`;
    case "privacy":
      return `
        ${toggle("encrypt", "Encrypt history on disk", "Windows DPAPI: readable only by your account")}
        ${toggle("clearOnExit", "Clear on exit and sign-out", "Pinned clips are kept")}
        <label class="setting stack">
          <span class="setting-text"><span>Never capture from</span><small>One app per line. Copies marked private by password managers are always skipped.</small></span>
          <textarea data-blocklist rows="4" spellcheck="false">${escapeHtml(current.blocklist.join("\n"))}</textarea>
        </label>
        <div class="button-row">
          <button type="button" class="btn${clearArmed ? " armed" : ""}" data-clear>${icons.trash}${clearArmed ? "Click again to clear" : "Clear history"}</button>
          <button type="button" class="btn" data-open-folder>${icons.folder}Open data folder</button>
        </div>
        <p class="note">Everything stays on this PC. ClipDeck makes no network connections and has no telemetry.</p>`;
    case "about":
      return `
        <div class="about-card"><span class="logo">${mark}</span><div><h3>ClipDeck</h3><p>Version ${escapeHtml(status.version)}</p></div></div>
        <div class="prose-sm">
          <p>Free for personal and other non-commercial use under the PolyForm Noncommercial License 1.0.0. Commercial use needs a separate licence from the author.</p>
          <p>Copyright © 2026 Tanmay Angarkar. Provided as is, without warranty.</p>
        </div>`;
  }
}

function renderPage(): void {
  const index = PAGES.findIndex((p) => p.id === page);
  layer.querySelector<HTMLElement>(".sheet-nav")!.style.setProperty("--nav", String(index));
  layer.querySelectorAll<HTMLButtonElement>("[data-page]").forEach((b) => {
    b.classList.toggle("active", b.dataset.page === page);
    b.setAttribute("aria-current", String(b.dataset.page === page));
  });
  const body = layer.querySelector<HTMLElement>(".sheet-page")!;
  body.innerHTML = pageHtml();
  body.scrollTop = 0;
}

// ---------------------------------------------------------------- events

function hotkeyFromEvent(e: KeyboardEvent): string | null {
  const key = e.key;
  if (["Control", "Shift", "Alt", "Meta"].includes(key)) return null;
  let name: string | null = null;
  if (/^[a-z0-9]$/i.test(key)) name = key.toUpperCase();
  else if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) name = key;
  else if (key === " ") name = "Space";
  else if (["Insert", "Home", "End", "PageUp", "PageDown"].includes(key)) name = key;
  if (!name) return null;
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Win"].filter(Boolean);
  if (!mods.length) return null;
  return [...mods, name].join("+");
}

let recording: HTMLButtonElement | null = null;

function stopRecording(): void {
  if (!recording) return;
  const key = recording.dataset.hotkey as keyof Settings;
  recording.classList.remove("capturing");
  recording.innerHTML = keycaps(String(current[key]));
  recording = null;
}

function clamp(value: number, box: HTMLElement): number {
  const min = Number(box.dataset.min);
  const max = Number(box.dataset.max);
  return Math.min(max, Math.max(min, Math.round(value || min)));
}

layer.addEventListener("click", (e) => {
  const t = e.target as HTMLElement;
  if (t.closest("[data-close]")) return closeSettings();

  const nav = t.closest<HTMLButtonElement>("[data-page]");
  if (nav) {
    page = nav.dataset.page as Page;
    clearArmed = false;
    return renderPage();
  }

  const seg = t.closest<HTMLButtonElement>(".segmented button");
  if (seg) {
    const group = seg.parentElement!;
    group.querySelectorAll("button").forEach((b) => {
      b.classList.toggle("active", b === seg);
      b.setAttribute("aria-checked", String(b === seg));
    });
    return set(group.dataset.key as keyof Settings, seg.dataset.value);
  }

  const delta = t.closest<HTMLButtonElement>("[data-delta]");
  if (delta) {
    const box = delta.closest<HTMLElement>(".stepper")!;
    const input = box.querySelector("input")!;
    const value = clamp(Number(input.value) + Number(delta.dataset.delta) * Number(box.dataset.step), box);
    input.value = String(value);
    return set(box.dataset.key as keyof Settings, value);
  }

  const rec = t.closest<HTMLButtonElement>(".recorder");
  if (rec) {
    stopRecording();
    recording = rec;
    rec.classList.add("capturing");
    rec.textContent = "Press keys…";
    rec.focus();
    return;
  }

  if (t.closest("[data-open-folder]")) return void api.openDataDir();

  const clear = t.closest<HTMLButtonElement>("[data-clear]");
  if (clear) {
    if (!clearArmed) {
      clearArmed = true;
      clear.classList.add("armed");
      clear.innerHTML = `${icons.trash}Click again to clear`;
      return;
    }
    clearArmed = false;
    void api.clearHistory(true).then(() => {
      clear.classList.remove("armed");
      clear.innerHTML = `${icons.trash}History cleared`;
    });
  }
});

layer.addEventListener("change", (e) => {
  const t = e.target as HTMLElement;
  if (t instanceof HTMLInputElement && t.classList.contains("switch")) return set(t.dataset.key as keyof Settings, t.checked);
  if (t instanceof HTMLInputElement && t.type === "number") {
    const box = t.closest<HTMLElement>(".stepper")!;
    const value = clamp(Number(t.value), box);
    t.value = String(value);
    return set(box.dataset.key as keyof Settings, value);
  }
  if (t instanceof HTMLTextAreaElement && t.dataset.blocklist !== undefined) {
    return set(
      "blocklist",
      t.value
        .split(/\r?\n/)
        .map((s) => s.trim().toLowerCase())
        .filter(Boolean),
    );
  }
});

layer.addEventListener("keydown", (e) => {
  if (!recording) return;
  e.preventDefault();
  e.stopPropagation();
  if (e.key === "Escape" || e.key === "Tab") return stopRecording();
  const combo = hotkeyFromEvent(e);
  if (!combo) return;
  (current as unknown as Record<string, unknown>)[recording.dataset.hotkey!] = combo;
  stopRecording();
  save();
});

layer.addEventListener("focusout", (e) => {
  if (recording && e.target === recording) stopRecording();
});
