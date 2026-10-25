import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeAll, expect, test, vi } from 'vitest';
import engineJson from '../api/fixtures/engine.json';
import liveJson from '../api/fixtures/live-frame.json';
import gestureJson from '../api/fixtures/gesture.json';
import type { EngineView, GestureEvent, LiveFrame } from '../api/types';
import { Live, useLive } from './Live';

const listeners = vi.hoisted(() => ({
  engine: new Set<(engine: EngineView) => void>(),
  gesture: new Set<(gesture: GestureEvent) => void>(),
  frame: new Set<(frame: LiveFrame) => void>(),
}));
vi.mock('../api/native', () => ({
  browserPreview: false,
  watchEngine: async (update: (engine: EngineView) => void) => {
    listeners.engine.add(update);
    return () => listeners.engine.delete(update);
  },
  watchGestures: async (update: (gesture: GestureEvent) => void) => {
    listeners.gesture.add(update);
    return () => listeners.gesture.delete(update);
  },
  subscribeLive: async (update: (frame: LiveFrame) => void) => {
    listeners.frame.clear();
    listeners.frame.add(update);
  },
}));
beforeAll(() => {
  // jsdom has no canvas; the drawing itself is tested in echo-view.test.ts.
  HTMLCanvasElement.prototype.getContext = () => null;
});
afterEach(() => {
  cleanup();
  Object.values(listeners).forEach((set) => set.clear());
});

const engine = engineJson as EngineView;
const frame = liveJson as LiveFrame;
const gesture = gestureJson as GestureEvent;
const emit = <T,>(set: Set<(value: T) => void>, value: T) =>
  act(() => set.forEach((listener) => listener(value)));

function Harness({
  pc = false,
  canTest = false,
  onTest = () => {},
  onRecord = () => {},
}: {
  pc?: boolean;
  canTest?: boolean;
  onTest?: () => void;
  onRecord?: () => void;
}) {
  const live = useLive();
  return (
    <Live
      live={live}
      pc={pc}
      tip="Tap Start in Veyl on your phone."
      canTest={canTest}
      onTest={onTest}
      canRecord
      onRecord={onRecord}
    />
  );
}

test('before the engine reports, the Live tab says what to do in words', async () => {
  render(<Harness />);
  await waitFor(() => expect(listeners.engine.size).toBe(1));
  expect(screen.getByText('No hand seen')).toBeDefined();
  expect(screen.getByText('Move your hand above your phone to see it here.')).toBeDefined();
  expect(screen.getByText('Distance from phone')).toBeDefined();
  const status = screen.getByRole('status');
  expect(status.getAttribute('aria-live')).toBe('polite');
  expect(status.textContent).toBe('Waiting for sound…');
  expect(screen.getByText('No sound')).toBeDefined();
  expect(screen.getByText('Tap Start in Veyl on your phone.')).toBeDefined();
  expect(screen.getByText('Pokes appear here.')).toBeDefined();
  // Without a listening source there is nothing to test yet.
  expect(screen.getByRole('button', { name: 'Test pokes' }).hasAttribute('disabled')).toBe(true);
});

test('Test pokes opens the poke test once Veyl listens', async () => {
  const onTest = vi.fn();
  render(<Harness canTest onTest={onTest} />);
  await waitFor(() => expect(listeners.engine.size).toBe(1));
  fireEvent.click(screen.getByRole('button', { name: 'Test pokes' }));
  expect(onTest).toHaveBeenCalledOnce();
});

test('engine events and frames fill the meter, status, warning and hand line', async () => {
  render(<Harness />);
  await waitFor(() => expect(listeners.frame.size).toBe(1));
  emit(listeners.engine, engine);
  expect(screen.getByRole('status').textContent).toBe(
    'Learning the room…The echo moved. Veyl found it again.',
  );
  const meter = screen.getByRole('region', { name: 'Sound quality' });
  expect(within(meter).getByText('68')).toBeDefined();
  expect(within(meter).getByText('Good')).toBeDefined();
  expect(
    within(meter).getByRole('meter', { name: 'Clear echo' }).getAttribute('aria-valuenow'),
  ).toBe('63');
  expect(within(meter).getByRole('meter', { name: 'Volume' }).getAttribute('aria-valuenow')).toBe(
    '100',
  );
  expect(within(meter).getByText('Detail ≈ 1.8 cm')).toBeDefined();
  expect(
    within(meter).getByText('This track has few high sounds. A brighter song works better.'),
  ).toBeDefined();
  emit(listeners.frame, { ...frame, hand: { present: true, distanceCm: 18.4 } });
  expect(screen.getByText('Hand seen at 18 cm')).toBeDefined();
  expect(screen.queryByText('Move your hand above your phone to see it here.')).toBeNull();
  emit(listeners.frame, { ...frame, hand: { present: false, distanceCm: null } });
  expect(screen.getByText('No hand seen')).toBeDefined();
});

test('the gesture log shows the last ten pokes, marks test pokes and announces them', async () => {
  const onRecord = vi.fn();
  render(<Harness onRecord={onRecord} />);
  await waitFor(() => expect(listeners.gesture.size).toBe(1));
  for (let index = 0; index < 12; index += 1)
    emit(listeners.gesture, {
      ...gesture,
      at: 1000 + index,
      distanceCm: 10 + index,
      testOnly: index !== 11,
      acted: index === 11,
    });
  const log = screen.getByRole('list', { name: 'Recent pokes' });
  expect(log.getAttribute('aria-live')).toBe('polite');
  const items = within(log).getAllByRole('listitem');
  expect(items).toHaveLength(10);
  // The newest poke acted, so it is not marked as a test.
  expect(items[0].textContent).toBe('Poke · 21 cm');
  expect(items[1].textContent).toBe('Poke · 20 cmtest');
  expect(items[9].textContent).toBe('Poke · 12 cmtest');
  expect(items[0].getAttribute('data-acted')).toBe('true');
  expect(items[1].getAttribute('data-acted')).toBe('false');
  fireEvent.click(screen.getByRole('button', { name: 'Record a test' }));
  expect(onRecord).toHaveBeenCalledOnce();
});

test('PC mode names the microphone in the axis, empty state and warnings', async () => {
  render(<Harness pc />);
  await waitFor(() => expect(listeners.engine.size).toBe(1));
  expect(screen.getByText('Distance from microphone')).toBeDefined();
  expect(
    screen.getByText('Move your hand in front of your microphone to see it here.'),
  ).toBeDefined();
  emit(listeners.engine, {
    ...engine,
    suspended: null,
    warning: 'impulsive_noise',
    meter: { ...engine.meter!, tip: 'ready' },
  });
  expect(screen.getByRole('status').textContent).toBe(
    'Watching for pokesSudden noises near the microphone make the echo hard to read.',
  );
  expect(screen.getByText('Ready. Poke toward your microphone.')).toBeDefined();
});
