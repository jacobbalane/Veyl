import { expect, test } from 'vitest';
import engineJson from '../api/fixtures/engine.json';
import liveJson from '../api/fixtures/live-frame.json';
import gestureJson from '../api/fixtures/gesture.json';
import { emptyEngine, type EngineView, type GestureEvent, type LiveFrame } from '../api/types';
import { engineStatus, warningText } from '../copy';
import {
  emptyLive,
  formatLatency,
  gestureLimit,
  gestureLine,
  handLine,
  liveReducer,
  meterDetail,
  meterLabel,
  meterPercent,
} from './live-model';

const engine = engineJson as EngineView;
const frame = liveJson as LiveFrame;
const gesture = gestureJson as GestureEvent;

test('meter values show as whole numbers, labels and the detail in one decimal', () => {
  expect(meterPercent(0.625)).toBe(63);
  expect(meterPercent(1)).toBe(100);
  expect(meterPercent(-0.2)).toBe(0);
  expect(meterPercent(1.4)).toBe(100);
  expect(meterPercent(Number.NaN)).toBe(0);
  expect(meterLabel(engine.meter)).toBe('Good');
  expect(meterLabel(null)).toBe('No sound');
  expect(meterLabel({ ...engine.meter!, label: 'great' })).toBe('Great');
  expect(meterLabel({ ...engine.meter!, label: 'poor' })).toBe('Poor');
  expect(meterDetail(engine.meter)).toBe('Detail ≈ 1.8 cm');
  expect(meterDetail({ ...engine.meter!, detailCm: null })).toBe('');
  expect(meterDetail(null)).toBe('');
});

test('the hand line and gesture log round to whole centimetres', () => {
  expect(handLine({ present: true, distanceCm: 18.4 })).toBe('Hand seen at 18 cm');
  expect(handLine({ present: false, distanceCm: null })).toBe('No hand seen');
  expect(handLine({ present: true, distanceCm: null })).toBe('No hand seen');
  expect(gestureLine(gesture)).toBe('Poke · 15 cm');
  expect(gestureLine({ ...gesture, distanceCm: 14.2 })).toBe('Poke · 14 cm');
});

test('every engine state, suspension and warning has words', () => {
  const states = ['no_audio', 'waiting_for_sound', 'aligning', 'settling', 'running'] as const;
  expect(states.map((state) => engineStatus({ state, suspended: null }))).toEqual([
    'Waiting for sound…',
    'Waiting for sound…',
    'Finding the echo…',
    'Learning the room…',
    'Watching for pokes',
  ]);
  // A suspension reason says more than the state, except while the engine is still starting.
  expect(engineStatus({ state: 'running', suspended: 'paused' })).toBe('Paused');
  expect(engineStatus({ state: 'aligning', suspended: 'source' })).toBe(
    'Waiting for sound Veyl can use',
  );
  expect(engineStatus({ state: 'running', suspended: 'sound' })).toBe(
    'Sound too weak to detect pokes',
  );
  expect(engineStatus({ state: 'running', suspended: 'settling' })).toBe('Learning the room…');
  expect(engineStatus({ state: 'aligning', suspended: 'starting' })).toBe('Finding the echo…');
  for (const warning of ['relocked', 'model_jump', 'impulsive_noise', 'drift'] as const)
    for (const pc of [false, true]) expect(warningText(warning, pc)).not.toBe('');
  expect(warningText('impulsive_noise', true)).toBe(
    'Sudden noises near the microphone make the echo hard to read.',
  );
  expect(warningText(null, false)).toBe('');
});

test('engine events win over the app state and frames add the hand and the meter', () => {
  let view = liveReducer(emptyLive, { type: 'engine', engine });
  expect(view.engine.suspended).toBe('settling');
  // A slower app state snapshot cannot replace a newer engine event.
  view = liveReducer(view, { type: 'initial', engine: emptyEngine });
  expect(view.engine.state).toBe('running');
  const moved = liveReducer(view, {
    type: 'frame',
    frame: { ...frame, hand: { present: true, distanceCm: 12.2 }, meter: null },
  });
  expect(moved.engine.hand.distanceCm).toBe(12.2);
  // A frame without a meter keeps the last meter, and suspension stays the engine event's.
  expect(moved.engine.meter).toBe(view.engine.meter);
  expect(moved.engine.suspended).toBe('settling');
  const scored = liveReducer(moved, {
    type: 'frame',
    frame: { ...frame, hand: moved.engine.hand, meter: { ...frame.meter!, score: 71 } },
  });
  expect(scored.engine.meter?.score).toBe(71);
});

test('frames that change nothing shown keep the same view, so they do not render', () => {
  const view = liveReducer(emptyLive, { type: 'frame', frame });
  expect(view.received).toBe(true);
  const same = liveReducer(view, {
    type: 'frame',
    frame: {
      ...frame,
      t: frame.t + 33,
      motion: frame.motion.map((value) => value + 1),
      hand: { present: true, distanceCm: 18.6 },
      meter: { ...frame.meter!, clarity: 0.6251 },
    },
  });
  expect(same).toBe(view);
  expect(liveReducer(emptyLive, { type: 'initial', engine }).engine).toBe(engine);
});

test('the gesture log keeps the newest ten pokes first', () => {
  let view = emptyLive;
  for (let index = 0; index < 13; index += 1)
    view = liveReducer(view, { type: 'gesture', gesture: { ...gesture, at: index } });
  expect(view.gestures).toHaveLength(gestureLimit);
  expect(gestureLimit).toBe(10);
  expect(view.gestures.map((item) => item.at)).toEqual([12, 11, 10, 9, 8, 7, 6, 5, 4, 3]);
});

test('poke delay shows median and p95, or a dash before any poke', () => {
  expect(formatLatency({ pokes: 0, medianMs: null, p95Ms: null })).toBe('—');
  expect(formatLatency({ pokes: 12, medianMs: 147.6, p95Ms: 212.2 })).toBe('148 / 212 ms');
});
