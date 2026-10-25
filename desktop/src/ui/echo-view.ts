import type { HandView, LiveFrame } from '../api/types';
import { liveCopy } from '../copy';

/** The echo view shows the last five seconds: 150 columns of 60 one-centimetre cells. */
export const echoWindowMs = 5000;
export const echoColumns = 150;
export const echoCells = 60;
/** Motion values are dB from 0 to 30; 30 dB draws a fully opaque cell. */
export const maxMotionDb = 30;
/** Fainter cells are skipped; they would not be visible against the panel. */
const minAlpha = 0.03;

export interface EchoColumn {
  t: number;
  motion: readonly number[];
}
export interface EchoCell {
  x: number;
  y: number;
  width: number;
  height: number;
  alpha: number;
}

/**
 * Holds what the canvas draws, outside React: live frames arrive 30 times a second and only the
 * canvas needs them. `version` changes whenever there is something new to draw.
 */
export class EchoHistory {
  columns: EchoColumn[] = [];
  markers: number[] = [];
  hand: HandView = { present: false, distanceCm: null };
  /** Time of the newest frame on the engine's clock, and when it arrived on the page's clock. */
  latest = Number.NEGATIVE_INFINITY;
  arrived = 0;
  version = 0;

  push(frame: LiveFrame, arrived: number): void {
    // A clock that runs backwards means a new run; old columns would sit in the wrong place.
    if (frame.t < this.latest) this.clear();
    this.columns.push({ t: frame.t, motion: frame.motion });
    this.hand = frame.hand;
    this.latest = frame.t;
    this.arrived = arrived;
    this.trim();
    this.version += 1;
  }
  mark(at: number): void {
    this.markers.push(at);
    this.trim();
    this.version += 1;
  }
  clear(): void {
    this.columns = [];
    this.markers = [];
    this.hand = { present: false, distanceCm: null };
    this.latest = Number.NEGATIVE_INFINITY;
    this.version += 1;
  }
  private trim(): void {
    const oldest = this.latest - echoWindowMs;
    while (this.columns.length > 0 && this.columns[0].t < oldest) this.columns.shift();
    // A stalled clock must not grow the history without bound.
    if (this.columns.length > echoColumns * 2) this.columns.splice(0, echoColumns);
    this.markers = this.markers.filter((at) => at >= oldest);
  }
}

export function motionAlpha(value: number | undefined): number {
  return value != null && Number.isFinite(value)
    ? Math.min(Math.max(value / maxMotionDb, 0), 1)
    : 0;
}
/** The horizontal centre of the column for time `t`, when the newest column ends at `width`. */
export function timeX(t: number, now: number, width: number): number {
  return width - width / echoColumns / 2 - ((now - t) / echoWindowMs) * width;
}

/**
 * Turns motion rows into canvas rectangles. The newest column touches the right edge, older
 * columns move left by their age, and each column runs from 0 cm at the bottom to 59 cm at the
 * top, with opacity value / 30. Columns are placed by time, so a missed frame leaves a gap
 * instead of shifting everything older.
 */
export function echoCellsFor(
  columns: readonly EchoColumn[],
  now: number,
  width: number,
  height: number,
): EchoCell[] {
  const columnWidth = width / echoColumns;
  const cellHeight = height / echoCells;
  const cells: EchoCell[] = [];
  for (const column of columns) {
    const x = timeX(column.t, now, width) - columnWidth / 2;
    if (x + columnWidth <= 0 || x >= width) continue;
    for (let cm = 0; cm < echoCells; cm += 1) {
      const alpha = motionAlpha(column.motion[cm]);
      if (alpha < minAlpha) continue;
      cells.push({
        x,
        y: height - (cm + 1) * cellHeight,
        width: columnWidth,
        height: cellHeight,
        alpha,
      });
    }
  }
  return cells;
}

/** The hand dot sits in the newest column at the hand's distance. */
export function handPoint(
  hand: HandView,
  latest: number,
  now: number,
  width: number,
  height: number,
): { x: number; y: number } | null {
  if (!hand.present || hand.distanceCm == null || !Number.isFinite(hand.distanceCm)) return null;
  const cm = Math.min(Math.max(hand.distanceCm, 0), echoCells);
  return { x: timeX(latest, now, width), y: height - (cm / echoCells) * height };
}

/** Horizontal positions of the poke lines still inside the view. */
export function markerXs(markers: readonly number[], now: number, width: number): number[] {
  return markers.map((at) => timeX(at, now, width)).filter((x) => x >= 0 && x <= width);
}

export interface EchoColors {
  aqua: string;
  text: string;
  /** Small accent text, darker than aqua in the light theme (decision 0003). */
  label: string;
}
/** Draws one picture of the echo view; `now` is on the engine's clock. */
export function drawEcho(
  context: CanvasRenderingContext2D,
  history: EchoHistory,
  now: number,
  width: number,
  height: number,
  colors: EchoColors,
): void {
  context.clearRect(0, 0, width, height);
  context.fillStyle = colors.aqua;
  for (const cell of echoCellsFor(history.columns, now, width, height)) {
    context.globalAlpha = cell.alpha;
    context.fillRect(cell.x, cell.y, cell.width, cell.height);
  }
  context.globalAlpha = 1;
  context.strokeStyle = colors.aqua;
  context.lineWidth = 1;
  context.font = '10px "Manrope Variable", sans-serif';
  context.textBaseline = 'top';
  for (const x of markerXs(history.markers, now, width)) {
    const line = Math.round(x) + 0.5;
    context.beginPath();
    context.moveTo(line, 0);
    context.lineTo(line, height);
    context.stroke();
    context.fillStyle = colors.label;
    // Near the right edge the label goes on the line's left, so it stays readable.
    const left = line + 4 + context.measureText(liveCopy.marker).width > width;
    context.textAlign = left ? 'right' : 'left';
    context.fillText(liveCopy.marker, left ? line - 4 : line + 4, 4);
  }
  const point = handPoint(history.hand, history.latest, now, width, height);
  if (point) {
    // The whole dot stays inside the view, even when the newest column is at the edge.
    const radius = 4;
    context.fillStyle = colors.text;
    context.beginPath();
    context.arc(Math.min(point.x, width - radius), point.y, radius, 0, Math.PI * 2);
    context.fill();
  }
}
