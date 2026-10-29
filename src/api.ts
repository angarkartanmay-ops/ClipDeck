// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { mockApi } from "./mock";

export type Kind = "text" | "rich" | "image" | "files";

export interface Item {
  id: number;
  kind: Kind;
  createdAt: number;
  sourceApp: string;
  sourceExe: string;
  pinned: boolean;
  size: number;
  text: string;
  meta: {
    width?: number;
    height?: number;
    files?: string[];
    fileCount?: number;
    chars?: number;
    lines?: number;
    formats?: string[];
  };
  hasThumb: boolean;
}

export interface Settings {
  historyLimit: number;
  maxDiskMb: number;
  maxItemMb: number;
  hotkeyDashboard: string;
  hotkeyQueue: string;
  slotModifiers: string;
  pasteOnSelect: boolean;
  restoreClipboard: boolean;
  position: "cursor" | "center";
  keepDashboardWarm: boolean;
  theme: "system" | "light" | "dark";
  accent: string;
  reduceMotion: "system" | "on" | "off";
  captureImages: boolean;
  captureFiles: boolean;
  captureRich: boolean;
  blocklist: string[];
  encrypt: boolean;
  clearOnExit: boolean;
  autostart: boolean;
}

export interface Status {
  paused: boolean;
  queueActive: boolean;
  queue: number[];
  hotkeyErrors: string[];
  itemCount: number;
  diskBytes: number;
  canPaste: boolean;
  glass: boolean;
  nativeCorners: boolean;
  version: string;
}

export const inTauri = "__TAURI_INTERNALS__" in window;

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T> =>
  inTauri ? invoke<T>(cmd, args) : mockApi<T>(cmd, args);

export const api = {
  listItems: () => call<Item[]>("list_items"),
  getThumb: (id: number) => call<string | null>("get_thumb", { id }),
  paste: (id: number, plain = false) => call<void>("paste_item", { id, plain }),
  copy: (id: number) => call<void>("copy_item", { id }),
  setPinned: (id: number, pinned: boolean) => call<void>("set_pinned", { id, pinned }),
  remove: (id: number) => call<void>("delete_item", { id }),
  clearHistory: (keepPinned: boolean) => call<void>("clear_history", { keepPinned }),
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  getStatus: () => call<Status>("get_status"),
  setPaused: (paused: boolean) => call<void>("set_paused", { paused }),
  setQueueActive: (active: boolean) => call<void>("set_queue_active", { active }),
  toggleQueued: (id: number) => call<void>("toggle_queued", { id }),
  ready: () => call<void>("dashboard_ready"),
  close: () => call<void>("close_dashboard"),
  openDataDir: () => call<void>("open_data_dir"),
};

export function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(() => {});
  return listen<T>(event, (e) => handler(e.payload));
}
