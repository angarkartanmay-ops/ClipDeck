// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

// Inline 24px stroke icons (drawn for ClipDeck; no icon font, no network).

const svg = (body: string) =>
  `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;

export const icons: Record<string, string> = {
  text: svg('<path d="M5 6h14M5 11h14M5 16h9"/>'),
  rich: svg('<path d="M7 5h6a3.5 3.5 0 0 1 0 7H7zM7 12h7a3.5 3.5 0 0 1 0 7H7z"/>'),
  image: svg('<rect x="3.5" y="4.5" width="17" height="15" rx="3"/><circle cx="9" cy="10" r="1.8"/><path d="m20.5 16-5-5-8.5 8.5"/>'),
  files: svg('<path d="M4 7.5A2.5 2.5 0 0 1 6.5 5H10l2 2h5.5A2.5 2.5 0 0 1 20 9.5v7a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 4 16.5z"/>'),
  file: svg('<path d="M7 3.5h6.5L18 8v11.5a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1v-15a1 1 0 0 1 1-1z"/><path d="M13 3.5V8h5"/>'),
  pin: svg('<path d="M9 4h6l-1 5 3 3v2H7v-2l3-3zM12 14v6"/>'),
  pinned: '<svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor" aria-hidden="true"><path d="M9 3.5h6a.6.6 0 0 1 .6.7L14.7 9l3 3.1a1 1 0 0 1 .3.7V14a.5.5 0 0 1-.5.5H13v5.2a1 1 0 0 1-2 0v-5.2H6.5A.5.5 0 0 1 6 14v-1.2a1 1 0 0 1 .3-.7L9.3 9l-.9-4.8A.6.6 0 0 1 9 3.5z"/></svg>',
  trash: svg('<path d="M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12M10.5 11v5M13.5 11v5"/>'),
  gear: svg('<circle cx="12" cy="12" r="3"/><path d="M12 3v2.2M12 18.8V21M4.2 7.5l1.9 1.1M17.9 15.4l1.9 1.1M4.2 16.5l1.9-1.1M17.9 8.6l1.9-1.1"/><circle cx="12" cy="12" r="7"/>'),
  search: svg('<circle cx="11" cy="11" r="6.5"/><path d="m20 20-4.2-4.2"/>'),
  queue: svg('<path d="M4 6h11M4 11h11M4 16h7M18 13v7M14.5 16.5 18 20l3.5-3.5"/>'),
  pause: svg('<rect x="6.5" y="5" width="3.5" height="14" rx="1"/><rect x="14" y="5" width="3.5" height="14" rx="1"/>'),
  play: svg('<path d="M8 5.5v13l10-6.5z"/>'),
  close: svg('<path d="M6 6l12 12M18 6 6 18"/>'),
  copy: svg('<rect x="8" y="8" width="12" height="12" rx="2.5"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>'),
  folder: svg('<path d="M4 7.5A2.5 2.5 0 0 1 6.5 5H10l2 2h5.5A2.5 2.5 0 0 1 20 9.5v7a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 4 16.5z"/>'),
  back: svg('<path d="M15 5l-7 7 7 7"/>'),
  sparkle: svg('<path d="M12 3.5l1.8 5 5 1.8-5 1.8-1.8 5-1.8-5-5-1.8 5-1.8zM18.5 15.5l.8 2.2 2.2.8-2.2.8-.8 2.2-.8-2.2-2.2-.8 2.2-.8z"/>'),
  shield: svg('<path d="M12 3.5 19 6v5.5c0 4.2-2.9 7.6-7 9-4.1-1.4-7-4.8-7-9V6z"/><path d="m9 12 2 2 4-4"/>'),
};

/** Replaces every `[data-icon]` placeholder under `root` with its SVG. */
export function hydrateIcons(root: ParentNode = document): void {
  root.querySelectorAll<HTMLElement>("[data-icon]").forEach((el) => {
    el.innerHTML = icons[el.dataset.icon ?? ""] ?? "";
  });
}
