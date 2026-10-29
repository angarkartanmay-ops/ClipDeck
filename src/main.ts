// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

import "./styles.css";
import { api, inTauri, on, type Item, type Settings, type Status } from "./api";
import { bytes, escapeHtml, timeAgo } from "./format";
import { fuzzyScore } from "./fuzzy";
import { hydrateIcons, icons } from "./icons";
import { openSettings, closeSettings, settingsOpen } from "./settings-view";

type Filter = "all" | "text" | "image" | "files" | "pinned";

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "text", label: "Text" },
  { id: "image", label: "Images" },
  { id: "files", label: "Files" },
  { id: "pinned", label: "Pinned" },
];

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const panel = $("panel");
const grid = $("grid");
const search = $<HTMLInputElement>("search");
const filtersNav = $("filters");
const banner = $("banner");
const hints = $("hints");
const footerStatus = $("footer-status");
const toastEl = $("toast");

let items: Item[] = [];
let view: Item[] = [];
let settings: Settings;
let status: Status;
let filter: Filter = "all";
let query = "";
/** In search mode every printable key types; otherwise letters are shortcuts. */
let searchMode = false;
let selected = 0;
let closing = false;
let bannerDismissed = false;
let mode = new URLSearchParams(location.search).get("mode") ?? "";
const thumbs = new Map<number, string>();

// ---------------------------------------------------------------- appearance

const media = {
  dark: matchMedia("(prefers-color-scheme: dark)"),
  reduce: matchMedia("(prefers-reduced-motion: reduce)"),
};

export function applyAppearance(s: Settings): void {
  const root = document.documentElement;
  root.dataset.theme = s.theme === "system" ? (media.dark.matches ? "dark" : "light") : s.theme;
  root.style.setProperty("--accent", s.accent);
  const reduce = s.reduceMotion === "on" || (s.reduceMotion === "system" && media.reduce.matches);
  root.dataset.motion = reduce ? "reduce" : "full";
}

const motionReduced = () => document.documentElement.dataset.motion === "reduce";

// ---------------------------------------------------------------- data

async function loadItems(): Promise<void> {
  const keepId = view[selected]?.id;
  items = await api.listItems();
  const live = new Set(items.map((i) => i.id));
  for (const id of thumbs.keys()) if (!live.has(id)) thumbs.delete(id);
  render(keepId);
}

async function loadStatus(): Promise<void> {
  status = await api.getStatus();
  renderChrome();
}

function computeView(): Item[] {
  let list = items;
  if (filter === "pinned") list = list.filter((i) => i.pinned);
  else if (filter === "text") list = list.filter((i) => i.kind === "text" || i.kind === "rich");
  else if (filter !== "all") list = list.filter((i) => i.kind === filter);
  const q = query.trim();
  if (!q) return list;
  return list
    .map((item) => ({ item, score: fuzzyScore(q, `${item.text}\n${item.sourceApp}`) }))
    .filter((r): r is { item: Item; score: number } => r.score !== null)
    .sort((a, b) => b.score - a.score)
    .map((r) => r.item);
}

// ---------------------------------------------------------------- rendering

function cardBody(item: Item): string {
  switch (item.kind) {
    case "image": {
      const dims = item.meta.width ? `${item.meta.width}×${item.meta.height}` : "";
      return `<div class="thumb"><img data-thumb="${item.id}" alt="" draggable="false"/><span class="dims">${dims}</span></div>`;
    }
    case "files": {
      const names = item.meta.files ?? [];
      const more = (item.meta.fileCount ?? names.length) - names.length;
      const rows = names
        .slice(0, 4)
        .map((n) => `<li>${icons.file}<span>${escapeHtml(n)}</span></li>`)
        .join("");
      const extra = more + Math.max(0, names.length - 4);
      return `<ul class="files">${rows}${extra > 0 ? `<li class="more">+${extra} more</li>` : ""}</ul>`;
    }
    default: {
      const text = item.text.trim();
      const color = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i.test(text) ? text : null;
      if (color) {
        return `<div class="color"><span class="swatch" style="background:${color}"></span><code>${escapeHtml(text)}</code></div>`;
      }
      const isLink = /^https?:\/\/\S+$/i.test(text);
      const code = /[{};]|=>|^\s{2,}\S/m.test(text) && !isLink;
      return `<pre class="snippet${isLink ? " link" : ""}${code ? " code" : ""}">${escapeHtml(text.slice(0, 600))}</pre>`;
    }
  }
}

function card(item: Item, index: number): string {
  const queuePos = status?.queue.indexOf(item.id) ?? -1;
  const slot = index < 9 ? `<span class="slot">${index + 1}</span>` : "";
  const queued = queuePos >= 0 ? `<span class="queued" title="Queued">Q${queuePos + 1}</span>` : "";
  const source = escapeHtml(item.sourceApp || "Unknown app");
  return `<article class="card kind-${item.kind}${index === selected ? " selected" : ""}${item.pinned ? " is-pinned" : ""}"
      role="option" data-index="${index}" style="--i:${Math.min(index, 14)}" aria-selected="${index === selected}">
    <div class="card-inner">
      <div class="card-head">
        <span class="kind">${icons[item.kind]}</span>
        <span class="source" title="${escapeHtml(item.sourceExe)}">${source}</span>
        ${item.pinned ? `<span class="pin-flag" title="Pinned">${icons.pinned}</span>` : ""}
        <span class="time" title="${new Date(item.createdAt).toLocaleString()}">${timeAgo(item.createdAt)}</span>
      </div>
      <div class="card-body">${cardBody(item)}</div>
      <div class="card-actions">
        <button data-act="pin" title="${item.pinned ? "Unpin" : "Pin"} (P)">${icons.pin}</button>
        <button data-act="queue" title="Add to paste queue (Q)">${icons.queue}</button>
        <button data-act="copy" title="Copy only (Ctrl+C)">${icons.copy}</button>
        <button data-act="delete" title="Delete (Del)">${icons.trash}</button>
      </div>
      ${slot}${queued}
    </div>
  </article>`;
}

function emptyState(): string {
  if (items.length === 0) {
    return `<div class="empty">
      <div class="empty-art">${icons.sparkle}</div>
      <h2>Nothing copied yet</h2>
      <p>Copy some text, an image or files, then press <kbd>${escapeHtml(settings.hotkeyDashboard)}</kbd> to bring ClipDeck back.</p>
    </div>`;
  }
  const what = query ? `No matches for “${escapeHtml(query)}”` : "Nothing in this view";
  return `<div class="empty"><div class="empty-art">${icons.search}</div><h2>${what}</h2><p>Try another filter or search term.</p></div>`;
}

function render(keepId?: number): void {
  view = computeView();
  if (keepId !== undefined) {
    const idx = view.findIndex((i) => i.id === keepId);
    selected = idx >= 0 ? idx : Math.min(selected, view.length - 1);
  }
  selected = Math.max(0, Math.min(selected, view.length - 1));
  grid.innerHTML = view.length ? view.map(card).join("") : emptyState();
  loadThumbs();
  renderChrome();
}

function loadThumbs(): void {
  grid.querySelectorAll<HTMLImageElement>("img[data-thumb]").forEach(async (img) => {
    const id = Number(img.dataset.thumb);
    let src = thumbs.get(id);
    if (!src) {
      src = (await api.getThumb(id)) ?? "";
      if (src) thumbs.set(id, src);
    }
    if (src) {
      img.onload = () => img.classList.add("loaded");
      img.src = src;
    }
  });
}

function renderChrome(): void {
  if (!settings || !status) return;
  filtersNav.querySelectorAll<HTMLButtonElement>("button").forEach((b) => {
    const active = b.dataset.filter === filter;
    b.classList.toggle("active", active);
    b.setAttribute("aria-selected", String(active));
  });

  const queueBtn = $("queue-toggle");
  queueBtn.classList.toggle("on", status.queueActive);
  queueBtn.title = status.queueActive
    ? `Paste queue on — ${settings.hotkeyDashboard} pastes the next item (click to turn off)`
    : `Paste queue (${settings.hotkeyQueue})`;
  $("queue-count").textContent = status.queueActive ? String(status.queue.length) : "";

  const pauseBtn = $("pause-toggle");
  pauseBtn.classList.toggle("on", status.paused);
  pauseBtn.innerHTML = status.paused ? icons.play : icons.pause;
  pauseBtn.title = status.paused ? "Capture paused — click to resume" : "Pause capture";

  const verb = status.canPaste ? "paste" : "copy";
  hints.innerHTML = searchMode
    ? `<span><kbd>↵</kbd> ${verb}</span><span><kbd>Alt</kbd><kbd>1–9</kbd> quick ${verb}</span><span><kbd>Ctrl</kbd><kbd>P</kbd> pin</span><span><kbd>Esc</kbd> clear</span>`
    : `<span><kbd>↵</kbd> ${verb}</span><span><kbd>⇧↵</kbd> plain text</span><span><kbd>1–9</kbd> quick ${verb}</span><span><kbd>P</kbd> pin</span><span><kbd>Q</kbd> queue</span><span><kbd>Del</kbd> remove</span><span><kbd>/</kbd> search</span>`;

  const parts = [`${status.itemCount} item${status.itemCount === 1 ? "" : "s"}`, bytes(status.diskBytes)];
  footerStatus.innerHTML = parts.join(" · ") + (status.paused ? ` <span class="chip warn">Capture paused</span>` : "");

  renderBanner();
}

function renderBanner(): void {
  let html = "";
  if (status.hotkeyErrors.length) {
    html = `${icons.shield}<div><b>Some hotkeys could not be registered.</b> ${status.hotkeyErrors
      .map(escapeHtml)
      .join("; ")}. Change them in Settings.</div>`;
  } else if (mode === "welcome" && !bannerDismissed) {
    html = `${icons.sparkle}<div><b>ClipDeck is running in your tray.</b> Copy anything as usual, then press <kbd>${escapeHtml(
      settings.hotkeyDashboard,
    )}</kbd> to pick what to paste. Plain <kbd>Ctrl+V</kbd> is unchanged.</div><button class="ghost" data-dismiss>Got it</button>`;
  }
  banner.hidden = !html;
  banner.innerHTML = html;
}

function updateSelection(next: number, scroll = true): void {
  if (!view.length) return;
  selected = Math.max(0, Math.min(next, view.length - 1));
  grid.querySelectorAll<HTMLElement>(".card").forEach((el) => {
    const on = Number(el.dataset.index) === selected;
    el.classList.toggle("selected", on);
    el.setAttribute("aria-selected", String(on));
    if (on && scroll) el.scrollIntoView({ block: "nearest", behavior: motionReduced() ? "auto" : "smooth" });
  });
}

function columns(): number {
  const first = grid.querySelector<HTMLElement>(".card");
  if (!first) return 1;
  const cols = getComputedStyle(grid).gridTemplateColumns.split(" ").length;
  return Math.max(1, cols);
}

// ---------------------------------------------------------------- actions

function toast(message: string): void {
  toastEl.textContent = message;
  toastEl.classList.add("show");
  clearTimeout((toast as unknown as { t?: number }).t);
  (toast as unknown as { t?: number }).t = window.setTimeout(() => toastEl.classList.remove("show"), 1600);
}

function close(): void {
  if (closing) return;
  closing = true;
  panel.classList.add("closing");
  window.setTimeout(() => void api.close(), motionReduced() ? 0 : 120);
}

async function choose(index: number, plain = false): Promise<void> {
  const item = view[index];
  if (!item || closing) return;
  closing = true;
  await api.paste(item.id, plain);
  if (!inTauri) toast(status.canPaste ? "Pasted (preview)" : "Copied (preview)");
}

async function copyOnly(index: number): Promise<void> {
  const item = view[index];
  if (!item) return;
  closing = true;
  await api.copy(item.id);
  if (!inTauri) toast("Copied (preview)");
}

async function togglePin(index: number): Promise<void> {
  const item = view[index];
  if (!item) return;
  await api.setPinned(item.id, !item.pinned);
  toast(item.pinned ? "Unpinned" : "Pinned — it will never be evicted");
  if (!inTauri) await loadItems();
}

async function toggleQueue(index: number): Promise<void> {
  const item = view[index];
  if (!item) return;
  await api.toggleQueued(item.id);
  await loadStatus();
  const queued = status.queue.includes(item.id);
  toast(queued ? `Queued — ${settings.hotkeyDashboard} pastes queued items in order` : "Removed from queue");
  render(item.id);
}

async function remove(index: number): Promise<void> {
  const item = view[index];
  if (!item) return;
  const el = grid.querySelector<HTMLElement>(`.card[data-index="${index}"]`);
  el?.classList.add("removing");
  await new Promise((r) => setTimeout(r, motionReduced() ? 0 : 140));
  await api.remove(item.id);
  thumbs.delete(item.id);
  if (!inTauri) await loadItems();
}

function setFilter(next: Filter): void {
  filter = next;
  selected = 0;
  render();
  grid.scrollTop = 0;
}

function setQuery(next: string): void {
  query = next;
  if (search.value !== next) search.value = next;
  searchMode = next.length > 0 || searchMode;
  selected = 0;
  render();
  grid.scrollTop = 0;
}

function exitSearch(): void {
  searchMode = false;
  setQuery("");
}

// ---------------------------------------------------------------- input

function onKeyDown(e: KeyboardEvent): void {
  if (settingsOpen()) {
    if (e.key === "Escape") {
      e.preventDefault();
      closeSettings();
      search.focus();
    }
    return;
  }
  const mod = e.ctrlKey || e.altKey || e.metaKey;
  const cols = columns();

  switch (e.key) {
    case "Escape":
      e.preventDefault();
      if (searchMode || query) exitSearch();
      else close();
      return;
    case "ArrowDown":
      e.preventDefault();
      updateSelection(selected + cols);
      return;
    case "ArrowUp":
      e.preventDefault();
      updateSelection(selected - cols);
      return;
    case "ArrowRight":
    case "ArrowLeft":
      if (searchMode && query) return; // move the caret instead
      e.preventDefault();
      updateSelection(selected + (e.key === "ArrowRight" ? 1 : -1));
      return;
    case "Home":
    case "End":
      if (searchMode && query) return;
      e.preventDefault();
      updateSelection(e.key === "Home" ? 0 : view.length - 1);
      return;
    case "PageDown":
    case "PageUp":
      e.preventDefault();
      updateSelection(selected + (e.key === "PageDown" ? 1 : -1) * cols * 3);
      return;
    case "Enter":
      e.preventDefault();
      void choose(selected, e.shiftKey);
      return;
    case "Tab": {
      e.preventDefault();
      const i = FILTERS.findIndex((f) => f.id === filter);
      setFilter(FILTERS[(i + (e.shiftKey ? FILTERS.length - 1 : 1)) % FILTERS.length].id);
      return;
    }
  }

  // Ctrl+C copies the selected item unless text is selected in the search box.
  if (e.ctrlKey && e.key.toLowerCase() === "c" && search.selectionStart === search.selectionEnd) {
    e.preventDefault();
    void copyOnly(selected);
    return;
  }

  if (searchMode) {
    // Typing goes to the search box; shortcuts need a modifier.
    if (e.altKey && /^[1-9]$/.test(e.key)) {
      e.preventDefault();
      void choose(Number(e.key) - 1);
    } else if (e.ctrlKey && e.key.toLowerCase() === "p") {
      e.preventDefault();
      void togglePin(selected);
    } else if (e.ctrlKey && e.key.toLowerCase() === "q") {
      e.preventDefault();
      void toggleQueue(selected);
    } else if (e.shiftKey && e.key === "Delete") {
      e.preventDefault();
      void remove(selected);
    }
    return;
  }

  if (mod) return;
  if (/^[1-9]$/.test(e.key)) {
    e.preventDefault();
    void choose(Number(e.key) - 1);
  } else if (e.key.toLowerCase() === "p") {
    e.preventDefault();
    void togglePin(selected);
  } else if (e.key.toLowerCase() === "q") {
    e.preventDefault();
    void toggleQueue(selected);
  } else if (e.key === "Delete") {
    e.preventDefault();
    void remove(selected);
  } else if (e.key === "/") {
    e.preventDefault();
    searchMode = true;
    renderChrome();
  } else if (e.key.length === 1 && e.key !== " ") {
    // Any other printable key starts a search and is typed normally.
    searchMode = true;
  }
}

let lastPointer = { x: -1, y: -1 };

function wireEvents(): void {
  document.addEventListener("keydown", onKeyDown);
  search.addEventListener("input", () => setQuery(search.value));
  // Keep typing focus in the search box whatever is clicked.
  document.addEventListener("mousedown", (e) => {
    const t = e.target as HTMLElement;
    if (!t.closest("input, textarea, #settings")) e.preventDefault();
  });

  grid.addEventListener("mousemove", (e) => {
    if (e.clientX === lastPointer.x && e.clientY === lastPointer.y) return;
    lastPointer = { x: e.clientX, y: e.clientY };
    const el = (e.target as HTMLElement).closest<HTMLElement>(".card");
    if (!el) return;
    const index = Number(el.dataset.index);
    if (index !== selected) updateSelection(index, false);
    if (!motionReduced()) {
      const r = el.getBoundingClientRect();
      const px = (e.clientX - r.left) / r.width - 0.5;
      const py = (e.clientY - r.top) / r.height - 0.5;
      el.style.setProperty("--ry", `${(px * 5).toFixed(2)}deg`);
      el.style.setProperty("--rx", `${(-py * 5).toFixed(2)}deg`);
    }
  });
  grid.addEventListener(
    "mouseleave",
    () => grid.querySelectorAll<HTMLElement>(".card").forEach((c) => c.style.removeProperty("--rx")),
  );
  grid.addEventListener("mouseout", (e) => {
    const el = (e.target as HTMLElement).closest<HTMLElement>(".card");
    if (el && !el.contains(e.relatedTarget as Node)) {
      el.style.removeProperty("--rx");
      el.style.removeProperty("--ry");
    }
  });
  grid.addEventListener("click", (e) => {
    const t = e.target as HTMLElement;
    const el = t.closest<HTMLElement>(".card");
    if (!el) return;
    const index = Number(el.dataset.index);
    const act = t.closest<HTMLElement>("[data-act]")?.dataset.act;
    if (act === "pin") void togglePin(index);
    else if (act === "queue") void toggleQueue(index);
    else if (act === "copy") void copyOnly(index);
    else if (act === "delete") void remove(index);
    else void choose(index, e.shiftKey);
  });

  filtersNav.addEventListener("click", (e) => {
    const b = (e.target as HTMLElement).closest<HTMLButtonElement>("button");
    if (b?.dataset.filter) setFilter(b.dataset.filter as Filter);
  });
  $("queue-toggle").addEventListener("click", async () => {
    await api.setQueueActive(!status.queueActive);
    await loadStatus();
    toast(status.queueActive ? `Paste queue on — new copies are queued` : "Paste queue off");
    render();
  });
  $("pause-toggle").addEventListener("click", async () => {
    await api.setPaused(!status.paused);
    await loadStatus();
    toast(status.paused ? "Capture paused" : "Capture resumed");
  });
  $("settings-btn").addEventListener("click", () => openSettings(settings, status, onSettingsSaved));
  banner.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).closest("[data-dismiss]")) {
      bannerDismissed = true;
      renderBanner();
    }
  });

  media.dark.addEventListener("change", () => applyAppearance(settings));
  media.reduce.addEventListener("change", () => applyAppearance(settings));

  void on("history-changed", () => void loadItems());
  void on<Status>("status-changed", (s) => {
    status = s;
    render(view[selected]?.id);
  });
  void on<Settings>("settings-changed", (s) => {
    settings = s;
    applyAppearance(s);
    renderChrome();
  });
  // Warm-window lifecycle (only when "keep dashboard ready" is on).
  void on<string>("dashboard-shown", (m) => void reopen(m));
  void on("dashboard-hidden", () => {
    panel.classList.remove("closing");
    closeSettings();
  });
}

function onSettingsSaved(next: Settings): void {
  settings = next;
  applyAppearance(next);
  renderChrome();
}

function playEnter(): void {
  panel.classList.remove("closing", "entered");
  void panel.offsetWidth;
  panel.classList.add("entered");
}

async function reopen(nextMode: string): Promise<void> {
  mode = nextMode;
  closing = false;
  filter = "all";
  searchMode = false;
  query = "";
  search.value = "";
  selected = 0;
  [settings, status] = await Promise.all([api.getSettings(), api.getStatus()]);
  applyAppearance(settings);
  await loadItems();
  grid.scrollTop = 0;
  playEnter();
  search.focus();
  if (mode === "settings") openSettings(settings, status, onSettingsSaved);
}

async function init(): Promise<void> {
  hydrateIcons();
  filtersNav.innerHTML = FILTERS.map(
    (f) => `<button role="tab" data-filter="${f.id}">${f.label}</button>`,
  ).join("");
  wireEvents();
  [settings, status] = await Promise.all([api.getSettings(), api.getStatus()]);
  applyAppearance(settings);
  items = await api.listItems();
  render();
  // Show the window only once the first frame has content.
  requestAnimationFrame(() => {
    playEnter();
    search.focus();
    void api.ready();
  });
  if (mode === "settings") openSettings(settings, status, onSettingsSaved);
}

void init();
