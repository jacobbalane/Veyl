import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import actionsJson from '../api/fixtures/actions.json';
import type { ActionsEvent } from '../api/types';
import { setActionsEnabled } from '../api/native';
import {
  ActionsNotices,
  ActionsSwitch,
  actionsReducer,
  emptyActions,
  toastTime,
  useActions,
} from './Actions';

const listeners = vi.hoisted(() => new Set<(event: ActionsEvent) => void>());
vi.mock('../api/native', () => ({
  setActionsEnabled: vi.fn(async () => {}),
  watchActions: async (update: (event: ActionsEvent) => void) => {
    listeners.add(update);
    return () => listeners.delete(update);
  },
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  vi.useRealTimers();
  listeners.clear();
});
const emit = (event: ActionsEvent) => act(() => listeners.forEach((update) => update(event)));

function Harness({ initial = false, hotkey = 'Ctrl+Alt+V' }) {
  const actions = useActions(initial);
  return (
    <>
      <ActionsSwitch actions={actions} hotkey={hotkey} ready />
      <ActionsNotices actions={actions} />
    </>
  );
}

test('a change toasts once, a repeat does not, and a block shows until actions switch', () => {
  let view = actionsReducer(emptyActions, { type: 'initial', enabled: false });
  view = actionsReducer(view, { type: 'event', event: { enabled: true, error: null } });
  expect(view).toMatchObject({ enabled: true, blocked: false, toast: { text: 'Actions on' } });
  const toast = view.toast;
  view = actionsReducer(view, { type: 'event', event: { enabled: true, error: null } });
  expect(view.toast).toBe(toast);
  view = actionsReducer(view, { type: 'event', event: actionsJson as ActionsEvent });
  expect(view).toMatchObject({ enabled: true, blocked: true });
  expect(view.toast).toBe(toast);
  view = actionsReducer(view, { type: 'event', event: { enabled: false, error: null } });
  expect(view).toMatchObject({ enabled: false, blocked: false, toast: { text: 'Actions off' } });
  // A late app state can't undo an event.
  expect(actionsReducer(view, { type: 'initial', enabled: true }).enabled).toBe(false);
  expect(actionsReducer(view, { type: 'hide-toast', id: view.toast!.id }).toast).toBeNull();
  // An event before the app state is the starting state, not a change, so it doesn't toast.
  const early = actionsReducer(emptyActions, { type: 'event', event: actionsJson as ActionsEvent });
  expect(early).toMatchObject({ enabled: true, blocked: true, toast: null });
});

test('the switch asks Rust and shows only what Rust reports, from any source', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  render(<Harness hotkey="Ctrl+Shift+F9" />);
  const button = screen.getByRole('switch', { name: 'Actions' });
  expect(button.getAttribute('aria-checked')).toBe('false');
  const tooltip = document.getElementById(button.getAttribute('aria-describedby')!);
  expect(tooltip?.textContent).toBe(
    'When actions are on, each poke clicks where your pointer is. Press Ctrl+Shift+F9 to switch them on or off.',
  );
  expect(screen.getByText('Ctrl+Shift+F9').tagName).toBe('KBD');
  fireEvent.click(button);
  expect(setActionsEnabled).toHaveBeenCalledWith(true);
  expect(button.getAttribute('aria-checked')).toBe('false');
  await act(async () => {});
  // The tray or the hotkey: an event alone switches the header and toasts.
  emit({ enabled: true, error: null });
  expect(button.getAttribute('aria-checked')).toBe('true');
  expect(screen.getByText('Actions on')).toBeDefined();
  act(() => vi.advanceTimersByTime(toastTime));
  expect(screen.queryByText('Actions on')).toBeNull();
  emit({ enabled: true, error: 'blocked_elevated' });
  expect(screen.getByRole('alert').textContent).toBe(
    'Windows blocked the click in an app running as administrator.',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Close' }));
  expect(screen.queryByRole('alert')).toBeNull();
  emit({ enabled: false, error: null });
  expect(button.getAttribute('aria-checked')).toBe('false');
  expect(screen.getByText('Actions off')).toBeDefined();
});

test('a failed request says so and leaves the switch as it was', async () => {
  vi.mocked(setActionsEnabled).mockRejectedValueOnce('failed');
  render(<Harness initial />);
  const button = screen.getByRole('switch', { name: 'Actions' });
  await waitFor(() => expect(button.getAttribute('aria-checked')).toBe('true'));
  fireEvent.click(button);
  expect(await screen.findByText("Veyl couldn't complete that request. Try again.")).toBeDefined();
  expect(button.getAttribute('aria-checked')).toBe('true');
});
