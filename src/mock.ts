// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

// Sample data so the dashboard can be designed in a plain browser
// (`npm run dev`) without the Rust core. Never used inside the app.

import type { Item, Settings, Status } from "./api";

const now = Date.now();
const min = 60_000;

const swatch = (a: string, b: string) =>
  "data:image/svg+xml;utf8," +
  encodeURIComponent(
    `<svg xmlns='http://www.w3.org/2000/svg' width='480' height='300'><defs><linearGradient id='g' x2='1' y2='1'><stop offset='0' stop-color='${a}'/><stop offset='1' stop-color='${b}'/></linearGradient></defs><rect width='480' height='300' fill='url(#g)'/><circle cx='360' cy='90' r='50' fill='white' opacity='.35'/><rect x='40' y='190' width='240' height='24' rx='12' fill='white' opacity='.5'/></svg>`,
  );

let items: Item[] = [
  { id: 9, kind: "text", createdAt: now - 20_000, sourceApp: "Visual Studio Code", sourceExe: "code.exe", pinned: false, size: 120, text: "const store = Store::open(&data_dir)?;\nstore.enforce_limits(50, 500 * MB)?;", meta: { chars: 68, lines: 2 }, hasThumb: false },
  { id: 8, kind: "image", createdAt: now - 3 * min, sourceApp: "Snipping Tool", sourceExe: "snippingtool.exe", pinned: false, size: 840_000, text: "", meta: { width: 1920, height: 1080 }, hasThumb: true },
  { id: 7, kind: "rich", createdAt: now - 12 * min, sourceApp: "Google Chrome", sourceExe: "chrome.exe", pinned: true, size: 2_400, text: "Windows clipboard history is limited to 25 items and clears on restart unless items are pinned.", meta: { chars: 96, lines: 1 }, hasThumb: false },
  { id: 6, kind: "files", createdAt: now - 40 * min, sourceApp: "Windows Explorer", sourceExe: "explorer.exe", pinned: false, size: 400, text: "C:\\Users\\me\\Pictures\\holiday.jpg\nC:\\Users\\me\\Documents\\report-final.docx\nC:\\Users\\me\\Desktop\\notes.txt", meta: { files: ["holiday.jpg", "report-final.docx", "notes.txt"], fileCount: 3 }, hasThumb: false },
  { id: 5, kind: "text", createdAt: now - 2 * 60 * min, sourceApp: "Discord", sourceExe: "discord.exe", pinned: false, size: 60, text: "https://github.com/angarkartanmay-ops/ClipDeck", meta: { chars: 46, lines: 1 }, hasThumb: false },
  { id: 4, kind: "image", createdAt: now - 5 * 60 * min, sourceApp: "Figma", sourceExe: "figma.exe", pinned: true, size: 1_200_000, text: "", meta: { width: 1280, height: 800 }, hasThumb: true },
  { id: 3, kind: "text", createdAt: now - 26 * 60 * min, sourceApp: "Notepad", sourceExe: "notepad.exe", pinned: false, size: 300, text: "Meeting notes\n- ship M3 by Friday\n- test paste into Word, Paint, VS Code\n- measure idle RAM over 4 hours", meta: { chars: 96, lines: 4 }, hasThumb: false },
  { id: 2, kind: "rich", createdAt: now - 3 * 24 * 60 * min, sourceApp: "Microsoft Word", sourceExe: "winword.exe", pinned: false, size: 9_000, text: "Quarterly summary: revenue up 12%, churn down to 2.1%.", meta: { chars: 54, lines: 1 }, hasThumb: false },
  { id: 1, kind: "text", createdAt: now - 9 * 24 * 60 * min, sourceApp: "Windows Terminal", sourceExe: "windowsterminal.exe", pinned: false, size: 40, text: "npm run tauri build", meta: { chars: 19, lines: 1 }, hasThumb: false },
];

let settings: Settings = {
  historyLimit: 50, maxDiskMb: 500, maxItemMb: 64,
  hotkeyDashboard: "Ctrl+Shift+V", hotkeyQueue: "Ctrl+Shift+Q", slotModifiers: "Ctrl+Alt",
  pasteOnSelect: true, restoreClipboard: false, position: "cursor", keepDashboardWarm: false,
  theme: "system", accent: "#7c5cff", reduceMotion: "system",
  captureImages: true, captureFiles: true, captureRich: true,
  blocklist: ["1password.exe", "bitwarden.exe", "keepass.exe"], encrypt: false, clearOnExit: false, autostart: true,
};

const status: Status = { paused: false, queueActive: false, queue: [], hotkeyErrors: [], itemCount: 9, diskBytes: 2_500_000, canPaste: true, version: "0.1.0-dev" };

export async function mockApi<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const id = args.id as number;
  const result = ((): unknown => {
    switch (cmd) {
      case "list_items": return items;
      case "get_thumb": return id === 8 ? swatch("#ff8a5c", "#c13cff") : swatch("#2fd6a8", "#2a6cff");
      case "set_pinned": items = items.map((i) => (i.id === id ? { ...i, pinned: args.pinned as boolean } : i)); return;
      case "delete_item": items = items.filter((i) => i.id !== id); return;
      case "clear_history": items = args.keepPinned ? items.filter((i) => i.pinned) : []; return;
      case "get_settings": return settings;
      case "save_settings": settings = args.settings as Settings; return settings;
      case "get_status": return status;
      case "set_paused": status.paused = args.paused as boolean; return;
      case "set_queue_active": status.queueActive = args.active as boolean; if (!status.queueActive) status.queue = []; return;
      case "toggle_queued": status.queue = status.queue.includes(id) ? status.queue.filter((q) => q !== id) : [...status.queue, id]; status.queueActive = true; return;
      default: console.info("[mock]", cmd, args); return;
    }
  })();
  return structuredClone(result) as T;
}
