import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, beforeAll, expect, test, vi } from 'vitest';
import gestureJson from '../api/fixtures/gesture.json';
import {
  defaultSettings,
  emptyLink,
  emptySource,
  type GestureEvent,
  type LinkSnapshot,
  type SettingsPatch,
  type SourceView,
} from '../api/types';
import { pairPhone, setSettings, setSource, startPokeTest, stopPokeTest } from '../api/native';
import { EchoHistory } from './echo-view';
import { emptyLive } from './live-model';
import { Onboarding, type OnboardingStep } from './Onboarding';
import { onboardingPath, onboardingSteps } from './onboarding-steps';
import { useSettings } from './Settings';
import { useSources } from './Sources';

const native = vi.hoisted(() => ({
  gestures: new Set<(gesture: GestureEvent) => void>(),
  sources: new Set<(source: SourceView) => void>(),
  refuse: false,
}));
vi.mock('../api/native', () => ({
  browserPreview: false,
  pairPhone: vi.fn(async () => {}),
  setSource: vi.fn(async () => {}),
  setPcDevices: vi.fn(async () => {}),
  checkPcSound: vi.fn(async () => {}),
  cancelPcSoundCheck: vi.fn(async () => {}),
  watchSource: async (update: (source: SourceView) => void) => {
    native.sources.add(update);
    return () => native.sources.delete(update);
  },
  startPokeTest: vi.fn(async () => {}),
  stopPokeTest: vi.fn(async () => ({ seen: 1, of: 10 })),
  watchGestures: async (update: (gesture: GestureEvent) => void) => {
    native.gestures.add(update);
    return () => native.gestures.delete(update);
  },
  setSettings: vi.fn(async (patch: SettingsPatch) => {
    if (native.refuse) throw 'settings_save_failed';
    return { ...defaultSettings, ...patch };
  }),
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
  native.gestures.clear();
  native.sources.clear();
  native.refuse = false;
});

const live = { ...emptyLive, history: new EchoHistory() };
const source = (recommended: 'phone' | 'pc'): SourceView => ({
  ...structuredClone(emptySource),
  devices: { ...structuredClone(emptySource.devices), recommendedSource: recommended, error: null },
});
function nearby(): LinkSnapshot {
  const link = structuredClone(emptyLink);
  link.phones.nearby = [{ id: 'abcd', name: 'Desk phone', model: 'Pixel', address: '' }];
  return link;
}
function Harness({
  initialStep,
  onClose = () => {},
  recommended = 'phone',
  hotkey = 'Ctrl+Alt+V',
}: {
  initialStep?: OnboardingStep;
  onClose?: () => void;
  recommended?: 'phone' | 'pc';
  hotkey?: string;
}) {
  // The hooks take new initial values as updates, so these stay the same object.
  const [initial] = useState(() => ({
    source: source(recommended),
    settings: { ...defaultSettings, hotkey },
    link: nearby(),
  }));
  const sources = useSources(initial.source);
  const settings = useSettings(initial.settings);
  return (
    <Onboarding
      sources={sources}
      link={initial.link}
      live={live}
      settings={settings}
      initialStep={initialStep}
      onClose={onClose}
    />
  );
}
const dialog = () => screen.getByRole('dialog');
const title = () => within(dialog()).getByRole('heading').textContent;
const click = (name: string | RegExp) =>
  fireEvent.click(within(dialog()).getByRole('button', { name }));

test('both paths share the welcome, the choice, the practice and the actions steps', () => {
  const phone = onboardingPath('phone');
  const pc = onboardingPath('pc');
  expect(phone).toEqual([
    'welcome',
    'choose',
    'phone-app',
    'pair',
    'place',
    'phone-play',
    'practice',
    'actions',
  ]);
  expect(pc).toEqual(['welcome', 'choose', 'pc-devices', 'pc-play', 'practice', 'actions']);
  expect(new Set(onboardingSteps)).toEqual(new Set([...phone, ...pc]));
});

test('the phone path walks from welcome to Finish, with a live practice poke', async () => {
  const onClose = vi.fn();
  render(<Harness onClose={onClose} hotkey="Ctrl+Shift+F9" />);
  expect(screen.getByRole('dialog', { name: 'Welcome to Veyl' })).toBeDefined();
  expect(dialog().textContent).toContain(
    'Control your PC by poking the air. Veyl listens to the echo of the music you play.',
  );
  click('Get started');
  expect(title()).toBe('How do you want to use Veyl?');
  const phone = within(dialog()).getByRole('button', { name: /With my phone/ });
  expect(phone.textContent).toContain('Recommended');
  expect(within(dialog()).getByRole('button', { name: /With this PC/ }).textContent).not.toContain(
    'Recommended',
  );
  fireEvent.click(phone);
  await waitFor(() => expect(title()).toBe('Get Veyl on your phone'));
  expect(setSource).toHaveBeenCalledWith('phone');
  expect(within(dialog()).getByText('jacobbalane.github.io/Veyl').className).toBe('onboarding-url');
  click('Next');
  expect(title()).toBe('Pair your phone');
  expect(dialog().textContent).toContain('Your phone appears here when Veyl is open on it.');
  click('Pair');
  await waitFor(() => expect(pairPhone).toHaveBeenCalledWith({ nearbyId: 'abcd' }));
  click('Next');
  expect(title()).toBe('Place your phone');
  click('Next');
  expect(title()).toBe('Play something');
  expect(dialog().textContent).toContain('YouTube or YouTube Music work best');
  click('Next');
  expect(title()).toBe('Try a poke');
  expect(dialog().textContent).toContain('Hold your hand about 20 cm above your phone.');
  await waitFor(() => expect(startPokeTest).toHaveBeenCalledOnce());
  expect(within(dialog()).getByRole('status').textContent).toContain('No hand seen');
  act(() => native.gestures.forEach((update) => update(gestureJson as GestureEvent)));
  expect(within(dialog()).getByRole('status').textContent).toBe('Nice! Veyl saw that.');
  click('Next');
  await waitFor(() => expect(stopPokeTest).toHaveBeenCalledOnce());
  expect(title()).toBe("Turn on actions when you're ready");
  expect(dialog().textContent).toContain(
    'When actions are on, each poke clicks where your pointer is. Press Ctrl+Shift+F9 any time to switch them on or off.',
  );
  // Going back starts a new practice, and leaving it stops that one too.
  click('Back');
  expect(title()).toBe('Try a poke');
  await waitFor(() => expect(startPokeTest).toHaveBeenCalledTimes(2));
  click('Next');
  expect(stopPokeTest).toHaveBeenCalledTimes(2);
  click('Finish');
  await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
  expect(setSettings).toHaveBeenCalledWith({ onboardingDone: true });
});

test('the PC path asks for speakers and a microphone and plays on this PC', async () => {
  render(<Harness initialStep="choose" recommended="pc" />);
  const pc = within(dialog()).getByRole('button', { name: /With this PC/ });
  expect(pc.textContent).toContain('Recommended');
  fireEvent.click(pc);
  await waitFor(() => expect(title()).toBe('Choose speakers and a microphone'));
  expect(setSource).toHaveBeenCalledWith('pc');
  expect(within(dialog()).getByLabelText('Speakers')).toBeDefined();
  expect(within(dialog()).getByLabelText('Microphone')).toBeDefined();
  expect(within(dialog()).getByRole('button', { name: "Check this PC's sound" })).toBeDefined();
  click('Next');
  expect(title()).toBe('Play something');
  expect(dialog().textContent).toContain('Play a song or video on this PC. Any app works.');
  click('Next');
  expect(dialog().textContent).toContain('Hold your hand about 20 cm in front of your microphone.');
});

test('Escape closes onboarding and stops a running practice; Finish errors keep it open', async () => {
  const onClose = vi.fn();
  const view = render(<Harness initialStep="practice" onClose={onClose} />);
  await waitFor(() => expect(startPokeTest).toHaveBeenCalledOnce());
  fireEvent(dialog(), new Event('cancel', { cancelable: true }));
  expect(onClose).toHaveBeenCalledOnce();
  view.unmount();
  await waitFor(() => expect(stopPokeTest).toHaveBeenCalledOnce());
  native.refuse = true;
  render(<Harness initialStep="actions" onClose={onClose} />);
  click('Finish');
  expect(await within(dialog()).findByRole('alert')).toBeDefined();
  expect(within(dialog()).getByRole('alert').textContent).toBe(
    "Veyl couldn't save your settings. Try again.",
  );
  expect(onClose).toHaveBeenCalledOnce();
  click('Close');
  expect(onClose).toHaveBeenCalledTimes(2);
});
