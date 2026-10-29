// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

// Sample data so the dashboard can be designed in a plain browser
// (`npm run dev`) without the Rust core. Never used inside the app.

import type { Item, Settings, Status } from "./api";

const now = Date.now();
const min = 60_000;

const swatch = (a: string, b: string, w: number, h: number) => {
  const W = 480;
  const H = Math.round((W * h) / w);
  return (
    "data:image/svg+xml;utf8," +
    encodeURIComponent(
      `<svg xmlns='http://www.w3.org/2000/svg' width='${W}' height='${H}'><defs><linearGradient id='g' x2='1' y2='1'><stop offset='0' stop-color='${a}'/><stop offset='1' stop-color='${b}'/></linearGradient></defs><rect width='${W}' height='${H}' fill='url(#g)'/><circle cx='${W * 0.72}' cy='${H * 0.3}' r='${Math.min(W, H) * 0.12}' fill='white' opacity='.35'/><rect x='${W * 0.08}' y='${H * 0.66}' width='${W * 0.5}' height='${H * 0.07}' rx='${H * 0.035}' fill='white' opacity='.5'/></svg>`,
    )
  );
};

let items: Item[] = [
  { id: 9, kind: "text", createdAt: now - 20_000, sourceApp: "Visual Studio Code", sourceExe: "code.exe", pinned: false, size: 120, text: "const store = Store::open(&data_dir)?;\nstore.enforce_limits(50, 500 * MB)?;", meta: { chars: 68, lines: 2 }, hasThumb: false },
  { id: 8, kind: "image", createdAt: now - 3 * min, sourceApp: "Snipping Tool", sourceExe: "snippingtool.exe", pinned: false, size: 840_000, text: "", meta: { width: 1920, height: 1080 }, hasThumb: true },
  { id: 7, kind: "rich", createdAt: now - 12 * min, sourceApp: "Google Chrome", sourceExe: "chrome.exe", pinned: true, size: 2_400, text: "Windows clipboard history is limited to 25 items and clears on restart unless items are pinned.", meta: { chars: 96, lines: 1 }, hasThumb: false },
  { id: 6, kind: "files", createdAt: now - 40 * min, sourceApp: "Windows Explorer", sourceExe: "explorer.exe", pinned: false, size: 400, text: "C:\\Users\\me\\Pictures\\holiday.jpg\nC:\\Users\\me\\Documents\\report-final.docx\nC:\\Users\\me\\Desktop\\notes.txt", meta: { files: ["holiday.jpg", "report-final.docx", "notes.txt"], fileCount: 3 }, hasThumb: false },
  { id: 5, kind: "text", createdAt: now - 2 * 60 * min, sourceApp: "Discord", sourceExe: "discord.exe", pinned: false, size: 60, text: "https://github.com/angarkartanmay-ops/ClipDeck", meta: { chars: 46, lines: 1 }, hasThumb: false },
  { id: 4, kind: "image", createdAt: now - 5 * 60 * min, sourceApp: "Figma", sourceExe: "figma.exe", pinned: true, size: 1_200_000, text: "", meta: { width: 1280, height: 800 }, hasThumb: true },
  { id: 3, kind: "text", createdAt: now - 26 * 60 * min, sourceApp: "Notepad", sourceExe: "notepad.exe", pinned: false, size: 300, text: "Meeting notes\n- ship M3 by Friday\n- test paste into Word, Paint, VS Code\n- measure idle RAM over 4 hours", meta: { chars: 96, lines: 4 }, hasThumb: false },
  { id: 2, kind: "rich", createdAt: now - 3 * 24 * 60 * min, sourceApp: "Microsoft Word", sourceExe: "winword.exe", pinned: false, size: 9_000, text: "Quarterly summary: revenue up 12%, churn down to 2.1%.", meta: { chars: 54, lines: 1 }, hasThumb: false },
  { id: 14, kind: "image", createdAt: now - 50_000, sourceApp: "Snipping Tool", sourceExe: "snippingtool.exe", pinned: false, size: 520_000, text: "", meta: { width: 820, height: 1240 }, hasThumb: true },
  { id: 13, kind: "text", createdAt: now - 7 * min, sourceApp: "Slack", sourceExe: "slack.exe", pinned: false, size: 140, text: "Can you send the invoice for October before 4? Finance closes the books tonight.", meta: { chars: 82, lines: 1 }, hasThumb: false },
  { id: 12, kind: "text", createdAt: now - 18 * min, sourceApp: "Figma", sourceExe: "figma.exe", pinned: false, size: 16, text: "#1e2b3c", meta: { chars: 7, lines: 1 }, hasThumb: false },
  { id: 11, kind: "image", createdAt: now - 33 * min, sourceApp: "Microsoft Edge", sourceExe: "msedge.exe", pinned: false, size: 2_300_000, text: "", meta: { width: 2560, height: 1080 }, hasThumb: true },
  { id: 10, kind: "text", createdAt: now - 4 * 60 * min, sourceApp: "Visual Studio Code", sourceExe: "code.exe", pinned: false, size: 300, text: [
      "fn place(window: &WebviewWindow, settings: &Settings) {",
      "    let Some(monitor) = cursor_monitor() else { return };",
      "    let (pos, size) = window_rect(settings.window_bounds, monitor);",
      "    window.set_position(pos);",
      "}",
    ].join("\n"), meta: { chars: 190, lines: 5 }, hasThumb: false },
  { id: 1, kind: "text", createdAt: now - 9 * 24 * 60 * min, sourceApp: "Windows Terminal", sourceExe: "windowsterminal.exe", pinned: false, size: 40, text: "npm run tauri build", meta: { chars: 19, lines: 1 }, hasThumb: false },
];

let settings: Settings = {
  historyLimit: 50, maxDiskMb: 500, maxItemMb: 64,
  hotkeyDashboard: "Ctrl+Shift+V", hotkeyQueue: "Ctrl+Shift+Q", slotModifiers: "Ctrl+Alt",
  pasteOnSelect: true, restoreClipboard: false, displayMode: "fullscreen", windowBounds: null, keepDashboardWarm: false,
  reduceMotion: "system",
  captureImages: true, captureFiles: true, captureRich: true,
  blocklist: ["1password.exe", "bitwarden.exe", "keepass.exe"], encrypt: false, clearOnExit: false, autostart: true,
};

const status: Status = { paused: false, queueActive: false, queue: [], hotkeyErrors: [], itemCount: 9, diskBytes: 2_500_000, canPaste: true, glass: true, nativeCorners: true, version: "0.1.0-dev" };

export async function mockApi<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const id = args.id as number;
  const result = ((): unknown => {
    switch (cmd) {
      case "list_items": return items;
      case "get_preview":
      case "get_thumb": {
        const item = items.find((i) => i.id === id);
        const pairs: Record<number, [string, string]> = { 8: ["#ff8a5c", "#c13cff"], 4: ["#2fd6a8", "#2a6cff"], 11: ["#f4d35e", "#ee964b"], 14: ["#9bb1ff", "#3d3b8e"] };
        const [a, b] = pairs[id] ?? ["#888", "#333"];
        return swatch(a, b, item?.meta.width ?? 480, item?.meta.height ?? 300);
      }
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
