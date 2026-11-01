// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
  displayMode: "fullscreen" | "window";
  windowBounds: { x: number; y: number; width: number; height: number } | null;
  keepDashboardWarm: boolean;
  reduceMotion: "system" | "on" | "off";
  captureImages: boolean;
  captureFiles: boolean;
  captureRich: boolean;
  blocklist: string[];
  encrypt: boolean;
  retentionDays: number;
  clearOnExit: boolean;
  autostart: boolean;
}

export type Platform = "windows" | "macos" | "linux";

export interface Notice {
  id: string;
  text: string;
  action: string | null;
  actionLabel: string | null;
}

export interface Status {
  platform: Platform;
  paused: boolean;
  queueActive: boolean;
  queue: number[];
  hotkeyErrors: string[];
  itemCount: number;
  diskBytes: number;
  canPaste: boolean;
  canAutoPaste: boolean;
  encryptionSupported: boolean;
  glass: boolean;
  /** Settings fixed by the organization's policy. */
  managed: string[];
  notices: Notice[];
  version: string;
}

export const inTauri = "__TAURI_INTERNALS__" in window;

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T> =>
  inTauri ? invoke<T>(cmd, args) : mockApi<T>(cmd, args);

export const api = {
  listItems: () => call<Item[]>("list_items"),
  getThumb: (id: number) => call<string | null>("get_thumb", { id }),
  getPreview: (id: number) => call<string | null>("get_preview", { id }),
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
  openSystemSettings: (kind: string) => call<void>("open_system_settings", { kind }),
  /** Starts moving the window with the mouse (window mode). */
  startDragging: () => (inTauri ? getCurrentWindow().startDragging() : Promise.resolve()),
};

export function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(() => {});
  return listen<T>(event, (e) => handler(e.payload));
}
