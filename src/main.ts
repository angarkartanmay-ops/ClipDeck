// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

import "./styles.css";
import { api, inTauri, on, type Item, type Settings, type Status } from "./api";
import { bytes, escapeHtml } from "./format";
import { fuzzyScore } from "./fuzzy";
import { deckArt, hydrateIcons, icons, mark } from "./icons";
import { comboText, isMac, primary, primaryCombo, setPlatform } from "./keys";
import { layout, type Box } from "./layout";
import { closeSettings, openSettings, settingsOpen } from "./settings-view";

type Filter = "all" | "text" | "image" | "files" | "pinned";
type Flavor = "text" | "rich" | "code" | "link" | "color" | "image" | "files";
type Display = Settings["displayMode"];

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "text", label: "Text" },
  { id: "image", label: "Images" },
  { id: "files", label: "Files" },
  { id: "pinned", label: "Pinned" },
];

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const root = document.documentElement;
const app = $("app");
const bar = $("bar");
const stage = $("stage");
const field = $("field");
const search = $<HTMLInputElement>("search");
const searchBox = search.closest<HTMLElement>(".search")!;
const filtersNav = $("filters");
const filterIndicator = filtersNav.querySelector<HTMLElement>(".filter-indicator")!;
const banner = $("banner");
const hints = $("hints");
const statusEl = $("status");
const countEl = $("count");
const quicklook = $("quicklook");
const toastEl = $("toast");

let items: Item[] = [];
let view: Item[] = [];
let boxes: Box[] = [];
let settings: Settings;
let status: Status;
let filter: Filter = "all";
let query = "";
/** In search mode every printable key types; otherwise letters are shortcuts. */
let searchMode = false;
let selected = 0;
let closing = false;
let loadError: string | null = null;
let bannerDismissed = false;
let mode = new URLSearchParams(location.search).get("mode") ?? "";
let lastPointer = { x: -1, y: -1 };
let qlIndex: number | null = null;
const thumbs = new Map<number, string>();
const removing = new Set<number>();

if (!inTauri) root.classList.add("mock");

// ---------------------------------------------------------------- environment

const media = { reduce: matchMedia("(prefers-reduced-motion: reduce)") };

function applyMotion(s: Settings): void {
  const reduce = s.reduceMotion === "on" || (s.reduceMotion === "system" && media.reduce.matches);
  root.dataset.motion = reduce ? "reduce" : "full";
}

const motionReduced = () => root.dataset.motion === "reduce";
const display = (): Display => settings?.displayMode ?? "fullscreen";
const wait = (ms: number) => new Promise((r) => setTimeout(r, motionReduced() ? 0 : ms));
const frame = () => new Promise((r) => requestAnimationFrame(() => r(null)));
const css = (name: string) => getComputedStyle(root).getPropertyValue(name).trim();

// ---------------------------------------------------------------- item helpers

const COLOR_RE = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i;
const LINK_RE = /^https?:\/\/\S+$/i;

function flavor(item: Item): Flavor {
  if (item.kind === "image" || item.kind === "files") return item.kind;
  const t = item.text.trim();
  if (COLOR_RE.test(t)) return "color";
  if (LINK_RE.test(t)) return "link";
  if (/[{};]\s*$|=>|^\s*(?:import|export|const|let|fn|def|class|function|SELECT|public)\b/m.test(t)) return "code";
  return item.kind === "rich" ? "rich" : "text";
}

const isShortText = (item: Item) => item.text.trim().length <= 48 && !item.text.trim().includes("\n");

/** Width / height of a tile's card, so the grid reads like real windows. */
function ratio(item: Item, f: Flavor): number {
  switch (f) {
    case "image": {
      const { width = 16, height = 10 } = item.meta;
      return Math.min(2.2, Math.max(0.62, width / Math.max(1, height)));
    }
    case "code":
      return 1.55;
    case "link":
      return 1.75;
    case "color":
      return 1.15;
    case "files":
      return 1.3;
    default:
      return isShortText(item) ? 1.6 : 1.3;
  }
}

function titleOf(item: Item, f: Flavor): string {
  switch (f) {
    case "image":
      return item.meta.width ? `Image ${item.meta.width} × ${item.meta.height}` : "Image";
    case "files": {
      const names = item.meta.files ?? [];
      const n = item.meta.fileCount ?? names.length;
      return n === 1 ? (names[0] ?? "1 file") : `${n} files`;
    }
    case "link":
      return item.text.trim().replace(/^https?:\/\/(www\.)?/i, "");
    case "color":
      return item.text.trim().toUpperCase();
    default: {
      const line = item.text.split(/\r?\n/).find((l) => l.trim()) ?? "";
      return line.trim().replace(/\s+/g, " ").slice(0, 120) || "Empty text";
    }
  }
}

/** "Today, 7:58 PM", "Yesterday, 9:12 AM" or "Oct 21, 3:40 PM". */
function when(ms: number): string {
  const d = new Date(ms);
  const time = d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  const day = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((day(new Date()) - day(d)) / 86_400_000);
  if (diff === 0) return `Today, ${time}`;
  if (diff === 1) return `Yesterday, ${time}`;
  return `${d.toLocaleDateString(undefined, { month: "short", day: "numeric" })}, ${time}`;
}

/** Escapes `text` and wraps the parts that match the search tokens. */
function highlightMatches(text: string): string {
  const tokens = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!tokens.length) return escapeHtml(text);
  const lower = text.toLowerCase();
  const marks: [number, number][] = [];
  for (const t of tokens) {
    const i = lower.indexOf(t);
    if (i >= 0) marks.push([i, i + t.length]);
  }
  marks.sort((a, b) => a[0] - b[0]);
  let out = "";
  let pos = 0;
  for (const [s, e] of marks) {
    if (s < pos) continue;
    out += escapeHtml(text.slice(pos, s)) + `<b>${escapeHtml(text.slice(s, e))}</b>`;
    pos = e;
  }
  return out + escapeHtml(text.slice(pos));
}

function hexInfo(text: string): { full: string; rgb: string } {
  const hex = text.trim();
  const full = hex.length === 4 ? `#${[...hex.slice(1)].map((c) => c + c).join("")}` : hex;
  const n = parseInt(full.slice(1), 16);
  return { full: full.toUpperCase(), rgb: `${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}` };
}

function splitUrl(text: string): { host: string; rest: string } {
  try {
    const u = new URL(text.trim());
    return { host: u.host.replace(/^www\./, ""), rest: `${u.pathname}${u.search}${u.hash}`.replace(/^\/$/, "") };
  } catch {
    return { host: text.trim(), rest: "" };
  }
}

// ---------------------------------------------------------------- data

async function loadItems(animate: "bubble" | "flip" | "none" = "flip"): Promise<void> {
  const keepId = view[selected]?.id;
  try {
    items = await api.listItems();
    loadError = null;
  } catch (err) {
    loadError = String(err);
  }
  const live = new Set(items.map((i) => i.id));
  for (const id of thumbs.keys()) if (!live.has(id)) thumbs.delete(id);
  render(animate, keepId);
}

function computeView(): Item[] {
  let out = items;
  if (filter === "pinned") out = out.filter((i) => i.pinned);
  else if (filter === "text") out = out.filter((i) => i.kind === "text" || i.kind === "rich");
  else if (filter !== "all") out = out.filter((i) => i.kind === filter);
  const q = query.trim();
  if (!q) return out;
  return out
    .map((item) => ({ item, score: fuzzyScore(q, `${item.text}\n${item.sourceApp}\n${(item.meta.files ?? []).join(" ")}`) }))
    .filter((r): r is { item: Item; score: number } => r.score !== null)
    .sort((a, b) => b.score - a.score)
    .map((r) => r.item);
}

async function thumb(id: number): Promise<string> {
  let src = thumbs.get(id);
  if (!src) {
    src = (await api.getThumb(id)) ?? "";
    if (src) thumbs.set(id, src);
  }
  return src;
}

/** Quick Look shows the full-resolution image, falling back to the thumbnail. */
function loadPreview(scope: ParentNode): void {
  const img = scope.querySelector<HTMLImageElement>("img[data-preview]");
  if (!img) return;
  const id = Number(img.dataset.preview);
  const cached = thumbs.get(id);
  if (cached) img.src = cached;
  img.onload = () => img.classList.add("loaded");
  void api.getPreview(id).then((src) => {
    if (src && img.isConnected) img.src = src;
  });
}

function loadImages(scope: ParentNode): void {
  scope.querySelectorAll<HTMLImageElement>("img[data-thumb]:not(.loaded)").forEach(async (img) => {
    const src = await thumb(Number(img.dataset.thumb));
    if (!src) return;
    img.onload = () => img.classList.add("loaded");
    img.src = src;
  });
}

// ---------------------------------------------------------------- tiles

function cardContent(item: Item, f: Flavor): string {
  switch (f) {
    case "image": {
      const cached = thumbs.get(item.id);
      return `<div class="content c-image"><img data-thumb="${item.id}" ${cached ? `src="${cached}" class="loaded"` : ""} alt="" draggable="false"/><div class="shimmer"></div></div>`;
    }
    case "files": {
      const names = item.meta.files ?? [];
      const n = item.meta.fileCount ?? names.length;
      if (n === 1) {
        const name = names[0] ?? "";
        const dir = !/\.[a-z0-9]{1,6}$/i.test(name);
        return `<div class="content c-files single"><span class="big-icon">${dir ? icons.folder : icons.file}</span><span class="fname">${escapeHtml(name)}</span></div>`;
      }
      const rows = names
        .slice(0, 4)
        .map((nm) => `<div class="file">${icons.file}<span>${escapeHtml(nm)}</span></div>`)
        .join("");
      return `<div class="content c-files">${rows}${n > 4 ? `<div class="more">+${n - 4} more</div>` : ""}</div>`;
    }
    case "color": {
      const { full } = hexInfo(item.text);
      return `<div class="content c-color" style="background:${full}"><span class="hex">${escapeHtml(full)}</span></div>`;
    }
    case "link": {
      const { host, rest } = splitUrl(item.text);
      return `<div class="content c-link"><span class="glyph">${icons.link}</span><span class="host">${escapeHtml(host)}</span>${
        rest ? `<span class="path">${escapeHtml(rest)}</span>` : ""
      }</div>`;
    }
    case "code":
      return `<div class="content c-code">${escapeHtml(item.text.slice(0, 900))}</div>`;
    default:
      return `<div class="content c-text${isShortText(item) ? " big" : ""}">${escapeHtml(item.text.slice(0, 700))}</div>`;
  }
}

function tile(item: Item, index: number, box: Box, labelH: number): string {
  const f = flavor(item);
  const queuePos = status?.queue.indexOf(item.id) ?? -1;
  const badges = [
    queuePos >= 0 ? `<span class="badge solid" title="Queued">Q${queuePos + 1}</span>` : "",
    item.pinned ? `<span class="badge" title="Pinned">${icons.pinned}</span>` : "",
  ].join("");
  const slotKey = !searchMode && index < 9 ? `<span class="slot-key"><kbd class="key">${index + 1}</kbd></span>` : "";
  return `<div class="slot" role="option" id="tile-${item.id}" data-index="${index}" data-id="${item.id}" aria-selected="false"
      aria-label="${escapeHtml(titleOf(item, f))}, from ${escapeHtml(item.sourceApp || "unknown app")}"
      style="transform:translate(${box.x}px,${box.y}px);width:${box.w}px;height:${box.h + labelH}px">
    <div class="tile">
      <div class="card" style="height:${box.h}px">${cardContent(item, f)}${slotKey}<div class="badges">${badges}</div></div>
      <div class="label">${icons[f]}<span class="name">${highlightMatches(titleOf(item, f))}</span></div>
    </div>
  </div>`;
}

function emptyState(): string {
  if (loadError) {
    return `<div class="empty">${icons.alert}<h2>Couldn't read your history</h2><p>${escapeHtml(loadError)}</p><button class="btn" data-retry>Try again</button></div>`;
  }
  if (items.length === 0) {
    return `<div class="empty">${deckArt}<h2>Nothing copied yet</h2>
      <p>Copy text, a screenshot or some files and they appear here. Bring this view back with <kbd class="key">${escapeHtml(comboText(settings.hotkeyDashboard))}</kbd>.</p></div>`;
  }
  if (query) {
    return `<div class="empty">${icons.search}<h2>No matches for “${escapeHtml(query)}”</h2><p>Try fewer letters, or press <kbd class="key">Tab</kbd> to switch filters.</p></div>`;
  }
  const label = FILTERS.find((x) => x.id === filter)?.label.toLowerCase();
  return `<div class="empty">${icons[filter === "pinned" ? "pin" : "history"]}<h2>No ${label} yet</h2><p>${
    filter === "pinned" ? 'Press <kbd class="key">P</kbd> on any clip to keep it forever.' : "Copied items of this type will show up here."
  }</p></div>`;
}

interface Geometry {
  pad: number;
  top: number;
  labelH: number;
}

function geometry(): Geometry {
  const full = display() === "fullscreen";
  return {
    pad: full ? Math.round(Math.min(110, Math.max(28, stage.clientWidth * 0.055))) : 16,
    top: full ? 24 : 12,
    labelH: 32,
  };
}

/** Lays the view out and (re)builds the tiles. */
function render(animate: "bubble" | "flip" | "none" = "none", keepId?: number): void {
  const before = animate === "flip" ? positions() : null;
  view = computeView();
  if (keepId !== undefined) {
    const idx = view.findIndex((i) => i.id === keepId);
    selected = idx >= 0 ? idx : Math.min(selected, view.length - 1);
  }
  selected = Math.max(0, Math.min(selected, view.length - 1));

  const g = geometry();
  const full = display() === "fullscreen";
  const width = Math.max(200, stage.clientWidth - g.pad * 2);
  const viewHeight = Math.max(160, stage.clientHeight - g.top * 2);
  const result = layout(
    view.map((it) => ratio(it, flavor(it))),
    {
      width,
      viewHeight,
      minRowHeight: full ? 120 : 92,
      maxRowHeight: full ? Math.min(320, viewHeight * 0.44) : 210,
      gapX: full ? 30 : 16,
      gapY: full ? 22 : 14,
      labelHeight: g.labelH,
    },
  );
  boxes = result.boxes;
  // Centre short layouts vertically (slightly above the middle).
  const offsetY = result.height < viewHeight ? Math.round((viewHeight - result.height) * 0.45) : 0;
  for (const b of boxes) b.y += offsetY + g.top;
  field.style.width = `${width}px`;
  field.style.height = `${Math.max(result.height + g.top * 2 + offsetY, stage.clientHeight - 1)}px`;

  field.innerHTML = view.length ? view.map((it, i) => tile(it, i, boxes[i], g.labelH)).join("") : emptyState();
  loadImages(field);
  if (animate === "bubble") bubbleIn();
  if (before) flip(before);
  updateSelection(selected, { scroll: animate !== "none" });
  renderChrome();
}

function positions(): Map<string, { x: number; y: number }> {
  const map = new Map<string, { x: number; y: number }>();
  field.querySelectorAll<HTMLElement>(".slot").forEach((el) => {
    const b = boxes[Number(el.dataset.index)];
    if (b) map.set(el.dataset.id!, { x: b.x, y: b.y });
  });
  return map;
}

/** Tiles bubble up from small, rippling out from the centre of the screen. */
function bubbleIn(): void {
  if (motionReduced() || !boxes.length) return;
  const spring = css("--spring");
  const cx = field.clientWidth / 2;
  const cy = stage.scrollTop + stage.clientHeight / 2;
  const far = Math.hypot(cx, stage.clientHeight / 2) || 1;
  field.querySelectorAll<HTMLElement>(".slot").forEach((el) => {
    const b = boxes[Number(el.dataset.index)];
    const d = Math.hypot(b.x + b.w / 2 - cx, b.y + b.h / 2 - cy);
    el.querySelector(".tile")!.animate(
      [
        { opacity: 0, transform: "scale(0.45)" },
        { opacity: 1, offset: 0.3 },
        { opacity: 1, transform: "scale(1)" },
      ],
      { duration: 680, delay: 30 + Math.min(1, d / far) * 230, easing: spring, fill: "backwards" },
    );
  });
}

/** FLIP: tiles that moved glide from where they were; new ones pop in. */
function flip(before: Map<string, { x: number; y: number }>): void {
  if (motionReduced()) return;
  const soft = css("--spring-soft");
  field.querySelectorAll<HTMLElement>(".slot").forEach((el) => {
    const b = boxes[Number(el.dataset.index)];
    const old = before.get(el.dataset.id!);
    if (!old) {
      el.querySelector(".tile")!.animate([{ opacity: 0, transform: "scale(0.6)" }, { opacity: 1, transform: "scale(1)" }], {
        duration: 520,
        easing: css("--spring"),
      });
      return;
    }
    if (old.x !== b.x || old.y !== b.y) {
      el.animate([{ transform: `translate(${old.x}px,${old.y}px)` }, { transform: `translate(${b.x}px,${b.y}px)` }], {
        duration: 560,
        easing: soft,
      });
    }
  });
}

function slotEl(index: number): HTMLElement | null {
  return field.querySelector<HTMLElement>(`.slot[data-index="${index}"]`);
}

function updateSelection(next: number, opts: { scroll?: boolean } = {}): void {
  if (!view.length) return;
  selected = Math.max(0, Math.min(next, view.length - 1));
  field.querySelectorAll<HTMLElement>(".slot.selected").forEach((el) => {
    el.classList.remove("selected");
    el.setAttribute("aria-selected", "false");
  });
  const el = slotEl(selected);
  if (!el) return;
  el.classList.add("selected");
  el.setAttribute("aria-selected", "true");
  field.setAttribute("aria-activedescendant", el.id);
  if (opts.scroll) {
    const b = boxes[selected];
    const top = b.y - 16;
    const bottom = b.y + b.h + geometry().labelH + 16;
    const behavior = motionReduced() ? "auto" : "smooth";
    if (top < stage.scrollTop) stage.scrollTo({ top, behavior });
    else if (bottom > stage.scrollTop + stage.clientHeight) stage.scrollTo({ top: bottom - stage.clientHeight, behavior });
  }
}

/** Up/Down: the tile in the neighbouring row closest horizontally. */
function moveVertical(dir: 1 | -1): void {
  const cur = boxes[selected];
  if (!cur) return;
  const cx = cur.x + cur.w / 2;
  let best = -1;
  let bestDist = Infinity;
  boxes.forEach((b, i) => {
    if (b.row !== cur.row + dir) return;
    const d = Math.abs(b.x + b.w / 2 - cx);
    if (d < bestDist) {
      bestDist = d;
      best = i;
    }
  });
  if (best >= 0) updateSelection(best, { scroll: true });
}

// ---------------------------------------------------------------- chrome

function moveFilterIndicator(instant = false): void {
  const b = filtersNav.querySelector<HTMLElement>(`button[data-filter="${filter}"]`);
  if (!b) return;
  if (instant) filterIndicator.style.transition = "none";
  filterIndicator.style.width = `${b.offsetWidth}px`;
  filterIndicator.style.transform = `translateX(${b.offsetLeft}px)`;
  if (instant) requestAnimationFrame(() => filterIndicator.style.removeProperty("transition"));
}

function renderChrome(): void {
  if (!settings || !status) return;
  root.dataset.display = display();
  filtersNav.querySelectorAll<HTMLButtonElement>("button").forEach((b) => {
    const active = b.dataset.filter === filter;
    b.classList.toggle("active", active);
    b.setAttribute("aria-selected", String(active));
  });
  moveFilterIndicator();
  searchBox.classList.toggle("typing", searchMode || query.length > 0);
  countEl.textContent = `${status.itemCount}`;

  const queueBtn = $("queue-toggle");
  queueBtn.classList.toggle("on", status.queueActive);
  queueBtn.title = status.queueActive
    ? `Paste queue on: ${comboText(settings.hotkeyDashboard)} pastes the next item. Click to stop.`
    : `Paste queue (${comboText(settings.hotkeyQueue)})`;
  $("queue-count").textContent = status.queueActive && status.queue.length ? String(status.queue.length) : "";

  const pauseBtn = $("pause-toggle");
  pauseBtn.classList.toggle("on", status.paused);
  pauseBtn.innerHTML = status.paused ? icons.play : icons.pause;
  pauseBtn.title = status.paused ? "Resume capture" : "Pause capture";
  $("settings-btn").title = "Settings";
  const modeBtn = $("mode-toggle");
  modeBtn.innerHTML = display() === "fullscreen" ? icons.windowed : icons.maximize;
  modeBtn.title = display() === "fullscreen" ? "Window mode: move and resize" : "Full screen";
  $("close-btn").title = "Close (Esc)";

  statusEl.classList.toggle("paused", status.paused);
  statusEl.innerHTML = `<span class="dot"></span><span>${status.paused ? "Paused" : "Capturing"}</span><span class="num">${bytes(
    status.diskBytes,
  )}</span>`;

  const verb = status.canPaste ? "paste" : "copy";
  const hint = (key: string, what: string) => `<span><kbd class="key">${escapeHtml(key)}</kbd>${what}</span>`;
  hints.innerHTML = (
    searchMode && query
      ? [hint("↵", verb), hint(isMac() ? "⌥1–9" : "Alt 1–9", "quick"), hint(primaryCombo("P"), "pin"), hint("Esc", "clear")]
      : [
          hint("↵", verb),
          hint("Space", "preview"),
          hint("1–9", "quick"),
          hint("P", "pin"),
          hint("Q", "queue"),
          hint(isMac() ? "⌘⌫" : "Del", "remove"),
          hint("Esc", "close"),
        ]
  ).join("");

  renderBanner();
}

/** Notices dismissed while this dashboard is open. */
const dismissedNotices = new Set<string>();

function renderBanner(): void {
  let html = "";
  const notice = status.notices.find((n) => !dismissedNotices.has(n.id));
  if (status.hotkeyErrors.length) {
    html = `${icons.alert}<div><b>Hotkey conflict.</b> ${status.hotkeyErrors.map(escapeHtml).join("; ")}.</div><button class="btn" data-open-settings>Fix</button>`;
  } else if (notice) {
    const action = notice.action
      ? `<button class="btn" data-notice-action="${escapeHtml(notice.action)}">${escapeHtml(notice.actionLabel ?? "Open")}</button>`
      : "";
    html = `${icons[notice.id === "policy" ? "shield" : "info"]}<div>${escapeHtml(notice.text)}</div>${action}<button class="tool" type="button" data-dismiss-notice="${escapeHtml(
      notice.id,
    )}" aria-label="Dismiss">${icons.close}</button>`;
  } else if (mode === "welcome" && !bannerDismissed) {
    html = `${icons.keyboard}<div><b>ClipDeck runs in your ${isMac() ? "menu bar" : "tray"}.</b> Copy as usual, then press <kbd class="key">${escapeHtml(
      comboText(settings.hotkeyDashboard),
    )}</kbd> to pick what to paste. Plain <kbd class="key">${primaryCombo("V")}</kbd> is unchanged.</div><button class="btn" data-dismiss>Got it</button>`;
  }
  if (banner.innerHTML !== html) banner.innerHTML = html;
  banner.hidden = !html;
}

// ---------------------------------------------------------------- actions

let toastTimer = 0;

function toast(message: string, icon = "paste"): void {
  toastEl.innerHTML = `${icons[icon] ?? ""}<span>${escapeHtml(message)}</span>`;
  toastEl.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => toastEl.classList.remove("show"), 1700);
}

function leave(): void {
  app.classList.add("leaving");
  field.classList.add("leaving");
}

function close(): void {
  if (closing) return;
  closing = true;
  leave();
  window.setTimeout(() => {
    void api.close();
    if (!inTauri) resetAfterMockClose();
  }, motionReduced() ? 0 : 170);
}

async function choose(index: number, plain = false): Promise<void> {
  const item = view[index];
  if (!item || closing) return;
  closing = true;
  updateSelection(index);
  slotEl(index)?.classList.add("chosen");
  leave();
  await wait(150);
  await api.paste(item.id, plain);
  if (!inTauri) {
    toast(status.canPaste ? "Pasted into your app" : "Copied to clipboard");
    resetAfterMockClose();
  }
}

async function copyOnly(index: number): Promise<void> {
  const item = view[index];
  if (!item || closing) return;
  closing = true;
  slotEl(index)?.classList.add("chosen");
  leave();
  await wait(150);
  await api.copy(item.id);
  if (!inTauri) {
    toast("Copied to clipboard", "copy");
    resetAfterMockClose();
  }
}

/** The browser preview has no window to close; bring the deck back. */
function resetAfterMockClose(): void {
  window.setTimeout(() => {
    closing = false;
    field.classList.remove("leaving");
    playEnter();
    render("bubble");
  }, 900);
}

async function togglePin(index: number): Promise<void> {
  const item = view[index];
  if (!item) return;
  await api.setPinned(item.id, !item.pinned);
  toast(item.pinned ? "Unpinned" : "Pinned, never evicted", "pin");
  if (!inTauri) await loadItems("flip");
}

async function toggleQueue(index: number): Promise<void> {
  const item = view[index];
  if (!item) return;
  await api.toggleQueued(item.id);
  status = await api.getStatus();
  const n = status.queue.indexOf(item.id);
  toast(n >= 0 ? `Queued #${n + 1}: ${comboText(settings.hotkeyDashboard)} pastes in order` : "Removed from queue", "queue");
  render("none", item.id);
}

async function remove(index: number): Promise<void> {
  const item = view[index];
  if (!item || removing.has(item.id)) return;
  removing.add(item.id);
  slotEl(index)?.classList.add("removing");
  await wait(170);
  try {
    await api.remove(item.id);
    thumbs.delete(item.id);
    if (!inTauri) await loadItems("flip");
  } finally {
    removing.delete(item.id);
  }
}

function setFilter(next: Filter): void {
  if (next === filter) return;
  filter = next;
  selected = 0;
  stage.scrollTop = 0;
  render("bubble");
}

function setQuery(next: string): void {
  query = next;
  if (search.value !== next) search.value = next;
  if (next) searchMode = true;
  selected = 0;
  stage.scrollTop = 0;
  render("flip");
}

function exitSearch(): void {
  searchMode = false;
  setQuery("");
}

/** Switches between the full-screen overlay and the movable window. */
async function setDisplay(next: Display): Promise<void> {
  if (next === display()) return;
  field.classList.add("leaving");
  await wait(150);
  settings = { ...(await api.saveSettings({ ...settings, displayMode: next })), displayMode: next };
  root.dataset.display = next;
  // The core resizes the window; let the new size land before laying out.
  await frame();
  await frame();
  await wait(40);
  field.classList.remove("leaving");
  render("bubble", view[selected]?.id);
  search.focus();
}

// ---------------------------------------------------------------- quick look

function qlBody(item: Item, f: Flavor): string {
  switch (f) {
    case "image":
      return `<div class="ql-image"><img data-preview="${item.id}" alt="Copied image" draggable="false"/><div class="shimmer"></div></div>`;
    case "files":
      return `<div class="ql-files">${item.text
        .split(/\r?\n/)
        .filter(Boolean)
        .map((p) => {
          const name = p.split(/[\\/]/).pop() ?? p;
          const dir = !/\.[a-z0-9]{1,6}$/i.test(name);
          return `<div class="file"><span class="ficon">${dir ? icons.folder : icons.file}</span><span><div class="name">${escapeHtml(
            name,
          )}</div><div class="path">${escapeHtml(p)}</div></span></div>`;
        })
        .join("")}</div>`;
    case "color": {
      const { full, rgb } = hexInfo(item.text);
      return `<div class="ql-color"><div class="big" style="background:${full}"></div><dl><dt>HEX</dt><dd>${escapeHtml(full)}</dd><dt>RGB</dt><dd>${rgb}</dd></dl></div>`;
    }
    case "link": {
      const { host, rest } = splitUrl(item.text);
      return `<div class="ql-link"><span class="host">${escapeHtml(host)}</span>${rest ? `<span class="path">${escapeHtml(rest)}</span>` : ""}</div>`;
    }
    case "code":
      return `<pre class="code">${escapeHtml(item.text)}</pre>`;
    default:
      return `<pre>${escapeHtml(item.text)}</pre>`;
  }
}

function qlPanel(item: Item): string {
  const f = flavor(item);
  const verb = status.canPaste ? "Paste" : "Copy";
  const extra =
    f === "image"
      ? bytes(item.size)
      : f === "files"
        ? `${item.meta.fileCount ?? 0} items`
        : `${(item.meta.chars ?? item.text.length).toLocaleString()} chars`;
  return `<section class="ql-panel" role="dialog" aria-label="Preview">
      <header class="ql-head">
        <span class="title">${escapeHtml(titleOf(item, f))}</span>
        <span class="meta">${escapeHtml(item.sourceApp || "Unknown app")} · ${when(item.createdAt)} · ${extra}</span>
        <button class="tool" type="button" data-ql-close aria-label="Close preview">${icons.close}</button>
      </header>
      <div class="ql-body">${qlBody(item, f)}</div>
      <footer class="ql-foot">
        <button class="btn primary" data-act="paste">${icons.paste}${verb}<kbd class="key">↵</kbd></button>
        ${item.kind === "rich" ? `<button class="btn" data-act="plain">Plain text<kbd class="key">⇧↵</kbd></button>` : ""}
        <span class="spacer"></span>
        <button class="btn" data-act="pin">${icons.pin}${item.pinned ? "Unpin" : "Pin"}</button>
        <button class="btn" data-act="copy">${icons.copy}Copy</button>
        <button class="btn" data-act="delete">${icons.trash}Delete</button>
      </footer>
    </section>`;
}

/** Zooms a preview out of the tile, like macOS Quick Look. */
function openQuickLook(index: number): void {
  const item = view[index];
  if (!item) return;
  qlIndex = index;
  quicklook.innerHTML = `<div class="ql-backdrop" data-ql-close></div>${qlPanel(item)}`;
  quicklook.hidden = false;
  quicklook.classList.remove("closing");
  app.classList.add("covered");
  loadPreview(quicklook);
  const panel = quicklook.querySelector<HTMLElement>(".ql-panel")!;
  const card = slotEl(index)?.querySelector<HTMLElement>(".card");
  if (card && !motionReduced()) {
    const from = card.getBoundingClientRect();
    const to = panel.getBoundingClientRect();
    panel.animate(
      [
        {
          transform: `translate(${from.left - to.left}px, ${from.top - to.top}px) scale(${from.width / to.width}, ${from.height / to.height})`,
          opacity: 0.2,
        },
        { opacity: 1, offset: 0.25 },
        { transform: "none", opacity: 1 },
      ],
      { duration: 520, easing: css("--spring-soft") },
    );
  }
}

async function closeQuickLook(): Promise<void> {
  if (qlIndex === null) return;
  const panel = quicklook.querySelector<HTMLElement>(".ql-panel");
  const card = slotEl(qlIndex)?.querySelector<HTMLElement>(".card");
  qlIndex = null;
  app.classList.remove("covered");
  quicklook.classList.add("closing");
  if (panel && card && !motionReduced()) {
    const from = panel.getBoundingClientRect();
    const to = card.getBoundingClientRect();
    await panel.animate(
      [
        { transform: "none", opacity: 1 },
        {
          transform: `translate(${to.left - from.left}px, ${to.top - from.top}px) scale(${to.width / from.width}, ${to.height / from.height})`,
          opacity: 0,
        },
      ],
      { duration: 200, easing: css("--ease-in"), fill: "forwards" },
    ).finished;
  }
  quicklook.hidden = true;
  quicklook.innerHTML = "";
  search.focus();
}

function stepQuickLook(delta: number): void {
  if (qlIndex === null) return;
  const next = Math.max(0, Math.min(view.length - 1, qlIndex + delta));
  if (next === qlIndex) return;
  qlIndex = next;
  updateSelection(next, { scroll: true });
  quicklook.querySelector(".ql-panel")!.outerHTML = qlPanel(view[next]);
  loadPreview(quicklook);
  quicklook
    .querySelector<HTMLElement>(".ql-body")
    ?.animate([{ opacity: 0, transform: `translateX(${delta * 14}px)` }, { opacity: 1, transform: "none" }], {
      duration: 260,
      easing: css("--ease"),
    });
}

// ---------------------------------------------------------------- keyboard

function onKeyDown(e: KeyboardEvent): void {
  if (settingsOpen()) {
    if (e.key === "Escape") {
      e.preventDefault();
      closeSettings();
      search.focus();
    }
    return;
  }

  if (qlIndex !== null) {
    if (e.key === "Escape" || e.key === " ") {
      e.preventDefault();
      void closeQuickLook();
    } else if (e.key === "Enter") {
      e.preventDefault();
      const i = qlIndex;
      void closeQuickLook().then(() => choose(i, e.shiftKey));
    } else if (e.key === "ArrowRight" || e.key === "ArrowDown") {
      e.preventDefault();
      stepQuickLook(1);
    } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
      e.preventDefault();
      stepQuickLook(-1);
    }
    return;
  }

  switch (e.key) {
    case "Escape":
      e.preventDefault();
      if (searchMode || query) exitSearch();
      else close();
      return;
    case "ArrowDown":
    case "ArrowUp":
      e.preventDefault();
      moveVertical(e.key === "ArrowDown" ? 1 : -1);
      return;
    case "ArrowRight":
    case "ArrowLeft":
      if (query) return; // caret movement in the search box
      e.preventDefault();
      updateSelection(selected + (e.key === "ArrowRight" ? 1 : -1), { scroll: true });
      return;
    case "Home":
    case "End":
      if (query) return;
      e.preventDefault();
      updateSelection(e.key === "Home" ? 0 : view.length - 1, { scroll: true });
      return;
    case "Enter":
      e.preventDefault();
      void choose(selected, e.shiftKey);
      return;
    case "Tab": {
      e.preventDefault();
      const i = FILTERS.findIndex((x) => x.id === filter);
      setFilter(FILTERS[(i + (e.shiftKey ? FILTERS.length - 1 : 1)) % FILTERS.length].id);
      return;
    }
  }

  // Ctrl+C (⌘C) copies the selected item unless text is selected in the search box.
  if (primary(e) && e.key.toLowerCase() === "c" && search.selectionStart === search.selectionEnd) {
    e.preventDefault();
    void copyOnly(selected);
    return;
  }
  // ⌘⌫ removes on macOS, where the Delete key is Backspace.
  if (isMac() && e.metaKey && e.key === "Backspace" && !query) {
    e.preventDefault();
    void remove(selected);
    return;
  }

  if (searchMode && query) {
    if (e.altKey && /^[1-9]$/.test(e.key)) {
      e.preventDefault();
      void choose(Number(e.key) - 1);
    } else if (primary(e) && e.key.toLowerCase() === "p") {
      e.preventDefault();
      void togglePin(selected);
    } else if (primary(e) && e.key.toLowerCase() === "q") {
      e.preventDefault();
      void toggleQueue(selected);
    } else if (e.shiftKey && e.key === "Delete") {
      e.preventDefault();
      void remove(selected);
    }
    return;
  }

  if (e.ctrlKey || e.altKey || e.metaKey) return;
  const k = e.key.toLowerCase();
  if (e.key === " " && !searchMode) {
    e.preventDefault();
    openQuickLook(selected);
  } else if (searchMode) {
    // "/" was pressed: everything types into the search box.
  } else if (/^[1-9]$/.test(k)) {
    e.preventDefault();
    void choose(Number(k) - 1);
  } else if (k === "p") {
    e.preventDefault();
    void togglePin(selected);
  } else if (k === "q") {
    e.preventDefault();
    void toggleQueue(selected);
  } else if (e.key === "Delete") {
    e.preventDefault();
    void remove(selected);
  } else if (e.key === "/") {
    e.preventDefault();
    searchMode = true;
    render("none");
  } else if (e.key.length === 1) {
    // Any other printable key starts a search and is typed normally.
    searchMode = true;
  }
}

// ---------------------------------------------------------------- wiring

function wireEvents(): void {
  document.addEventListener("keydown", onKeyDown);
  search.addEventListener("input", () => setQuery(search.value));
  // Keep typing focus in the search box whatever is clicked.
  document.addEventListener("mousedown", (e) => {
    const t = e.target as HTMLElement;
    if (!t.closest("input, textarea, .sheet, .ql-body")) e.preventDefault();
  });

  // Window mode: drag the bar to move, double-click it to go full screen.
  bar.addEventListener("mousedown", (e) => {
    if (display() !== "window" || e.button !== 0) return;
    if ((e.target as HTMLElement).closest("button, label, input, nav")) return;
    void api.startDragging();
  });
  bar.addEventListener("dblclick", (e) => {
    if (display() !== "window") return;
    if ((e.target as HTMLElement).closest("button, label, input, nav")) return;
    void setDisplay("fullscreen");
  });

  field.addEventListener("mousemove", (e) => {
    if (e.clientX === lastPointer.x && e.clientY === lastPointer.y) return;
    // The first event after opening is Chromium's synthetic one for a
    // cursor merely resting over the deck; only real movement selects.
    const first = lastPointer.x < 0;
    lastPointer = { x: e.clientX, y: e.clientY };
    if (first) return;
    const el = (e.target as HTMLElement).closest<HTMLElement>(".slot");
    if (el && Number(el.dataset.index) !== selected) updateSelection(Number(el.dataset.index));
  });
  stage.addEventListener("click", (e) => {
    const t = e.target as HTMLElement;
    if (t.closest("[data-retry]")) return void loadItems("bubble");
    const el = t.closest<HTMLElement>(".slot");
    if (el) return void choose(Number(el.dataset.index), e.shiftKey);
    // Clicking empty space dismisses the overlay, like Mission Control.
    if (display() === "fullscreen" && !t.closest(".empty")) close();
  });
  stage.addEventListener("contextmenu", (e) => {
    const el = (e.target as HTMLElement).closest<HTMLElement>(".slot");
    if (!el) return;
    e.preventDefault();
    updateSelection(Number(el.dataset.index));
    openQuickLook(Number(el.dataset.index));
  });

  quicklook.addEventListener("click", (e) => {
    const t = e.target as HTMLElement;
    if (t.closest("[data-ql-close]")) return void closeQuickLook();
    const act = t.closest<HTMLElement>("[data-act]")?.dataset.act;
    if (!act) return;
    const i = selected;
    if (act === "paste" || act === "plain") void closeQuickLook().then(() => choose(i, act === "plain"));
    else if (act === "copy") void closeQuickLook().then(() => copyOnly(i));
    else if (act === "pin") void closeQuickLook().then(() => togglePin(i));
    else if (act === "delete") void closeQuickLook().then(() => remove(i));
  });

  filtersNav.addEventListener("click", (e) => {
    const b = (e.target as HTMLElement).closest<HTMLButtonElement>("button");
    if (b?.dataset.filter) setFilter(b.dataset.filter as Filter);
  });
  $("queue-toggle").addEventListener("click", async () => {
    await api.setQueueActive(!status.queueActive);
    status = await api.getStatus();
    toast(status.queueActive ? "Paste queue on: new copies line up" : "Paste queue off", "queue");
    render("none", view[selected]?.id);
  });
  $("pause-toggle").addEventListener("click", async () => {
    await api.setPaused(!status.paused);
    status = await api.getStatus();
    toast(status.paused ? "Capture paused" : "Capture resumed", status.paused ? "pause" : "play");
    renderChrome();
  });
  $("settings-btn").addEventListener("click", () => openSettings(settings, status, onSettingsSaved));
  $("mode-toggle").addEventListener("click", () => void setDisplay(display() === "fullscreen" ? "window" : "fullscreen"));
  $("close-btn").addEventListener("click", () => close());
  banner.addEventListener("click", (e) => {
    const t = e.target as HTMLElement;
    if (t.closest("[data-dismiss]")) {
      bannerDismissed = true;
      renderBanner();
    } else if (t.closest("[data-open-settings]")) {
      openSettings(settings, status, onSettingsSaved, "shortcuts");
    } else if (t.closest("[data-notice-action]")) {
      void api.openSystemSettings(t.closest<HTMLElement>("[data-notice-action]")!.dataset.noticeAction!);
    } else if (t.closest("[data-dismiss-notice]")) {
      dismissedNotices.add(t.closest<HTMLElement>("[data-dismiss-notice]")!.dataset.dismissNotice!);
      renderBanner();
    }
  });

  let resizeFrame = 0;
  new ResizeObserver(() => {
    cancelAnimationFrame(resizeFrame);
    resizeFrame = requestAnimationFrame(() => {
      if (settings && !field.classList.contains("leaving")) render("none", view[selected]?.id);
    });
  }).observe(stage);

  media.reduce.addEventListener("change", () => applyMotion(settings));

  void on("history-changed", () => void loadItems("flip"));
  void on<Status>("status-changed", (s) => {
    status = s;
    setPlatform(s.platform);
    root.dataset.glass = s.glass ? "on" : "off";
    render("none", view[selected]?.id);
  });
  void on<Settings>("settings-changed", (s) => {
    settings = s;
    applyMotion(s);
    renderChrome();
  });
  // Warm-window lifecycle (only when "keep dashboard ready" is on).
  void on<string>("dashboard-shown", (m) => void reopen(m));
  void on("dashboard-hidden", () => {
    app.classList.remove("leaving", "entered");
    field.classList.remove("leaving");
    closeSettings();
    void closeQuickLook();
  });
}

function onSettingsSaved(next: Settings): void {
  const modeChanged = next.displayMode !== display();
  settings = next;
  applyMotion(next);
  if (modeChanged) {
    root.dataset.display = next.displayMode;
    requestAnimationFrame(() => render("bubble", view[selected]?.id));
  }
  renderChrome();
}

function playEnter(): void {
  app.classList.remove("leaving", "entered");
  void app.offsetWidth;
  app.classList.add("entered");
}

async function reopen(nextMode: string): Promise<void> {
  mode = nextMode;
  closing = false;
  filter = "all";
  searchMode = false;
  query = "";
  search.value = "";
  selected = 0;
  lastPointer = { x: -1, y: -1 };
  [settings, status] = await Promise.all([api.getSettings(), api.getStatus()]);
  setPlatform(status.platform);
  applyMotion(settings);
  root.dataset.glass = status.glass ? "on" : "off";
  root.dataset.display = display();
  stage.scrollTop = 0;
  field.classList.remove("leaving");
  playEnter();
  await loadItems("bubble");
  search.focus();
  if (mode === "settings") openSettings(settings, status, onSettingsSaved);
}

async function init(): Promise<void> {
  hydrateIcons();
  app.querySelector(".mark")!.innerHTML = mark;
  filtersNav.insertAdjacentHTML(
    "beforeend",
    FILTERS.map((x) => `<button type="button" role="tab" data-filter="${x.id}">${x.label}</button>`).join(""),
  );
  wireEvents();
  [settings, status] = await Promise.all([api.getSettings(), api.getStatus()]);
  setPlatform(status.platform);
  applyMotion(settings);
  root.dataset.glass = status.glass ? "on" : "off";
  root.dataset.display = display();
  await document.fonts.ready;
  try {
    items = await api.listItems();
  } catch (err) {
    loadError = String(err);
  }
  // Show the window first, then bubble the tiles in once it has its size.
  void api.ready();
  await frame();
  await frame();
  playEnter();
  render("bubble");
  moveFilterIndicator(true);
  search.focus();
  if (mode === "settings") openSettings(settings, status, onSettingsSaved);
}

void init();
