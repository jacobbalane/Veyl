import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, beforeAll, expect, test, vi } from 'vitest';
import gestureJson from '../api/fixtures/gesture.json';
import pokeTestJson from '../api/fixtures/poke-test.json';
import type { GestureEvent, PokeTestResult } from '../api/types';
import { startPokeTest, stopPokeTest } from '../api/native';
import { pokeAdvice } from '../copy';
import { PokeTestDialog, pokeTestTime } from './PokeTest';

const native = vi.hoisted(() => ({
  gestures: new Set<(gesture: GestureEvent) => void>(),
  result: { seen: 0, of: 10 } as PokeTestResult,
}));
vi.mock('../api/native', () => ({
  startPokeTest: vi.fn(async () => {}),
  stopPokeTest: vi.fn(async () => native.result),
  watchGestures: async (update: (gesture: GestureEvent) => void) => {
    native.gestures.add(update);
    return () => native.gestures.delete(update);
  },
}));
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
});
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  vi.useRealTimers();
  native.gestures.clear();
});

const gesture = gestureJson as GestureEvent;
const settle = () => act(async () => {});
const poke = (testOnly = true) =>
  act(() => native.gestures.forEach((update) => update({ ...gesture, testOnly })));

function Harness() {
  const [open, setOpen] = useState(true);
  return open ? <PokeTestDialog onClose={() => setOpen(false)} /> : <p>closed</p>;
}

test('advice follows how many of the ten pokes Veyl saw', () => {
  const advice = Array.from({ length: 11 }, (_, seen) => pokeAdvice(seen));
  expect(new Set(advice.slice(0, 6))).toEqual(
    new Set(['Try a brighter song, more volume, or Sensitivity: High.']),
  );
  expect(new Set(advice.slice(6, 9))).toEqual(new Set(['Good. Sensitivity: High may catch more.']));
  expect(new Set(advice.slice(9))).toEqual(new Set(['Great.']));
  expect(pokeAdvice((pokeTestJson as PokeTestResult).seen)).toBe(
    'Good. Sensitivity: High may catch more.',
  );
});

test('test pokes count live, and the tenth ends the test with Rust’s count', async () => {
  native.result = { seen: 10, of: 10 };
  render(<Harness />);
  await settle();
  expect(startPokeTest).toHaveBeenCalledOnce();
  const dialog = screen.getByRole('dialog', { name: 'Test pokes' });
  expect(dialog.textContent).toContain('Poke 10 times, about once a second.');
  expect(screen.getByRole('status').textContent).toBe('0 of 10');
  poke();
  poke(false);
  expect(screen.getByRole('status').textContent).toBe('1 of 10');
  for (let index = 0; index < 9; index += 1) poke();
  await settle();
  expect(stopPokeTest).toHaveBeenCalledOnce();
  expect(screen.getByRole('status').textContent).toBe('Veyl saw 10 of 10 pokes.Great.');
  fireEvent.click(screen.getByRole('button', { name: 'Close' }));
  expect(screen.getByText('closed')).toBeDefined();
  await settle();
  expect(stopPokeTest).toHaveBeenCalledOnce();
});

test('after 20 s the test ends with what Rust counted, with advice', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  native.result = { ...(pokeTestJson as PokeTestResult), seen: 4 };
  render(<Harness />);
  await settle();
  poke();
  act(() => vi.advanceTimersByTime(pokeTestTime - 1));
  expect(stopPokeTest).not.toHaveBeenCalled();
  act(() => vi.advanceTimersByTime(1));
  await settle();
  expect(stopPokeTest).toHaveBeenCalledOnce();
  expect(screen.getByText('Veyl saw 4 of 10 pokes.')).toBeDefined();
  expect(screen.getByText('Try a brighter song, more volume, or Sensitivity: High.')).toBeDefined();
});

test('Escape or Cancel closes the dialog and always stops the test', async () => {
  const view = render(<Harness />);
  await settle();
  fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }));
  expect(screen.getByText('closed')).toBeDefined();
  await settle();
  expect(stopPokeTest).toHaveBeenCalledOnce();
  view.unmount();
  render(<Harness />);
  await settle();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  await settle();
  expect(stopPokeTest).toHaveBeenCalledTimes(2);
  // Closing before the test even started still stops it.
  render(<Harness />);
  cleanup();
  await settle();
  expect(stopPokeTest).toHaveBeenCalledTimes(3);
  expect(startPokeTest).toHaveBeenCalledTimes(2);
});
