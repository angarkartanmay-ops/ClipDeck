// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

import "./styles.css";
import { api, inTauri, on, type Item, type Settings, type Status } from "./api";
import { bytes, escapeHtml, timeAgo } from "./format";
import { fuzzyScore } from "./fuzzy";
import { deckArt, hydrateIcons, icons } from "./icons";
import { closeSettings, openSettings, settingsOpen } from "./settings-view";

type Filter = "all" | "text" | "image" | "files" | "pinned";

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "text", label: "Text" },
  { id: "image", label: "Images" },
  { id: "files", label: "Files" },
  { id: "pinned", label: "Pinned" },
];

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const root = document.documentElement;
const panel = $("panel");
const body = panel.querySelector<HTMLElement>(".body")!;
const listScroll = $("list-scroll");
const list = $("list");
const highlight = $("highlight");
const preview = $("preview");
const search = $<HTMLInputElement>("search");
const searchBox = search.closest<HTMLElement>(".search")!;
const filtersNav = $("filters");
const filterIndicator = filtersNav.querySelector<HTMLElement>(".filter-indicator")!;
const banner = $("banner");
const hints = $("hints");
const statusEl = $("status");
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
let loadError: string | null = null;
let bannerDismissed = false;
let mode = new URLSearchParams(location.search).get("mode") ?? "";
const thumbs = new Map<number, string>();

if (!inTauri) root.classList.add("mock");

// ---------------------------------------------------------------- appearance

const media = {
  dark: matchMedia("(prefers-color-scheme: dark)"),
  reduce: matchMedia("(prefers-reduced-motion: reduce)"),
};

function applyAppearance(s: Settings): void {
  root.dataset.theme = s.theme === "system" ? (media.dark.matches ? "dark" : "light") : s.theme;
  root.style.setProperty("--accent", s.accent);
  const reduce = s.reduceMotion === "on" || (s.reduceMotion === "system" && media.reduce.matches);
  root.dataset.motion = reduce ? "reduce" : "full";
}

/** Glass needs the native acrylic backdrop; without it the panel is opaque. */
function applyWindowStyle(s: Status): void {
  root.dataset.glass = s.glass ? "on" : "off";
}

const motionReduced = () => root.dataset.motion === "reduce";
const wait = (ms: number) => new Promise((r) => setTimeout(r, motionReduced() ? 0 : ms));

// ---------------------------------------------------------------- item helpers

type Flavor = "text" | "rich" | "code" | "link" | "color" | "image" | "files";

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

/** Row title as [main text, muted suffix]. */
function titleOf(item: Item, f: Flavor): [string, string] {
  switch (f) {
    case "image":
      return ["Image", item.meta.width ? `${item.meta.width} × ${item.meta.height}` : ""];
    case "files": {
      const names = item.meta.files ?? [];
      const n = item.meta.fileCount ?? names.length;
      return n === 1 ? [names[0] ?? "1 file", ""] : [`${n} files`, names.slice(0, 3).join(", ")];
    }
    case "link":
      return [item.text.trim().replace(/^https?:\/\/(www\.)?/i, ""), ""];
    default: {
      const line = item.text.split(/\r?\n/).find((l) => l.trim()) ?? "";
      return [line.trim().replace(/\s+/g, " ").slice(0, 160) || "Empty text", ""];
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

const KIND_LABEL: Record<Flavor, string> = {
  text: "Text",
  rich: "Rich text",
  code: "Code",
  link: "Link",
  color: "Colour",
  image: "Image",
  files: "Files",
};

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
  if (!marks.length) return escapeHtml(text);
  marks.sort((a, b) => a[0] - b[0]);
  let out = "";
  let pos = 0;
  for (const [s, e] of marks) {
    if (s < pos) continue;
    out += escapeHtml(text.slice(pos, s)) + `<mark>${escapeHtml(text.slice(s, e))}</mark>`;
    pos = e;
  }
  return out + escapeHtml(text.slice(pos));
}

// ---------------------------------------------------------------- data

async function loadItems(animate: "enter" | "flip" | "none" = "flip"): Promise<void> {
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

function loadImages(scope: ParentNode): void {
  scope.querySelectorAll<HTMLImageElement>("img[data-thumb]").forEach(async (img) => {
    const src = await thumb(Number(img.dataset.thumb));
    if (!src) return;
    img.onload = () => img.classList.add("loaded");
    img.src = src;
  });
}

// ---------------------------------------------------------------- list

function tile(item: Item, f: Flavor): string {
  if (f === "image") return `<span class="tile"><img data-thumb="${item.id}" alt="" draggable="false"/>${icons.image}</span>`;
  if (f === "color") return `<span class="tile"><span class="swatch" style="background:${item.text.trim()}"></span></span>`;
  return `<span class="tile">${icons[f] ?? icons.text}</span>`;
}

function row(item: Item, index: number): string {
  const f = flavor(item);
  const queuePos = status?.queue.indexOf(item.id) ?? -1;
  const slot = !searchMode && index < 9 ? `<kbd class="key">${index + 1}</kbd>` : "";
  const mono = f === "code" || f === "color";
  const [title, sub] = titleOf(item, f);
  return `<div class="row" role="option" id="row-${item.id}" data-index="${index}" data-id="${item.id}" style="--i:${Math.min(index, 12)}" aria-selected="false">
    ${tile(item, f)}
    <span class="row-text">
      <span class="title${mono ? " mono" : ""}">${highlightMatches(title)}${sub ? ` <span class="sub">${highlightMatches(sub)}</span>` : ""}</span>
      <span class="meta">${escapeHtml(item.sourceApp || "Unknown app")} · ${timeAgo(item.createdAt)}</span>
    </span>
    <span class="trail">
      ${queuePos >= 0 ? `<span class="queued">Q${queuePos + 1}</span>` : ""}
      ${item.pinned ? `<span class="pin-flag" title="Pinned">${icons.pinned}</span>` : ""}
      ${slot}
    </span>
  </div>`;
}

function skeleton(): string {
  return Array.from(
    { length: 7 },
    (_, i) => `<div class="row skeleton" style="--i:${i}"><span class="tile"></span><span class="row-text"><span class="bar"></span><span class="bar short"></span></span><span></span></div>`,
  ).join("");
}

function emptyState(): string {
  if (loadError) {
    return `<div class="empty"><div class="error-box">${icons.alert}<h2>Couldn't read your history</h2><p>${escapeHtml(loadError)}</p><button class="btn" data-retry>Try again</button></div></div>`;
  }
  if (items.length === 0) {
    return `<div class="empty">${deckArt}<h2>Nothing on the deck yet</h2>
      <p>Copy text, a screenshot or some files and they stack up here. Bring this back with <kbd class="key">${escapeHtml(settings.hotkeyDashboard)}</kbd>.</p></div>`;
  }
  if (query) {
    return `<div class="empty">${icons.search}<h2>No matches for “${escapeHtml(query)}”</h2><p>Try fewer letters, or press <kbd class="key">Tab</kbd> to switch filters.</p></div>`;
  }
  const label = FILTERS.find((f) => f.id === filter)?.label.toLowerCase();
  return `<div class="empty">${icons[filter === "pinned" ? "pin" : "history"]}<h2>No ${label} yet</h2><p>${
    filter === "pinned" ? "Press <kbd class=\"key\">P</kbd> on any item to keep it forever." : "Copied items of this type will show up here."
  }</p></div>`;
}

/** Re-renders the list. `flip` animates rows from their old positions. */
function render(animate: "enter" | "flip" | "none" = "none", keepId?: number): void {
  const before = animate === "flip" ? positions() : null;
  view = computeView();
  if (keepId !== undefined) {
    const idx = view.findIndex((i) => i.id === keepId);
    selected = idx >= 0 ? idx : Math.min(selected, view.length - 1);
  }
  selected = Math.max(0, Math.min(selected, view.length - 1));

  const isEmpty = view.length === 0;
  body.classList.toggle("is-empty", isEmpty);
  list.classList.toggle("is-empty", isEmpty);
  list.querySelectorAll(".row, .empty").forEach((el) => el.remove());
  list.insertAdjacentHTML("beforeend", isEmpty ? emptyState() : view.map(row).join(""));

  if (animate === "enter") list.querySelectorAll(".row").forEach((el) => el.classList.add("enter"));
  if (before) flip(before);
  loadImages(list);
  updateSelection(selected, { scroll: true, instant: animate === "enter" });
  renderChrome();
}

function positions(): Map<string, number> {
  const map = new Map<string, number>();
  list.querySelectorAll<HTMLElement>(".row[data-id]").forEach((el) => map.set(el.dataset.id!, el.offsetTop));
  return map;
}

/** FLIP: rows that moved slide from where they were; new rows fade in. */
function flip(before: Map<string, number>): void {
  if (motionReduced()) return;
  list.querySelectorAll<HTMLElement>(".row[data-id]").forEach((el) => {
    const old = before.get(el.dataset.id!);
    if (old === undefined) {
      el.animate([{ opacity: 0, transform: "scale(0.97)" }, { opacity: 1, transform: "none" }], {
        duration: 260,
        easing: "cubic-bezier(0.16, 1, 0.3, 1)",
      });
      return;
    }
    const dy = old - el.offsetTop;
    if (dy) {
      el.animate([{ transform: `translateY(${dy}px)` }, { transform: "none" }], {
        duration: 300,
        easing: "cubic-bezier(0.22, 0.9, 0.3, 1)",
      });
    }
  });
}

function updateSelection(next: number, opts: { scroll?: boolean; instant?: boolean } = {}): void {
  if (!view.length) {
    renderPreview();
    return;
  }
  const prev = selected;
  selected = Math.max(0, Math.min(next, view.length - 1));
  if (opts.instant) {
    highlight.style.transition = "none";
    void highlight.offsetWidth;
  }
  highlight.style.setProperty("--sel", String(selected));
  if (opts.instant) requestAnimationFrame(() => highlight.style.removeProperty("transition"));

  list.querySelectorAll<HTMLElement>(".row[data-index]").forEach((el) => {
    const on = Number(el.dataset.index) === selected;
    el.classList.toggle("selected", on);
    el.setAttribute("aria-selected", String(on));
  });
  list.setAttribute("aria-activedescendant", `row-${view[selected].id}`);

  if (opts.scroll) {
    const pitch = list.querySelector<HTMLElement>(".row")?.offsetHeight ?? 40;
    const top = selected * (pitch + 2);
    const pad = 6;
    if (top < listScroll.scrollTop) listScroll.scrollTo({ top: top - pad, behavior: motionReduced() ? "auto" : "smooth" });
    else if (top + pitch > listScroll.scrollTop + listScroll.clientHeight - pad * 2)
      listScroll.scrollTo({ top: top + pitch - listScroll.clientHeight + pad * 2, behavior: motionReduced() ? "auto" : "smooth" });
  }
  if (prev !== selected || !preview.dataset.id || Number(preview.dataset.id) !== view[selected].id) renderPreview();
}

// ---------------------------------------------------------------- preview

function stageFor(item: Item, f: Flavor): string {
  const text = item.text;
  switch (f) {
    case "image": {
      const dims = item.meta.width ? `${item.meta.width} × ${item.meta.height}` : "";
      return `<div class="image-view"><img data-thumb="${item.id}" alt="Copied image" draggable="false"/><div class="placeholder"></div>${
        dims ? `<span class="chip">${dims}</span>` : ""
      }</div>`;
    }
    case "files": {
      const paths = text.split(/\r?\n/).filter(Boolean);
      return `<div class="file-view">${paths
        .map((p) => {
          const name = p.split(/[\\/]/).pop() ?? p;
          const isDir = !/\.[a-z0-9]{1,6}$/i.test(name);
          return `<div class="file"><span class="tile">${isDir ? icons.folder : icons.file}</span><span class="row-text"><span class="name">${escapeHtml(
            name,
          )}</span><span class="path">${escapeHtml(p)}</span></span></div>`;
        })
        .join("")}</div>`;
    }
    case "color": {
      const hex = text.trim();
      const full = hex.length === 4 ? `#${[...hex.slice(1)].map((c) => c + c).join("")}` : hex;
      const n = parseInt(full.slice(1), 16);
      const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
      return `<div class="color-view"><div class="big" style="background:${full}"></div><dl><dt>HEX</dt><dd>${escapeHtml(
        full.toUpperCase(),
      )}</dd><dt>RGB</dt><dd>${r}, ${g}, ${b}</dd></dl></div>`;
    }
    case "link": {
      let host = text.trim();
      let rest = "";
      try {
        const u = new URL(text.trim());
        host = u.host.replace(/^www\./, "");
        rest = `${u.pathname}${u.search}${u.hash}`;
      } catch {
        /* show as-is */
      }
      return `<div class="link-view"><span class="tile">${icons.link}</span><span class="host">${escapeHtml(host)}</span>${
        rest && rest !== "/" ? `<span class="path">${escapeHtml(rest)}</span>` : ""
      }</div>`;
    }
    case "code":
      return `<pre class="code">${escapeHtml(text)}</pre>`;
    default:
      return `<pre>${escapeHtml(text)}</pre>`;
  }
}

function renderPreview(): void {
  const item = view[selected];
  if (!item) {
    preview.innerHTML = "";
    delete preview.dataset.id;
    return;
  }
  const f = flavor(item);
  preview.dataset.id = String(item.id);
  const textual = item.kind === "text" || item.kind === "rich";
  const verb = status?.canPaste ? "Paste" : "Copy";
  const queued = status?.queue.includes(item.id);
  const extra =
    f === "image"
      ? bytes(item.size)
      : f === "files"
        ? `${item.meta.fileCount ?? 0} item${item.meta.fileCount === 1 ? "" : "s"}`
        : `${(item.meta.chars ?? item.text.length).toLocaleString()} chars`;
  preview.innerHTML = `
    <div class="stage"><div class="stage-content">${stageFor(item, f)}</div></div>
    <div class="details">
      <span class="tag">${KIND_LABEL[f]}</span>
      <span>${icons.app}${escapeHtml(item.sourceApp || "Unknown app")}</span>
      <span title="${new Date(item.createdAt).toLocaleString()}">${icons.clock}${when(item.createdAt)}</span>
      <span>${extra}</span>
    </div>
    <div class="actions">
      <button class="btn primary" data-act="paste">${icons.paste}${verb}<kbd class="key">↵</kbd></button>
      ${textual && item.kind === "rich" ? `<button class="btn" data-act="plain" title="Paste without formatting">Plain<kbd class="key">⇧↵</kbd></button>` : ""}
      <span class="spacer"></span>
      <button class="icon-btn${item.pinned ? " on" : ""}" data-act="pin" title="${item.pinned ? "Unpin" : "Pin"} (P)" aria-label="${item.pinned ? "Unpin" : "Pin"}">${icons.pin}</button>
      <button class="icon-btn${queued ? " on" : ""}" data-act="queue" title="${queued ? "Remove from queue" : "Add to paste queue"} (Q)" aria-label="Queue">${icons.queue}</button>
      <button class="icon-btn" data-act="copy" title="Copy only (Ctrl+C)" aria-label="Copy">${icons.copy}</button>
      <button class="icon-btn danger" data-act="delete" title="Delete (Del)" aria-label="Delete">${icons.trash}</button>
    </div>`;
  loadImages(preview);
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
  filtersNav.querySelectorAll<HTMLButtonElement>("button").forEach((b) => {
    const active = b.dataset.filter === filter;
    b.classList.toggle("active", active);
    b.setAttribute("aria-selected", String(active));
  });
  moveFilterIndicator();
  searchBox.classList.toggle("typing", searchMode || query.length > 0);

  const queueBtn = $("queue-toggle");
  queueBtn.classList.toggle("on", status.queueActive);
  queueBtn.title = status.queueActive
    ? `Paste queue on: ${settings.hotkeyDashboard} pastes the next item. Click to stop.`
    : `Paste queue (${settings.hotkeyQueue})`;
  $("queue-count").textContent = status.queueActive && status.queue.length ? String(status.queue.length) : "";

  const pauseBtn = $("pause-toggle");
  pauseBtn.classList.toggle("on", status.paused);
  pauseBtn.innerHTML = status.paused ? icons.play : icons.pause;
  pauseBtn.title = status.paused ? "Resume capture" : "Pause capture";
  $("settings-btn").title = "Settings";

  statusEl.classList.toggle("paused", status.paused);
  statusEl.innerHTML = `<span class="dot"></span><span>${status.paused ? "Paused" : "Capturing"}</span><span class="num">${
    status.itemCount
  } · ${bytes(status.diskBytes)}</span>`;

  const verb = status.canPaste ? "paste" : "copy";
  hints.innerHTML = searchMode
    ? `<span><kbd class="key">↑↓</kbd>move</span><span><kbd class="key">↵</kbd>${verb}</span><span><kbd class="key">Alt 1–9</kbd>quick</span><span><kbd class="key">Esc</kbd>clear</span>`
    : `<span><kbd class="key">↑↓</kbd>move</span><span><kbd class="key">↵</kbd>${verb}</span><span><kbd class="key">P</kbd>pin</span><span><kbd class="key">Q</kbd>queue</span><span><kbd class="key">Del</kbd>remove</span><span><kbd class="key">Esc</kbd>close</span>`;

  renderBanner();
}

function renderBanner(): void {
  let html = "";
  let warn = false;
  if (status.hotkeyErrors.length) {
    warn = true;
    html = `${icons.alert}<div><b>Hotkey conflict.</b> ${status.hotkeyErrors.map(escapeHtml).join("; ")}.</div><button class="btn" data-open-settings>Fix</button>`;
  } else if (mode === "welcome" && !bannerDismissed) {
    html = `${icons.keyboard}<div><b>ClipDeck is running in your tray.</b> Copy as usual, then press <kbd class="key">${escapeHtml(
      settings.hotkeyDashboard,
    )}</kbd> to pick what to paste. Plain <kbd class="key">Ctrl+V</kbd> is unchanged.</div><button class="btn" data-dismiss>Got it</button>`;
  }
  banner.classList.toggle("warn", warn);
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

function close(): void {
  if (closing) return;
  closing = true;
  panel.classList.add("closing");
  window.setTimeout(() => void api.close(), motionReduced() ? 0 : 100);
}

function rowEl(index: number): HTMLElement | null {
  return list.querySelector<HTMLElement>(`.row[data-index="${index}"]`);
}

async function choose(index: number, plain = false): Promise<void> {
  const item = view[index];
  if (!item || closing) return;
  closing = true;
  updateSelection(index);
  rowEl(index)?.classList.add("flash");
  panel.classList.add("closing");
  await wait(70);
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
  rowEl(index)?.classList.add("flash");
  panel.classList.add("closing");
  await wait(70);
  await api.copy(item.id);
  if (!inTauri) {
    toast("Copied to clipboard", "copy");
    resetAfterMockClose();
  }
}

function resetAfterMockClose(): void {
  window.setTimeout(() => {
    closing = false;
    panel.classList.remove("closing");
  }, 700);
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
  toast(n >= 0 ? `Queued #${n + 1}: ${settings.hotkeyDashboard} pastes in order` : "Removed from queue", "queue");
  render("none", item.id);
}

const removing = new Set<number>();

async function remove(index: number): Promise<void> {
  const item = view[index];
  if (!item || removing.has(item.id)) return;
  removing.add(item.id);
  rowEl(index)?.classList.add("leaving");
  await wait(140);
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
  listScroll.scrollTop = 0;
  render("enter");
}

function setQuery(next: string): void {
  query = next;
  if (search.value !== next) search.value = next;
  if (next) searchMode = true;
  selected = 0;
  listScroll.scrollTop = 0;
  render("none");
}

function exitSearch(): void {
  searchMode = false;
  setQuery("");
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

  switch (e.key) {
    case "Escape":
      e.preventDefault();
      if (searchMode || query) exitSearch();
      else close();
      return;
    case "ArrowDown":
    case "ArrowUp":
      e.preventDefault();
      updateSelection(selected + (e.key === "ArrowDown" ? 1 : -1), { scroll: true });
      return;
    case "PageDown":
    case "PageUp":
      e.preventDefault();
      updateSelection(selected + (e.key === "PageDown" ? 8 : -8), { scroll: true });
      return;
    case "Home":
    case "End":
      if (query) return; // caret movement in the search box
      e.preventDefault();
      updateSelection(e.key === "Home" ? 0 : view.length - 1, { scroll: true });
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

  if (e.ctrlKey || e.altKey || e.metaKey) return;
  const k = e.key.toLowerCase();
  if (/^[1-9]$/.test(k)) {
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
  } else if (e.key.length === 1 && e.key !== " ") {
    // Any other printable key starts a search and is typed normally.
    searchMode = true;
  }
}

// ---------------------------------------------------------------- wiring

let lastPointer = { x: -1, y: -1 };

function wireEvents(): void {
  document.addEventListener("keydown", onKeyDown);
  search.addEventListener("input", () => setQuery(search.value));
  // Keep typing focus in the search box whatever is clicked.
  document.addEventListener("mousedown", (e) => {
    const t = e.target as HTMLElement;
    if (!t.closest("input, textarea, #settings, .stage pre, .stage .prose")) e.preventDefault();
  });

  list.addEventListener("mousemove", (e) => {
    if (e.clientX === lastPointer.x && e.clientY === lastPointer.y) return;
    // The first event after opening is Chromium's synthetic one for a cursor
    // that is merely resting over the list; only real movement selects.
    const first = lastPointer.x < 0;
    lastPointer = { x: e.clientX, y: e.clientY };
    if (first) return;
    const el = (e.target as HTMLElement).closest<HTMLElement>(".row[data-index]");
    if (el && Number(el.dataset.index) !== selected) updateSelection(Number(el.dataset.index));
  });
  list.addEventListener("click", (e) => {
    const t = e.target as HTMLElement;
    if (t.closest("[data-retry]")) return void loadItems("enter");
    const el = t.closest<HTMLElement>(".row[data-index]");
    if (el) void choose(Number(el.dataset.index), e.shiftKey);
  });

  preview.addEventListener("click", (e) => {
    const act = (e.target as HTMLElement).closest<HTMLElement>("[data-act]")?.dataset.act;
    if (act === "paste") void choose(selected);
    else if (act === "plain") void choose(selected, true);
    else if (act === "pin") void togglePin(selected);
    else if (act === "queue") void toggleQueue(selected);
    else if (act === "copy") void copyOnly(selected);
    else if (act === "delete") void remove(selected);
  });
  preview.addEventListener("mousemove", (e) => {
    const stage = (e.target as HTMLElement).closest<HTMLElement>(".stage");
    if (!stage || motionReduced()) return;
    const r = stage.getBoundingClientRect();
    stage.style.setProperty("--mx", `${e.clientX - r.left}px`);
    stage.style.setProperty("--my", `${e.clientY - r.top}px`);
  });
  preview.addEventListener("dblclick", (e) => {
    if ((e.target as HTMLElement).closest(".stage")) void choose(selected);
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
  banner.addEventListener("click", (e) => {
    const t = e.target as HTMLElement;
    if (t.closest("[data-dismiss]")) {
      bannerDismissed = true;
      renderBanner();
    } else if (t.closest("[data-open-settings]")) {
      openSettings(settings, status, onSettingsSaved, "shortcuts");
    }
  });

  media.dark.addEventListener("change", () => applyAppearance(settings));
  media.reduce.addEventListener("change", () => applyAppearance(settings));

  void on("history-changed", () => void loadItems("flip"));
  void on<Status>("status-changed", (s) => {
    status = s;
    applyWindowStyle(s);
    render("none", view[selected]?.id);
  });
  void on<Settings>("settings-changed", (s) => {
    settings = s;
    applyAppearance(s);
    renderChrome();
  });
  // Warm-window lifecycle (only when "keep dashboard ready" is on).
  void on<string>("dashboard-shown", (m) => void reopen(m));
  void on("dashboard-hidden", () => {
    panel.classList.remove("closing", "entered");
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
  lastPointer = { x: -1, y: -1 };
  closing = false;
  filter = "all";
  searchMode = false;
  query = "";
  search.value = "";
  selected = 0;
  [settings, status] = await Promise.all([api.getSettings(), api.getStatus()]);
  applyAppearance(settings);
  applyWindowStyle(status);
  listScroll.scrollTop = 0;
  await loadItems("enter");
  playEnter();
  search.focus();
  if (mode === "settings") openSettings(settings, status, onSettingsSaved);
}

async function init(): Promise<void> {
  hydrateIcons();
  filtersNav.insertAdjacentHTML(
    "beforeend",
    FILTERS.map((f) => `<button type="button" role="tab" data-filter="${f.id}">${f.label}</button>`).join(""),
  );
  list.insertAdjacentHTML("beforeend", skeleton());
  wireEvents();
  [settings, status] = await Promise.all([api.getSettings(), api.getStatus()]);
  applyAppearance(settings);
  applyWindowStyle(status);
  await document.fonts.ready;
  await loadItems("enter");
  moveFilterIndicator(true);
  // Show the window only once the first frame has content.
  requestAnimationFrame(() => {
    playEnter();
    search.focus();
    void api.ready();
  });
  if (mode === "settings") openSettings(settings, status, onSettingsSaved);
}

void init();
