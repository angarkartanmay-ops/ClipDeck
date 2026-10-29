// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

// Justified "Mission Control" layout: tiles keep their aspect ratio, rows
// stretch to the full width, and the row height is chosen so that, when
// possible, every clip fits on screen at once.

export interface Box {
  x: number;
  y: number;
  /** Card size (the label sits below it). */
  w: number;
  h: number;
  row: number;
}

export interface LayoutResult {
  boxes: Box[];
  height: number;
}

export interface LayoutOptions {
  width: number;
  /** Space available before scrolling; used to fit everything if possible. */
  viewHeight: number;
  minRowHeight: number;
  maxRowHeight: number;
  gapX: number;
  gapY: number;
  /** Height reserved under each card for its label. */
  labelHeight: number;
}

function pack(ratios: number[], o: LayoutOptions, target: number): LayoutResult {
  const boxes: Box[] = [];
  let y = 0;
  let row = 0;
  let start = 0;
  while (start < ratios.length) {
    let end = start;
    let sum = 0;
    // Add tiles until the row at the target height overflows the width.
    while (end < ratios.length) {
      sum += ratios[end];
      end++;
      if (sum * target + o.gapX * (end - start - 1) >= o.width) break;
    }
    const n = end - start;
    const full = sum * target + o.gapX * (n - 1) >= o.width;
    // Full rows shrink to fit the width exactly; the last row keeps the
    // target height and is centred, like Mission Control.
    let h = full ? (o.width - o.gapX * (n - 1)) / sum : target;
    h = Math.min(h, o.maxRowHeight * 1.15);
    const rowWidth = sum * h + o.gapX * (n - 1);
    let x = (o.width - rowWidth) / 2;
    for (let i = start; i < end; i++) {
      const w = ratios[i] * h;
      boxes.push({ x, y, w, h, row });
      x += w + o.gapX;
    }
    y += h + o.labelHeight + o.gapY;
    row++;
    start = end;
  }
  return { boxes, height: Math.max(0, y - o.gapY) };
}

export function layout(ratios: number[], o: LayoutOptions): LayoutResult {
  if (!ratios.length || o.width <= 0) return { boxes: [], height: 0 };
  // Largest row height whose layout still fits in the view (binary search).
  let lo = o.minRowHeight;
  let hi = o.maxRowHeight;
  let best = pack(ratios, o, lo);
  if (best.height > o.viewHeight) return best; // too many: scroll at the minimum size
  for (let i = 0; i < 14; i++) {
    const mid = (lo + hi) / 2;
    const result = pack(ratios, o, mid);
    if (result.height <= o.viewHeight) {
      best = result;
      lo = mid;
    } else {
      hi = mid;
    }
  }
  return best;
}
