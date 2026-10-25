import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { defaultSettings, type SettingsPatch, type SettingsView } from '../api/types';
import { setSettings } from '../api/native';
import { SettingsPanel, useSettings } from './Settings';
import { useSources } from './Sources';

const saved = vi.hoisted(() => ({ value: {} as SettingsView, refuse: new Map<string, string>() }));
vi.mock('../api/native', () => ({
  browserPreview: false,
  // Like Rust: a refused field rejects with its code, anything else is saved and returned.
  setSettings: vi.fn(async (patch: SettingsPatch) => {
    for (const field of Object.keys(patch))
      if (saved.refuse.has(field)) throw saved.refuse.get(field);
    saved.value = { ...saved.value, ...patch };
    return saved.value;
  }),
  watchSource: async () => () => {},
  setSource: vi.fn(async () => {}),
  setPcDevices: vi.fn(async () => {}),
  checkPcSound: vi.fn(async () => {}),
  cancelPcSoundCheck: vi.fn(async () => {}),
}));
beforeEach(() => {
  saved.value = { ...defaultSettings, pcName: 'Test PC' };
  saved.refuse.clear();
});
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

function Harness() {
  const settings = useSettings(saved.value);
  const sources = useSources();
  return <SettingsPanel settings={settings} sources={sources} />;
}
const lastPatch = () => vi.mocked(setSettings).mock.lastCall?.[0];

test('each choice and switch sends only its own change', async () => {
  render(<Harness />);
  const changes: [string, string, SettingsPatch][] = [
    ['Poke action', 'play_pause', { pokeAction: 'play_pause' }],
    ['Sensitivity', 'high', { sensitivity: 'high' }],
    ['Distance range', 'far', { range: 'far' }],
  ];
  for (const [label, value, patch] of changes) {
    fireEvent.change(screen.getByLabelText(label), { target: { value } });
    await waitFor(() => expect(lastPatch()).toEqual(patch));
    expect((screen.getByLabelText(label) as HTMLSelectElement).value).toBe(value);
  }
  const switches: [string, SettingsPatch][] = [
    ['Turn actions on when Veyl starts', { actionsOnAtStart: true }],
    ['Start Veyl with Windows', { startWithWindows: true }],
    ['Keep Veyl running in the tray when I close the window', { closeToTray: false }],
    ['Detailed logs', { detailedLogs: true }],
    // On means 7 days; off keeps recordings until they are deleted.
    ['Keep test recordings for 7 days', { keepRecordingsDays: 0 }],
  ];
  for (const [name, patch] of switches) {
    const control = screen.getByRole('switch', { name });
    const before = (control as HTMLInputElement).checked;
    fireEvent.click(control);
    await waitFor(() => expect(lastPatch()).toEqual(patch));
    await waitFor(() => expect((control as HTMLInputElement).checked).toBe(!before));
  }
  fireEvent.click(screen.getByRole('switch', { name: 'Keep test recordings for 7 days' }));
  await waitFor(() => expect(lastPatch()).toEqual({ keepRecordingsDays: 7 }));
  expect(setSettings).toHaveBeenCalledTimes(9);
});

test('detailed logs say they start next time, and names and temperatures save once typed', async () => {
  render(<Harness />);
  const logs = screen.getByRole('switch', { name: 'Detailed logs' });
  const note = document.getElementById(logs.getAttribute('aria-describedby')!);
  expect(note?.textContent).toBe('Detailed logs start the next time Veyl starts.');
  const name = screen.getByLabelText('PC name');
  fireEvent.change(name, { target: { value: 'Desk PC' } });
  expect(setSettings).not.toHaveBeenCalled();
  fireEvent.keyDown(name, { key: 'Enter' });
  fireEvent.blur(name);
  await waitFor(() => expect(lastPatch()).toEqual({ pcName: 'Desk PC' }));
  expect(setSettings).toHaveBeenCalledOnce();
  const temperature = screen.getByLabelText('Room temperature');
  fireEvent.change(temperature, { target: { value: '26.5' } });
  fireEvent.blur(temperature);
  await waitFor(() => expect(lastPatch()).toEqual({ temperatureC: 26.5 }));
  // An empty field would reach Rust as "no change", so it is refused here.
  fireEvent.change(temperature, { target: { value: '' } });
  fireEvent.blur(temperature);
  expect(await screen.findByText('Enter a room temperature from -10 to 45 °C.')).toBeDefined();
  expect(setSettings).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(temperature, { key: 'Escape' });
  expect((temperature as HTMLInputElement).value).toBe('26.5');
  expect(screen.queryByText('Enter a room temperature from -10 to 45 °C.')).toBeNull();
});

test('refused changes say why in plain words and show the saved value again', async () => {
  saved.refuse.set('pcName', 'invalid_pc_name');
  saved.refuse.set('startWithWindows', 'autostart_failed');
  saved.refuse.set('temperatureC', 'invalid_temperature');
  saved.refuse.set('sensitivity', 'engine_busy');
  saved.refuse.set('range', 'settings_save_failed');
  render(<Harness />);
  const name = screen.getByLabelText('PC name');
  fireEvent.change(name, { target: { value: ' ' } });
  fireEvent.blur(name);
  expect((await screen.findByRole('alert')).textContent).toBe(
    'Enter a PC name of up to 40 characters.',
  );
  // The typed name stays, so it can be fixed.
  expect((name as HTMLInputElement).value).toBe(' ');
  const windows = screen.getByRole('switch', { name: 'Start Veyl with Windows' });
  fireEvent.click(windows);
  expect(
    await screen.findByText("Windows didn't let Veyl change starting with Windows. Try again."),
  ).toBeDefined();
  expect((windows as HTMLInputElement).checked).toBe(false);
  const temperature = screen.getByLabelText('Room temperature');
  fireEvent.change(temperature, { target: { value: '80' } });
  fireEvent.blur(temperature);
  expect(await screen.findByText('Enter a room temperature from -10 to 45 °C.')).toBeDefined();
  // Rust saved the sensitivity before the engine was too busy to take it.
  fireEvent.change(screen.getByLabelText('Sensitivity'), { target: { value: 'low' } });
  expect(
    await screen.findByText(
      "Veyl saved this setting but couldn't apply it yet. Restart Veyl to apply it.",
    ),
  ).toBeDefined();
  expect((screen.getByLabelText('Sensitivity') as HTMLSelectElement).value).toBe('low');
  fireEvent.change(screen.getByLabelText('Distance range'), { target: { value: 'near' } });
  expect(await screen.findByText("Veyl couldn't save your settings. Try again.")).toBeDefined();
  expect((screen.getByLabelText('Distance range') as HTMLSelectElement).value).toBe('normal');
});

test('Change records the next key combination, and Escape keeps the old one', async () => {
  render(<Harness />);
  expect(screen.getByText('Ctrl+Alt+V')).toBeDefined();
  const change = screen.getByRole('button', { name: 'Change' });
  fireEvent.click(change);
  expect(change.textContent).toBe('Press a key combination, or Esc to cancel.');
  expect(change.getAttribute('aria-pressed')).toBe('true');
  fireEvent.keyDown(change, { code: 'KeyB', key: 'b' });
  expect(screen.getByRole('alert').textContent).toBe(
    'Use Ctrl, Alt, Shift or the Windows key together with another key.',
  );
  fireEvent.keyDown(change, { code: 'ControlLeft', key: 'Control', ctrlKey: true });
  fireEvent.keyDown(change, { code: 'Escape', key: 'Escape' });
  expect(change.textContent).toBe('Change');
  expect(screen.queryByRole('alert')).toBeNull();
  expect(setSettings).not.toHaveBeenCalled();
  fireEvent.click(change);
  fireEvent.keyDown(change, { code: 'KeyB', key: 'b', ctrlKey: true, shiftKey: true });
  await waitFor(() => expect(lastPatch()).toEqual({ hotkey: 'Ctrl+Shift+B' }));
  expect(await screen.findByText('Ctrl+Shift+B')).toBeDefined();
  expect(change.textContent).toBe('Change');
});

test('a hotkey another app holds is refused and the old one stays', async () => {
  saved.refuse.set('hotkey', 'hotkey_unavailable');
  render(<Harness />);
  const change = screen.getByRole('button', { name: 'Change' });
  fireEvent.click(change);
  fireEvent.keyDown(change, { code: 'F9', key: 'F9', altKey: true });
  await waitFor(() => expect(lastPatch()).toEqual({ hotkey: 'Alt+F9' }));
  expect(
    await screen.findByText('Another app already uses this key combination. Choose another one.'),
  ).toBeDefined();
  expect(screen.getByText('Ctrl+Alt+V')).toBeDefined();
  // Leaving the button stops recording.
  fireEvent.click(change);
  fireEvent.blur(change);
  expect(change.textContent).toBe('Change');
});
