// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

// Inline 24px stroke icons drawn for ClipDeck (no icon font, no network).
// One stroke weight everywhere: 1.6.

const svg = (body: string, size = 16) =>
  `<svg viewBox="0 0 24 24" width="${size}" height="${size}" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;

export const icons: Record<string, string> = {
  text: svg('<path d="M5 6.5h14M5 11.5h14M5 16.5h8.5"/>'),
  rich: svg('<path d="M4.5 18 9 6h1l4.5 12M6.3 14h6.4M16 18v-5.2a2.3 2.3 0 0 1 4.5 0V18M16 15.2h4.5"/>'),
  code: svg('<path d="m8.5 7.5-4.5 4.5 4.5 4.5M15.5 7.5l4.5 4.5-4.5 4.5M13.4 5.5l-2.8 13"/>'),
  link: svg('<path d="M10 14a4.2 4.2 0 0 0 6 0l2.6-2.6a4.2 4.2 0 0 0-6-6L11.4 6.6"/><path d="M14 10a4.2 4.2 0 0 0-6 0l-2.6 2.6a4.2 4.2 0 0 0 6 6l1.2-1.2"/>'),
  image: svg('<rect x="3.5" y="4.5" width="17" height="15" rx="3.2"/><circle cx="9" cy="10" r="1.7"/><path d="m20.5 15.5-4.6-4.6L7.4 19.5"/>'),
  files: svg('<path d="M3.8 7.6A2.6 2.6 0 0 1 6.4 5h3.4l2 2.1h5.8a2.6 2.6 0 0 1 2.6 2.6v6.7a2.6 2.6 0 0 1-2.6 2.6H6.4a2.6 2.6 0 0 1-2.6-2.6z"/>'),
  file: svg('<path d="M7 3.5h6.5L18 8v11.5a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1v-15a1 1 0 0 1 1-1z"/><path d="M13 3.5V8h5"/>'),
  color: svg('<path d="M12 3.5a8.5 8.5 0 1 0 0 17c1 0 1.6-.8 1.6-1.6 0-1.4-1.1-1.6-1.1-2.6 0-.8.6-1.3 1.4-1.3h2a4.6 4.6 0 0 0 4.6-4.6c0-3.8-3.8-6.9-8.5-6.9z"/><circle cx="7.8" cy="11" r="1"/><circle cx="10.5" cy="7.6" r="1"/><circle cx="15" cy="8.2" r="1"/>'),
  pin: svg('<path d="M9 4h6l-1 5 3 3v2H7v-2l3-3zM12 14v6"/>'),
  pinned: '<svg viewBox="0 0 24 24" width="12" height="12" fill="currentColor" aria-hidden="true"><path d="M9 3.5h6a.6.6 0 0 1 .6.7L14.7 9l3 3.1a1 1 0 0 1 .3.7V14a.5.5 0 0 1-.5.5H13v5.2a1 1 0 0 1-2 0v-5.2H6.5A.5.5 0 0 1 6 14v-1.2a1 1 0 0 1 .3-.7L9.3 9l-.9-4.8A.6.6 0 0 1 9 3.5z"/></svg>',
  trash: svg('<path d="M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12M10.5 11v5M13.5 11v5"/>'),
  gear: svg('<path d="M10.3 3.8h3.4l.5 2.3 1.6.9 2.2-.8 1.7 2.9-1.7 1.6v1.8l1.7 1.6-1.7 2.9-2.2-.8-1.6.9-.5 2.3h-3.4l-.5-2.3-1.6-.9-2.2.8-1.7-2.9 1.7-1.6v-1.8L4.3 9.1 6 6.2l2.2.8 1.6-.9z"/><circle cx="12" cy="12" r="2.6"/>'),
  search: svg('<circle cx="11" cy="11" r="6.2"/><path d="m19.5 19.5-3.9-3.9"/>'),
  queue: svg('<path d="M4 6.5h11M4 11.5h11M4 16.5h7M18.5 12.5v7M15.5 16.5l3 3 3-3"/>'),
  pause: svg('<path d="M9 6v12M15 6v12"/>'),
  play: svg('<path d="M8 5.8v12.4a.8.8 0 0 0 1.2.7l9.6-6.2a.8.8 0 0 0 0-1.4L9.2 5.1A.8.8 0 0 0 8 5.8z"/>'),
  copy: svg('<rect x="8.5" y="8.5" width="11" height="11" rx="2.4"/><path d="M15.5 8.5V6.9a2.4 2.4 0 0 0-2.4-2.4H6.9a2.4 2.4 0 0 0-2.4 2.4v6.2a2.4 2.4 0 0 0 2.4 2.4h1.6"/>'),
  paste: svg('<path d="M9 4.5h6v2.5H9z"/><path d="M15 5.5h1.6A1.9 1.9 0 0 1 18.5 7.4v11.2a1.9 1.9 0 0 1-1.9 1.9H7.4a1.9 1.9 0 0 1-1.9-1.9V7.4a1.9 1.9 0 0 1 1.9-1.9H9"/><path d="M9.5 13.5l2 2 3.5-3.5"/>'),
  plain: svg('<path d="M6 6.5h12M12 6.5v12M4 20l16-16" opacity=".55"/><path d="M6 6.5h12M12 6.5v12"/>'),
  folder: svg('<path d="M3.8 7.6A2.6 2.6 0 0 1 6.4 5h3.4l2 2.1h5.8a2.6 2.6 0 0 1 2.6 2.6v6.7a2.6 2.6 0 0 1-2.6 2.6H6.4a2.6 2.6 0 0 1-2.6-2.6z"/>'),
  back: svg('<path d="M14.5 6 8.5 12l6 6"/>'),
  close: svg('<path d="M7 7l10 10M17 7 7 17"/>'),
  maximize: svg('<path d="M4.5 9V4.5H9M15 4.5h4.5V9M19.5 15v4.5H15M9 19.5H4.5V15"/>'),
  windowed: svg('<rect x="4" y="6.5" width="16" height="11" rx="2.4"/><path d="M4 10h16"/>'),
  eye: svg('<path d="M2.8 12s3.4-6.2 9.2-6.2 9.2 6.2 9.2 6.2-3.4 6.2-9.2 6.2S2.8 12 2.8 12z"/><circle cx="12" cy="12" r="2.6"/>'),
  display: svg('<rect x="3.5" y="4.5" width="17" height="11.5" rx="2.2"/><path d="M9 19.5h6M12 16v3.5"/>'),
  keyboard: svg('<rect x="3" y="6" width="18" height="12" rx="2.6"/><path d="M7 10h.01M10.3 10h.01M13.7 10h.01M17 10h.01M8 14h8"/>'),
  history: svg('<path d="M4.5 12a7.5 7.5 0 1 0 2.2-5.3L4.5 9"/><path d="M4.5 4.5V9H9M12 8v4.2l2.8 1.8"/>'),
  shield: svg('<path d="M12 3.5 19 6v5.5c0 4.2-2.9 7.6-7 9-4.1-1.4-7-4.8-7-9V6z"/><path d="m9 12 2 2 4-4"/>'),
  palette: svg('<path d="M12 3.5a8.5 8.5 0 1 0 0 17c1 0 1.6-.8 1.6-1.6 0-1.4-1.1-1.6-1.1-2.6 0-.8.6-1.3 1.4-1.3h2a4.6 4.6 0 0 0 4.6-4.6c0-3.8-3.8-6.9-8.5-6.9z"/><circle cx="7.8" cy="11" r="1"/><circle cx="10.5" cy="7.6" r="1"/><circle cx="15" cy="8.2" r="1"/>'),
  info: svg('<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5M12 8h.01"/>'),
  sliders: svg('<path d="M5 7h9M18 7h1M5 17h3M12 17h7"/><circle cx="16" cy="7" r="2"/><circle cx="10" cy="17" r="2"/>'),
  alert: svg('<path d="M12 4.5 20.5 19h-17z"/><path d="M12 10v4M12 16.5h.01"/>'),
  app: svg('<rect x="4" y="4" width="16" height="16" rx="4"/><path d="M4 9h16"/>'),
  clock: svg('<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>'),
  weight: svg('<path d="M6.5 9.5h11l1.5 10H5z"/><circle cx="12" cy="6.5" r="2.2"/>'),
};

/** Replaces every `[data-icon]` placeholder under `root` with its SVG. */
export function hydrateIcons(root: ParentNode = document): void {
  root.querySelectorAll<HTMLElement>("[data-icon]").forEach((el) => {
    el.innerHTML = icons[el.dataset.icon ?? ""] ?? "";
  });
}

/** Empty-state illustration: a small fanned deck of cards. */
/** Brand glyph: a small stack of cards. */
export const mark = `<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"><rect x="7" y="3.5" width="12" height="15" rx="2.6" opacity=".45"/><rect x="5" y="5.5" width="12" height="15" rx="2.6" fill="currentColor" fill-opacity=".14"/></svg>`;

export const deckArt = `<svg viewBox="0 0 120 92" width="120" height="92" aria-hidden="true">
  <g fill="none" stroke-width="1.4">
    <rect x="34" y="12" width="56" height="70" rx="9" transform="rotate(10 62 47)" class="art-back"/>
    <rect x="30" y="12" width="56" height="70" rx="9" transform="rotate(-8 58 47)" class="art-mid"/>
    <rect x="32" y="10" width="56" height="70" rx="9" class="art-front"/>
    <rect x="49" y="5" width="22" height="10" rx="4" class="art-clip"/>
    <path d="M42 30h30M42 40h24M42 50h28M42 60h16" class="art-lines"/>
  </g>
</svg>`;
