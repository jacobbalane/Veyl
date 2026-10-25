import { afterEach, expect, test, vi } from 'vitest';
import type { LiveFrame, Meter } from '../api/types';
import { engineStatus, tipText, warningText } from '../copy';
import {
  previewChoices,
  previewDiagnostics,
  previewEngine,
  previewFrame,
  previewLink,
  previewOptions,
  previewPokes,
  previewSource,
  subscribeLive,
} from './fake-native';

const defaults = previewOptions('');
afterEach(() => {
  vi.useRealTimers();
});

function expectValidMeter(meter: Meter) {
  expect(Number.isInteger(meter.score) && meter.score >= 0 && meter.score <= 100).toBe(true);
  for (const value of [meter.high, meter.detail, meter.volume, meter.steady, meter.clarity])
    expect(value >= 0 && value <= 1).toBe(true);
  expect(['great', 'good', 'fair', 'poor', 'none']).toContain(meter.label);
  expect(previewChoices.tip).toContain(meter.tip);
}
function expectValidFrame(frame: LiveFrame) {
  expect(frame.motion).toHaveLength(60);
  expect(frame.motion.every((value) => Number.isFinite(value) && value >= 0 && value <= 30)).toBe(
    true,
  );
  expect(previewChoices.state).toContain(frame.state);
  if (frame.hand.present) expect(frame.hand.distanceCm).toBeGreaterThan(0);
  else expect(frame.hand.distanceCm).toBeNull();
  if (frame.meter) expectValidMeter(frame.meter);
}

test('the walk-through connects, starts the engine and then sees a hovering hand', () => {
  expect(previewLink(0, defaults).connection.state).toBe('connecting');
  expect(previewEngine(0, defaults)).toMatchObject({ state: 'no_audio', meter: null });
  expect(previewLink(1500, defaults).connection.capture?.state).toBe('running');
  expect(previewEngine(1500, defaults)).toMatchObject({ state: 'aligning', suspended: 'starting' });
  expect(previewEngine(2500, defaults).state).toBe('settling');
  const running = previewEngine(5000, defaults);
  expect(running).toMatchObject({ state: 'running', suspended: null, warning: null });
  expect(running.hand.distanceCm).toBeGreaterThanOrEqual(17);
  expect(running.hand.distanceCm).toBeLessThanOrEqual(23);
  expect(running.meter?.tip).toBe('ready');
});

test('frames are valid for a minute of preview and carry the meter in every third frame', () => {
  const frames = Array.from({ length: 1800 }, (_, index) =>
    previewFrame((index * 1000) / 30, index, defaults),
  );
  frames.forEach(expectValidFrame);
  // The meter starts once the phone listens, after the first second.
  expect(frames.every((frame, index) => !frame.meter || index % 3 === 0)).toBe(true);
  expect(frames.filter((frame) => frame.meter).length).toBe(590);
  // The same time and options give the same frame.
  expect(previewFrame(12345, 3, defaults)).toEqual(previewFrame(12345, 3, defaults));
  // The hand's echo is brightest near its distance.
  const frame = previewFrame(4200, 0, defaults);
  const brightest = frame.motion.indexOf(Math.max(...frame.motion));
  expect(Math.abs(brightest + 0.5 - frame.hand.distanceCm!)).toBeLessThan(1.5);
});

test('a poke is detected every 3 s while the engine runs, nearer than the hovering hand', () => {
  const pokes = previewPokes(0, 30000, defaults);
  expect(pokes.map((poke) => poke.at)).toEqual([
    4680, 7680, 10680, 13680, 16680, 19680, 22680, 25680, 28680,
  ]);
  expect(pokes.every((poke) => poke.testOnly && !poke.acted && poke.distanceCm < 17.1)).toBe(true);
  expect(previewPokes(4680, 7680, defaults).map((poke) => poke.at)).toEqual([7680]);
  expect(previewPokes(0, 30000, previewOptions('?hand=none'))).toEqual([]);
  expect(previewPokes(0, 30000, previewOptions('?suspended=paused'))).toEqual([]);
});

test('URL options pick every engine state, suspension, warning and tip', () => {
  for (const state of previewChoices.state) {
    const view = previewEngine(500, previewOptions(`?state=${state}`));
    expect(view.state).toBe(state);
    expect(engineStatus(view)).not.toBe('');
  }
  for (const suspended of previewChoices.suspended)
    expect(previewEngine(500, previewOptions(`?suspended=${suspended}`)).suspended).toBe(suspended);
  for (const warning of previewChoices.warning) {
    const view = previewEngine(500, previewOptions(`?warning=${warning}`));
    expect(view.warning).toBe(warning);
    expect(warningText(view.warning, false)).not.toBe('');
  }
  for (const tip of previewChoices.tip) {
    const options = previewOptions(`?tip=${tip}&mode=pc`);
    const meter = previewEngine(5000, options).meter!;
    expectValidMeter(meter);
    expect(meter.tip).toBe(tip);
    expect(tipText(meter.tip, true)).not.toBe('');
  }
  expect(previewEngine(5000, previewOptions('?tip=capture_off')).meter).toMatchObject({
    score: 0,
    label: 'none',
  });
  expect(previewEngine(5000, previewOptions('?score=40')).meter).toMatchObject({
    score: 40,
    label: 'fair',
  });
  expect(previewOptions('?state=flying&tip=nope&score=x')).toEqual(defaults);
});

test('connection and PC options set the link, the source and diagnostics', () => {
  for (const connection of previewChoices.connection)
    expect(previewLink(0, previewOptions(`?connection=${connection}`)).connection).toBeDefined();
  expect(previewLink(0, previewOptions('?connection=nearby')).phones.nearby).toHaveLength(1);
  expect(previewLink(0, previewOptions('?connection=pairing')).pairing?.stage).toBe('compare');
  expect(previewLink(0, previewOptions('?connection=unreachable')).connection.state).toBe(
    'unreachable',
  );
  const pc = previewOptions('?mode=pc');
  expect(previewSource(0, pc)).toMatchObject({ mode: 'pc', state: 'starting', choiceDone: true });
  expect(previewSource(5000, pc).state).toBe('running');
  expect(previewDiagnostics(500, defaults).levels.latency.medianMs).toBeNull();
  expect(previewDiagnostics(20000, defaults).levels.latency).toEqual({
    pokes: 6,
    medianMs: 148,
    p95Ms: 212,
  });
});

test('live subscriptions deliver about 30 frames a second, and a new one replaces the old', async () => {
  // Only the timers are fake; the preview's clock keeps real time.
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
  const first = vi.fn();
  const second = vi.fn();
  await subscribeLive(first);
  vi.advanceTimersByTime(1000);
  expect(first.mock.calls.length).toBeGreaterThanOrEqual(29);
  expect(first.mock.calls.length).toBeLessThanOrEqual(31);
  await subscribeLive(second);
  const before = first.mock.calls.length;
  vi.advanceTimersByTime(1000);
  expect(first.mock.calls.length).toBe(before);
  expect(second.mock.calls.length).toBeGreaterThanOrEqual(29);
  second.mock.calls.forEach(([frame]) => expectValidFrame(frame as LiveFrame));
  await subscribeLive(() => {});
});
