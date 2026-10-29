// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

import { api, type Settings, type Status } from "./api";
import { bytes, escapeHtml } from "./format";
import { icons } from "./icons";

const root = document.getElementById("settings") as HTMLElement;

/** Accents kept below ~80% saturation so they sit calmly on the glass. */
const ACCENTS = ["#4a86e8", "#2fa6a0", "#3fae78", "#d99a2b", "#e0705a", "#d9577f", "#8b6fd6", "#7a8494"];

type Page = "general" | "shortcuts" | "history" | "privacy" | "appearance" | "about";

const PAGES: { id: Page; label: string; icon: string }[] = [
  { id: "general", label: "General", icon: "sliders" },
  { id: "shortcuts", label: "Shortcuts", icon: "keyboard" },
  { id: "history", label: "History", icon: "history" },
  { id: "privacy", label: "Privacy", icon: "shield" },
  { id: "appearance", label: "Appearance", icon: "palette" },
  { id: "about", label: "About", icon: "info" },
];

let current: Settings;
let status: Status;
let page: Page = "general";
let onSaved: (s: Settings) => void = () => {};
let saveTimer = 0;
let clearArmed = false;

export const settingsOpen = () => !root.hidden;

const panel = root.closest<HTMLElement>(".panel")!;

export function closeSettings(): void {
  if (root.hidden) return;
  root.classList.remove("open");
  panel.classList.remove("settings-open");
  window.setTimeout(() => (root.hidden = true), 180);
}

export function openSettings(settings: Settings, st: Status, saved: (s: Settings) => void, start: Page = "general"): void {
  current = structuredClone(settings);
  status = st;
  onSaved = saved;
  page = start;
  root.innerHTML = shell();
  root.hidden = false;
  renderPage();
  requestAnimationFrame(() => {
    root.classList.add("open");
    panel.classList.add("settings-open");
  });
}

function shell(): string {
  return `
  <div class="settings-head">
    <button class="tool" type="button" data-close title="Back (Esc)" aria-label="Back">${icons.back}</button>
    <h2>Settings</h2>
    <span class="muted">v${escapeHtml(status.version)}</span>
  </div>
  <nav class="settings-nav" aria-label="Settings sections">
    <span class="nav-indicator" aria-hidden="true"></span>
    ${PAGES.map((p) => `<button type="button" data-page="${p.id}">${icons[p.icon]}${p.label}</button>`).join("")}
  </nav>
  <div class="settings-page"></div>`;
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
  }, 180);
}

// ---------------------------------------------------------------- controls

const set = (key: keyof Settings, value: unknown) => {
  (current as unknown as Record<string, unknown>)[key] = value;
  save();
};

const toggle = (key: keyof Settings, label: string, hint = "") => `
  <label class="setting">
    <span class="setting-text"><span>${label}</span>${hint ? `<small>${hint}</small>` : ""}</span>
    <input type="checkbox" class="switch" data-key="${key}" ${current[key] ? "checked" : ""}/>
  </label>`;

const segmented = (key: keyof Settings, label: string, options: [string, string][], hint = "") => `
  <div class="setting">
    <span class="setting-text"><span>${label}</span>${hint ? `<small>${hint}</small>` : ""}</span>
    <div class="segmented" data-key="${key}" role="radiogroup" aria-label="${label}">
      ${options
        .map(
          ([v, l]) =>
            `<button type="button" role="radio" aria-checked="${current[key] === v}" data-value="${v}" class="${current[key] === v ? "active" : ""}">${l}</button>`,
        )
        .join("")}
    </div>
  </div>`;

const stepper = (key: keyof Settings, label: string, min: number, max: number, step: number, unit: string, hint = "") => `
  <div class="setting">
    <span class="setting-text"><span>${label}</span>${hint ? `<small>${hint}</small>` : ""}</span>
    <span class="stepper" data-key="${key}" data-min="${min}" data-max="${max}" data-step="${step}">
      <button type="button" data-delta="-1" aria-label="Decrease">−</button>
      <input type="number" min="${min}" max="${max}" value="${current[key]}" aria-label="${label}"/>
      <button type="button" data-delta="1" aria-label="Increase">+</button>
      <em>${unit}</em>
    </span>
  </div>`;

const keycaps = (combo: string) =>
  combo
    ? combo
        .split("+")
        .map((k) => `<kbd class="key">${escapeHtml(k)}</kbd>`)
        .join('<span class="plus">+</span>')
    : `<span class="muted">Off</span>`;

const recorder = (key: keyof Settings, label: string, hint = "") => `
  <div class="setting">
    <span class="setting-text"><span>${label}</span>${hint ? `<small>${hint}</small>` : ""}</span>
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
        ${toggle("pasteOnSelect", "Paste into the previous app", "Off: choosing an item only copies it")}
        ${toggle("restoreClipboard", "Restore clipboard after pasting", "Puts back whatever you had copied before")}
        ${segmented("position", "Open the deck", [
          ["cursor", "At cursor"],
          ["center", "Centered"],
        ])}
        ${toggle("keepDashboardWarm", "Keep the deck ready", "Opens instantly, keeps ~120 MB in memory")}`;
    case "shortcuts": {
      const errors = status.hotkeyErrors.length
        ? `<p class="note warn">${status.hotkeyErrors.map(escapeHtml).join("<br/>")}</p>`
        : "";
      return `
        ${recorder("hotkeyDashboard", "Open the deck")}
        ${recorder("hotkeyQueue", "Toggle paste queue", "While on, the deck hotkey pastes queued items in order")}
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
        ${stepper("historyLimit", "Items to keep", 10, 200, 10, "", "Pinned items are extra and never removed")}
        ${stepper("maxDiskMb", "Disk budget", 50, 10000, 50, "MB", `${bytes(status.diskBytes)} in use`)}
        ${stepper("maxItemMb", "Largest item", 1, 512, 8, "MB")}
        ${toggle("captureImages", "Capture images")}
        ${toggle("captureFiles", "Capture copied files")}
        ${toggle("captureRich", "Keep rich text formatting")}`;
    case "privacy":
      return `
        ${toggle("encrypt", "Encrypt history on disk", "Windows DPAPI: readable only by your account")}
        ${toggle("clearOnExit", "Clear on exit and sign-out", "Pinned items are kept")}
        <label class="setting stack">
          <span class="setting-text"><span>Never capture from</span><small>One app per line. Copies marked private by password managers are always skipped.</small></span>
          <textarea data-blocklist rows="4" spellcheck="false">${escapeHtml(current.blocklist.join("\n"))}</textarea>
        </label>
        <div class="button-row">
          <button type="button" class="btn danger${clearArmed ? " armed" : ""}" data-clear>${icons.trash}${clearArmed ? "Click again to clear" : "Clear history"}</button>
          <button type="button" class="btn" data-open-folder>${icons.folder}Open data folder</button>
        </div>
        <p class="note">Everything stays on this PC. ClipDeck makes no network connections and has no telemetry.</p>`;
    case "appearance":
      return `
        ${segmented("theme", "Theme", [
          ["system", "System"],
          ["light", "Light"],
          ["dark", "Dark"],
        ])}
        <div class="setting">
          <span class="setting-text"><span>Accent</span><small>Tints the glass gradient and highlights</small></span>
          <div class="swatches" role="radiogroup" aria-label="Accent colour">${ACCENTS.map(
            (c) =>
              `<button type="button" role="radio" aria-checked="${current.accent === c}" aria-label="${c}" data-accent="${c}" style="--c:${c}" class="${
                current.accent === c ? "active" : ""
              }"></button>`,
          ).join("")}</div>
        </div>
        ${segmented("reduceMotion", "Reduce motion", [
          ["system", "System"],
          ["on", "On"],
          ["off", "Off"],
        ])}`;
    case "about":
      return `
        <div class="about-card"><span class="logo">${icons.paste}</span><div><h3>ClipDeck</h3><p>Version ${escapeHtml(status.version)}</p></div></div>
        <div class="prose-sm">
          <p>Free for personal and other non-commercial use under the PolyForm Noncommercial License 1.0.0. Commercial use needs a separate licence from the author.</p>
          <p>Copyright © 2026 Tanmay Angarkar. Provided as is, without warranty.</p>
        </div>`;
  }
}

function renderPage(): void {
  const index = PAGES.findIndex((p) => p.id === page);
  root.querySelector<HTMLElement>(".settings-nav")!.style.setProperty("--nav", String(index));
  root.querySelectorAll<HTMLButtonElement>("[data-page]").forEach((b) => {
    b.classList.toggle("active", b.dataset.page === page);
    b.setAttribute("aria-current", String(b.dataset.page === page));
  });
  const body = root.querySelector<HTMLElement>(".settings-page")!;
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

root.addEventListener("click", (e) => {
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

  const accent = t.closest<HTMLButtonElement>("[data-accent]");
  if (accent) {
    root.querySelectorAll("[data-accent]").forEach((b) => {
      b.classList.toggle("active", b === accent);
      b.setAttribute("aria-checked", String(b === accent));
    });
    document.documentElement.style.setProperty("--accent", accent.dataset.accent!);
    return set("accent", accent.dataset.accent);
  }

  const delta = t.closest<HTMLButtonElement>("[data-delta]");
  if (delta) {
    const box = delta.closest<HTMLElement>(".stepper")!;
    const input = box.querySelector("input")!;
    const step = Number(box.dataset.step);
    const value = clamp(Number(input.value) + Number(delta.dataset.delta) * step, box);
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

function clamp(value: number, box: HTMLElement): number {
  const min = Number(box.dataset.min);
  const max = Number(box.dataset.max);
  return Math.min(max, Math.max(min, Math.round(value || min)));
}

root.addEventListener("change", (e) => {
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

root.addEventListener("keydown", (e) => {
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

root.addEventListener("focusout", (e) => {
  if (recording && e.target === recording) stopRecording();
});
