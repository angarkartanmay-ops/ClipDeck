// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

import { api, type Settings, type Status } from "./api";
import { bytes, escapeHtml } from "./format";
import { icons } from "./icons";

const root = document.getElementById("settings") as HTMLElement;
const ACCENTS = ["#7c5cff", "#3b82f6", "#14b8a6", "#22c55e", "#f59e0b", "#f43f5e", "#ec4899", "#64748b"];

let current: Settings;
let onSaved: (s: Settings) => void = () => {};
let saveTimer = 0;
let confirmClear = false;

export const settingsOpen = () => !root.hidden;

export function closeSettings(): void {
  if (root.hidden) return;
  root.classList.remove("open");
  window.setTimeout(() => (root.hidden = true), 160);
}

export function openSettings(settings: Settings, status: Status, saved: (s: Settings) => void): void {
  current = structuredClone(settings);
  onSaved = saved;
  confirmClear = false;
  root.innerHTML = render(status);
  root.hidden = false;
  requestAnimationFrame(() => root.classList.add("open"));
  wire();
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
  }, 200);
}

const toggle = (key: keyof Settings, label: string, hint = "") => `
  <label class="row">
    <span class="row-text"><span>${label}</span>${hint ? `<small>${hint}</small>` : ""}</span>
    <input type="checkbox" class="switch" data-key="${key}" ${current[key] ? "checked" : ""}/>
  </label>`;

const segmented = (key: keyof Settings, label: string, options: [string, string][]) => `
  <div class="row">
    <span class="row-text"><span>${label}</span></span>
    <div class="segmented" data-key="${key}">
      ${options
        .map(([v, l]) => `<button type="button" data-value="${v}" class="${current[key] === v ? "active" : ""}">${l}</button>`)
        .join("")}
    </div>
  </div>`;

const number = (key: keyof Settings, label: string, min: number, max: number, suffix: string, hint = "") => `
  <label class="row">
    <span class="row-text"><span>${label}</span>${hint ? `<small>${hint}</small>` : ""}</span>
    <span class="number"><input type="number" data-key="${key}" min="${min}" max="${max}" value="${current[key]}"/><em>${suffix}</em></span>
  </label>`;

const hotkey = (key: keyof Settings, label: string, hint = "") => `
  <label class="row">
    <span class="row-text"><span>${label}</span>${hint ? `<small>${hint}</small>` : ""}</span>
    <input type="text" class="hotkey" readonly data-hotkey="${key}" value="${escapeHtml(String(current[key]))}"/>
  </label>`;

function render(status: Status): string {
  const errors = status.hotkeyErrors.length
    ? `<p class="warn-text">${status.hotkeyErrors.map(escapeHtml).join("<br/>")}</p>`
    : "";
  return `
  <div class="settings-head">
    <button class="icon-btn" data-close title="Back (Esc)">${icons.back}</button>
    <h2>Settings</h2>
    <span class="muted">ClipDeck ${escapeHtml(status.version)}</span>
  </div>
  <div class="settings-body">
    <section>
      <h3>General</h3>
      ${toggle("autostart", "Start with Windows")}
      ${toggle("pasteOnSelect", "Paste into the previous app", "Off: choosing an item only copies it")}
      ${toggle("restoreClipboard", "Restore clipboard after pasting", "Puts back what was on the clipboard before")}
      ${segmented("position", "Open dashboard", [["cursor", "Near cursor"], ["center", "Centered"]])}
      ${toggle("keepDashboardWarm", "Keep dashboard ready", "Opens instantly but keeps ~80–120 MB in memory")}
    </section>
    <section>
      <h3>Hotkeys</h3>
      ${hotkey("hotkeyDashboard", "Open dashboard")}
      ${hotkey("hotkeyQueue", "Toggle paste queue", "While on, the dashboard hotkey pastes queued items in order")}
      ${segmented("slotModifiers", "Paste slot 1–9", [["Ctrl+Alt", "Ctrl+Alt"], ["Ctrl+Shift", "Ctrl+Shift"], ["Alt+Shift", "Alt+Shift"], ["", "Off"]])}
      ${errors}
      <p class="muted small">Click a field and press a new combination. Plain Ctrl+C / Ctrl+V are never intercepted.</p>
    </section>
    <section>
      <h3>History</h3>
      ${number("historyLimit", "Items to keep", 10, 200, "items", "Pinned items don't count and are never removed")}
      ${number("maxDiskMb", "Disk budget", 50, 10000, "MB", `Using ${bytes(status.diskBytes)}`)}
      ${number("maxItemMb", "Largest item", 1, 512, "MB")}
      ${toggle("captureImages", "Capture images")}
      ${toggle("captureFiles", "Capture copied files")}
      ${toggle("captureRich", "Keep rich text formatting")}
    </section>
    <section>
      <h3>Privacy</h3>
      ${toggle("encrypt", "Encrypt history on disk", "Windows DPAPI, readable only by your account")}
      ${toggle("clearOnExit", "Clear history on exit and sign-out", "Pinned items are kept")}
      <label class="row column">
        <span class="row-text"><span>Never capture from these apps</span><small>One executable name per line. Copies marked private by password managers are always skipped.</small></span>
        <textarea data-blocklist rows="4" spellcheck="false">${escapeHtml(current.blocklist.join("\n"))}</textarea>
      </label>
      <div class="row buttons">
        <button type="button" class="ghost" data-open-folder>${icons.folder} Open data folder</button>
        <button type="button" class="danger" data-clear>${icons.trash} Clear history</button>
      </div>
      <p class="muted small">Everything stays on this PC. ClipDeck makes no network connections and collects no telemetry.</p>
    </section>
    <section>
      <h3>Appearance</h3>
      ${segmented("theme", "Theme", [["system", "System"], ["light", "Light"], ["dark", "Dark"]])}
      <div class="row">
        <span class="row-text"><span>Accent</span></span>
        <div class="swatches">${ACCENTS.map(
          (c) => `<button type="button" data-accent="${c}" style="--c:${c}" class="${current.accent === c ? "active" : ""}" title="${c}"></button>`,
        ).join("")}</div>
      </div>
      ${segmented("reduceMotion", "Reduce motion", [["system", "System"], ["on", "On"], ["off", "Off"]])}
    </section>
    <section class="about">
      <p>Free for personal and other non-commercial use under the PolyForm Noncommercial License 1.0.0. Commercial use needs a separate license from the author.</p>
      <p class="muted">Copyright © 2026 Tanmay Angarkar</p>
    </section>
  </div>`;
}

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

function wire(): void {
  root.querySelector("[data-close]")?.addEventListener("click", closeSettings);

  root.querySelectorAll<HTMLInputElement>("input.switch").forEach((el) =>
    el.addEventListener("change", () => {
      (current as unknown as Record<string, unknown>)[el.dataset.key!] = el.checked;
      save();
    }),
  );
  root.querySelectorAll<HTMLInputElement>("input[type=number]").forEach((el) =>
    el.addEventListener("change", () => {
      const min = Number(el.min);
      const max = Number(el.max);
      const value = Math.min(max, Math.max(min, Math.round(Number(el.value) || min)));
      el.value = String(value);
      (current as unknown as Record<string, unknown>)[el.dataset.key!] = value;
      save();
    }),
  );
  root.querySelectorAll<HTMLElement>(".segmented").forEach((group) =>
    group.addEventListener("click", (e) => {
      const b = (e.target as HTMLElement).closest<HTMLButtonElement>("button");
      if (!b) return;
      group.querySelectorAll("button").forEach((x) => x.classList.toggle("active", x === b));
      (current as unknown as Record<string, unknown>)[group.dataset.key!] = b.dataset.value;
      save();
    }),
  );
  root.querySelectorAll<HTMLButtonElement>("[data-accent]").forEach((b) =>
    b.addEventListener("click", () => {
      root.querySelectorAll("[data-accent]").forEach((x) => x.classList.toggle("active", x === b));
      current.accent = b.dataset.accent!;
      save();
    }),
  );
  root.querySelectorAll<HTMLInputElement>("input.hotkey").forEach((el) => {
    const key = el.dataset.hotkey as keyof Settings;
    el.addEventListener("focus", () => {
      el.value = "Press keys…";
      el.classList.add("capturing");
    });
    el.addEventListener("blur", () => {
      el.value = String(current[key]);
      el.classList.remove("capturing");
    });
    el.addEventListener("keydown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape" || e.key === "Tab") {
        el.blur();
        return;
      }
      const combo = hotkeyFromEvent(e);
      if (!combo) return;
      (current as unknown as Record<string, unknown>)[key] = combo;
      el.blur();
      save();
    });
  });
  const blocklist = root.querySelector<HTMLTextAreaElement>("[data-blocklist]")!;
  blocklist.addEventListener("change", () => {
    current.blocklist = blocklist.value
      .split(/\r?\n/)
      .map((s) => s.trim().toLowerCase())
      .filter(Boolean);
    save();
  });
  root.querySelector("[data-open-folder]")?.addEventListener("click", () => void api.openDataDir());
  const clear = root.querySelector<HTMLButtonElement>("[data-clear]")!;
  clear.addEventListener("click", async () => {
    if (!confirmClear) {
      confirmClear = true;
      clear.innerHTML = `${icons.trash} Click again to clear (pinned kept)`;
      return;
    }
    confirmClear = false;
    await api.clearHistory(true);
    clear.innerHTML = `${icons.trash} History cleared`;
  });
}
