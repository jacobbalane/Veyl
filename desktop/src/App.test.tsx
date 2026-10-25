import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeAll, expect, test, vi } from 'vitest';
import { App } from './App';
import {
  defaultSettings,
  emptyEngine,
  emptyLink,
  emptyLevels,
  emptySource,
  type ActionsEvent,
  type SettingsView,
} from './api/types';
import { setActionsEnabled } from './api/native';

const native = vi.hoisted(() => ({
  settings: {} as Partial<SettingsView>,
  actions: new Set<(event: ActionsEvent) => void>(),
}));
vi.mock('./api/native', () => ({
  browserPreview: true,
  getAppState: async () => ({
    version: '0.0.1',
    pcName: 'Test PC',
    actionsEnabled: false,
    link: structuredClone(emptyLink),
    source: { ...structuredClone(emptySource), choiceDone: true },
    engine: structuredClone(emptyEngine),
    settings: { ...defaultSettings, pcName: 'Test PC', onboardingDone: true, ...native.settings },
  }),
  previewView: async () => null,
  uiReady: async () => {},
  watchLink: async () => () => {},
  watchSource: async () => () => {},
  watchEngine: async () => () => {},
  watchGestures: async () => () => {},
  watchActions: async (update: (event: ActionsEvent) => void) => {
    native.actions.add(update);
    return () => native.actions.delete(update);
  },
  setActionsEnabled: vi.fn(async () => {}),
  setSettings: vi.fn(async () => ({ ...defaultSettings })),
  startPokeTest: vi.fn(async () => {}),
  stopPokeTest: vi.fn(async () => ({ seen: 0, of: 10 })),
  subscribeLive: async () => {},
  getDiagnostics: async () => ({
    link: structuredClone(emptyLink),
    levels: structuredClone(emptyLevels),
    source: structuredClone(emptySource),
  }),
}));
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
  HTMLCanvasElement.prototype.getContext = () => null;
});
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  native.settings = {};
  native.actions.clear();
});

test('every tab is reachable, and Help shows the version under About', async () => {
  render(<App />);
  const actions = screen.getByRole('switch', { name: 'Actions' });
  await waitFor(() => expect(actions.hasAttribute('disabled')).toBe(false));
  expect(actions.getAttribute('aria-checked')).toBe('false');
  expect(screen.queryByText(/Gesture control is coming soon/)).toBeNull();
  expect(screen.getByText('Preview — sample data')).toBeDefined();
  expect(screen.getByText('What Veyl hears')).toBeDefined();
  expect(screen.getByRole('status').textContent).toBe('Waiting for sound…');
  fireEvent.click(screen.getByRole('button', { name: 'Phones' }));
  expect(screen.getByText('Your phones')).toBeDefined();
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  expect(screen.getByDisplayValue('Test PC')).toBeDefined();
  expect(screen.getByLabelText('Poke action')).toBeDefined();
  fireEvent.click(screen.getByRole('button', { name: 'Help' }));
  expect(screen.getByText('Privacy')).toBeDefined();
  const about = screen.getByRole('region', { name: 'About' });
  expect(within(about).getByText(/Version 0\.0\.1/)).toBeDefined();
});

test('the header follows actions events from the switch, the tray and the hotkey', async () => {
  render(<App />);
  const actions = screen.getByRole('switch', { name: 'Actions' });
  await waitFor(() => expect(actions.hasAttribute('disabled')).toBe(false));
  expect(screen.getByText('Ctrl+Alt+V')).toBeDefined();
  fireEvent.click(actions);
  expect(setActionsEnabled).toHaveBeenCalledWith(true);
  // Nothing changes until Rust says so.
  expect(actions.getAttribute('aria-checked')).toBe('false');
  act(() => native.actions.forEach((update) => update({ enabled: true, error: null })));
  expect(actions.getAttribute('aria-checked')).toBe('true');
  expect(screen.getByText('Actions on')).toBeDefined();
  act(() =>
    native.actions.forEach((update) => update({ enabled: true, error: 'blocked_elevated' })),
  );
  expect(screen.getByRole('alert').textContent).toBe(
    'Windows blocked the click in an app running as administrator.',
  );
  act(() => native.actions.forEach((update) => update({ enabled: false, error: null })));
  expect(actions.getAttribute('aria-checked')).toBe('false');
  expect(screen.getByText('Actions off')).toBeDefined();
  expect(screen.queryByRole('alert')).toBeNull();
});

test('onboarding opens on first run, and closing it leaves the app usable', async () => {
  native.settings = { onboardingDone: false, hotkey: 'Ctrl+Shift+F9' };
  render(<App />);
  const dialog = await screen.findByRole('dialog', { name: 'Welcome to Veyl' });
  expect(within(dialog).getByRole('button', { name: 'Get started' })).toBeDefined();
  expect(screen.getByText('Ctrl+Shift+F9')).toBeDefined();
  fireEvent.click(within(dialog).getByRole('button', { name: 'Close' }));
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(screen.getByText('What Veyl hears')).toBeDefined();
});
