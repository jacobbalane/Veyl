import { expect, test } from 'vitest';
import type { LiveFrame } from '../api/types';
import {
  drawEcho,
  echoCellsFor,
  EchoHistory,
  handPoint,
  markerXs,
  motionAlpha,
  type EchoColumn,
} from './echo-view';

// 150 columns of 2 px and 60 cells of 3 px make the arithmetic easy to read.
const width = 300;
const height = 180;
const row = (values: Record<number, number>) =>
  Array.from({ length: 60 }, (_, cm) => values[cm] ?? 0);
const frame = (t: number, values: Record<number, number> = {}): LiveFrame => ({
  t,
  state: 'running',
  motion: row(values),
  hand: { present: false, distanceCm: null },
  meter: null,
});

test('opacity is the motion value over 30 dB, clamped, and ignores bad values', () => {
  expect(motionAlpha(15)).toBe(0.5);
  expect(motionAlpha(30)).toBe(1);
  expect(motionAlpha(45)).toBe(1);
  expect(motionAlpha(-3)).toBe(0);
  expect(motionAlpha(Number.NaN)).toBe(0);
  expect(motionAlpha(undefined)).toBe(0);
});

test('the newest column touches the right edge with 0 cm at the bottom and 59 cm at the top', () => {
  const columns: EchoColumn[] = [{ t: 1000, motion: row({ 0: 30, 59: 15 }) }];
  expect(echoCellsFor(columns, 1000, width, height)).toEqual([
    { x: 298, y: 177, width: 2, height: 3, alpha: 1 },
    { x: 298, y: 0, width: 2, height: 3, alpha: 0.5 },
  ]);
});

test('older columns move left by their age and leave the view after five seconds', () => {
  const columns: EchoColumn[] = [
    { t: 0, motion: row({ 10: 30 }) },
    { t: 1000, motion: row({ 10: 30 }) },
    { t: 5000, motion: row({ 10: 30 }) },
  ];
  const cells = echoCellsFor(columns, 5000, width, height);
  // At 5 s old a column sits just past the left edge; one second is 60 px.
  expect(cells.map((cell) => cell.x)).toEqual([58, 298]);
  // Sliding 1/30 s between frames moves everything one column to the left.
  expect(echoCellsFor(columns, 5000 + 1000 / 30, width, height).map((cell) => cell.x)).toEqual([
    56, 296,
  ]);
  expect(cells.every((cell) => cell.y === height - 11 * 3)).toBe(true);
});

test('quiet cells are skipped and a full window draws at most 150 by 60 cells', () => {
  expect(echoCellsFor([{ t: 0, motion: row({}) }], 0, width, height)).toEqual([]);
  const full = Array.from({ length: 150 }, (_, index) => ({
    t: (index * 1000) / 30,
    motion: row(Object.fromEntries(Array.from({ length: 60 }, (_, cm) => [cm, 20]))),
  }));
  const cells = echoCellsFor(full, (149 * 1000) / 30, width, height);
  expect(cells).toHaveLength(150 * 60);
  expect(Math.min(...cells.map((cell) => cell.x))).toBeCloseTo(0);
});

test('the hand dot and poke lines share the columns’ time axis', () => {
  expect(handPoint({ present: true, distanceCm: 30 }, 1000, 1000, width, height)).toEqual({
    x: 299,
    y: 90,
  });
  expect(handPoint({ present: true, distanceCm: 75 }, 1000, 1000, width, height)?.y).toBe(0);
  expect(handPoint({ present: false, distanceCm: null }, 1000, 1000, width, height)).toBeNull();
  expect(markerXs([1000, 4000, -2000], 5000, width)).toEqual([59, 239]);
});

test('the history keeps five seconds, restarts when the clock goes back and counts changes', () => {
  const history = new EchoHistory();
  for (let t = 0; t <= 6000; t += 100) history.push(frame(t), t);
  expect(history.columns[0].t).toBe(1000);
  history.mark(800);
  history.mark(5900);
  expect(history.markers).toEqual([5900]);
  const version = history.version;
  history.push(frame(50, { 3: 9 }), 7000);
  expect(history.columns).toHaveLength(1);
  expect(history.markers).toEqual([]);
  expect(history.version).toBeGreaterThan(version);
});

test('drawing paints cells in aqua, poke lines with their label and the hand in the text colour', () => {
  const history = new EchoHistory();
  history.push({ ...frame(1000, { 20: 30 }), hand: { present: true, distanceCm: 20 } }, 0);
  history.mark(0);
  history.mark(900);
  const calls: string[] = [];
  const context = new Proxy(
    {},
    {
      get: (_, name) =>
        name === 'measureText'
          ? () => ({ width: 20 })
          : typeof name === 'string' && /^(clearRect|fillRect|fillText|arc|stroke|fill)$/.test(name)
            ? (...args: unknown[]) => calls.push(`${name}(${args.join(',')})`)
            : () => {},
      set: (_, name, value) => {
        calls.push(`${String(name)}=${value}`);
        return true;
      },
    },
  ) as CanvasRenderingContext2D;
  drawEcho(context, history, 1000, width, height, { aqua: 'aqua', text: 'text', label: 'label' });
  expect(calls).toContain('fillStyle=aqua');
  expect(calls).toContain('fillRect(298,117,2,3)');
  expect(calls).toContain('fillStyle=label');
  expect(calls).toContain('fillText(poke,243.5,4)');
  // Near the right edge the label moves left of its line and the dot stays inside the view.
  expect(calls).toContain('textAlign=right');
  expect(calls).toContain('fillText(poke,289.5,4)');
  expect(calls).toContain('fillStyle=text');
  expect(calls).toContain(`arc(296,120,4,0,${Math.PI * 2})`);
});
