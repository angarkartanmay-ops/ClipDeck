// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

// Keyboard conventions per platform: macOS uses ⌘ where Windows and Linux
// use Ctrl, and shows modifier glyphs (⌃ ⌥ ⇧ ⌘) instead of words.

import type { Platform } from "./api";

let platform: Platform = "windows";

export function setPlatform(p: Platform): void {
  platform = p;
  document.documentElement.dataset.platform = p;
}

export const getPlatform = (): Platform => platform;
export const isMac = (): boolean => platform === "macos";

/** The platform's command modifier is held: ⌘ on macOS, Ctrl elsewhere. */
export const primary = (e: KeyboardEvent | MouseEvent): boolean => (isMac() ? e.metaKey : e.ctrlKey);

/** The settings name of the OS logo key on this platform. */
export const metaName = (): string => (isMac() ? "Cmd" : platform === "linux" ? "Super" : "Win");

const MAC_GLYPHS: Record<string, string> = {
  ctrl: "⌃",
  control: "⌃",
  alt: "⌥",
  option: "⌥",
  shift: "⇧",
  cmd: "⌘",
  command: "⌘",
  win: "⌘",
  super: "⌘",
  meta: "⌘",
};

function label(token: string): string {
  const k = token.toLowerCase();
  if (isMac()) return MAC_GLYPHS[k] ?? token;
  if (["cmd", "command", "win", "super", "meta"].includes(k)) return metaName();
  if (k === "option") return "Alt";
  return token;
}

/** A shortcut such as "Ctrl+Cmd+V" as keycaps for this platform. */
export function keycapParts(combo: string): string[] {
  return combo
    .split("+")
    .map((t) => t.trim())
    .filter(Boolean)
    .map(label);
}

/** A shortcut as inline text: "⌃⌘V" on macOS, "Ctrl+Shift+V" elsewhere. */
export function comboText(combo: string): string {
  return keycapParts(combo).join(isMac() ? "" : "+");
}

/** "⌘C" / "Ctrl+C" for an in-app shortcut on the primary modifier. */
export function primaryCombo(key: string): string {
  return isMac() ? `⌘${key}` : `Ctrl+${key}`;
}
