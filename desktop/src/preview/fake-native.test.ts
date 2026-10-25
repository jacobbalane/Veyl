import { afterEach, expect, test, vi } from 'vitest';
import { defaultSettings, type LiveFrame, type Meter, type SettingsPatch } from '../api/types';
import { engineStatus, tipText, warningText } from '../copy';
import {
  applySettings,
  getAppState,
  openLogsFolder,
  previewChoices,
  previewDiagnostics,
  previewEngine,
  previewFrame,
  previewLink,
  previewOptions,
  previewPokes,
  previewSource,
  previewTestResult,
  previewView,
  setActionsEnabled,
  setSettings,
  setSource,
  startPokeTest,
  stopPokeTest,
  subscribeLive,
  watchActions,
  watchSource,
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

test('URL options open a tab, the poke test, onboarding steps and the actions states', () => {
  expect(defaults).toMatchObject({ actions: 'off', onboarding: null, tab: null, dialog: null });
  expect(previewOptions('?tab=settings').tab).toBe('Settings');
  expect(previewOptions('?tab=Help').tab).toBe('Help');
  expect(previewOptions('?dialog=poke-test').dialog).toBe('poke-test');
  expect(previewOptions('?actions=blocked').actions).toBe('blocked');
  expect(previewOptions('?onboarding').onboarding).toBe('welcome');
  for (const step of previewChoices.onboarding)
    expect(previewOptions(`?onboarding=${step}`).onboarding).toBe(step);
  // The PC path's steps show PC mode.
  expect(previewOptions('?onboarding=pc-devices').mode).toBe('pc');
  expect(previewOptions('?onboarding=pair').mode).toBe('phone');
  // Any of them shows the running engine from the start, ready for a screenshot.
  expect(previewOptions('?tab=help').walkthrough).toBe(false);
  expect(previewOptions('?tab=nope&dialog=x&actions=maybe&onboarding=x')).toEqual(defaults);
});

test('pokes act only with actions on, outside a poke test and with an action other than Nothing', () => {
  const on = { actions: true, pokeAction: 'click' as const, testFrom: null };
  expect(previewPokes(0, 30000, defaults, on).every((poke) => poke.acted && !poke.testOnly)).toBe(
    true,
  );
  const nothing = previewPokes(0, 30000, defaults, { ...on, pokeAction: 'none' });
  expect(nothing.every((poke) => !poke.acted && !poke.testOnly)).toBe(true);
  const testing = previewPokes(0, 30000, defaults, { ...on, testFrom: 10000 });
  expect(testing.map((poke) => [poke.at, poke.acted, poke.testOnly])).toEqual([
    [4680, true, false],
    [7680, true, false],
    ...[10680, 13680, 16680, 19680, 22680, 25680, 28680].map((at) => [at, false, true]),
  ]);
  // Pokes every 3 s give six or seven in the test's 20 s, never more than ten.
  expect(previewTestResult(0, 60000, defaults)).toEqual({ seen: 6, of: 10 });
  expect(previewTestResult(10000, 60000, defaults)).toEqual({ seen: 7, of: 10 });
  expect(previewTestResult(10000, 13000, defaults)).toEqual({ seen: 1, of: 10 });
});

test('settings changes follow the Windows app’s checks and error codes', () => {
  const settings = { ...defaultSettings, pcName: 'Windows PC' };
  expect(
    applySettings(settings, { pcName: '  Desk PC ', sensitivity: 'high', temperatureC: 26.5 }),
  ).toEqual({ ...settings, pcName: 'Desk PC', sensitivity: 'high', temperatureC: 26.5 });
  expect(applySettings(settings, { keepRecordingsDays: 0, hotkey: 'Ctrl+Shift+F9' })).toMatchObject(
    { keepRecordingsDays: 0, hotkey: 'Ctrl+Shift+F9' },
  );
  const refused: [Record<string, unknown>, string][] = [
    [{ pcName: ' ' }, 'invalid_pc_name'],
    [{ pcName: 'é'.repeat(21) }, 'invalid_pc_name'],
    [{ pcName: 'Desk\u0007PC' }, 'invalid_pc_name'],
    [{ pokeAction: 'jump' }, 'invalid_poke_action'],
    [{ sensitivity: 'max' }, 'invalid_sensitivity'],
    [{ range: 'huge' }, 'invalid_range'],
    [{ hotkey: ' ' }, 'invalid_hotkey'],
    [{ hotkey: 'V' }, 'invalid_hotkey'],
    [{ hotkey: 'Ctrl+Alt+Delete' }, 'hotkey_unavailable'],
    [{ keepRecordingsDays: 999 }, 'invalid_keep_days'],
    [{ keepRecordingsDays: 1.5 }, 'invalid_keep_days'],
    [{ temperatureC: 80 }, 'invalid_temperature'],
    [{ temperatureC: Number.NaN }, 'invalid_temperature'],
  ];
  // Like Tauri, the preview rejects with the error code itself.
  const error = (patch: Record<string, unknown>) => {
    try {
      applySettings(settings, patch as SettingsPatch);
    } catch (code) {
      return code;
    }
    return null;
  };
  for (const [patch, code] of refused) expect(error(patch), code).toBe(code);
  expect(error({ pcId: 'x' })).toMatch(/unknown field pcId/);
  expect(error({ closeToTray: 'yes' })).toMatch(/closeToTray/);
});

test('the preview’s commands keep settings, actions, the poke test and the mode in memory', async () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
  expect(previewView()).toEqual({ tab: null, pokeTest: false, onboarding: null });
  const before = await getAppState();
  expect(before.settings).toMatchObject({ onboardingDone: true, hotkey: 'Ctrl+Alt+V' });
  expect(before.actionsEnabled).toBe(false);
  const events = vi.fn();
  const stop = await watchActions(events);
  await setActionsEnabled(true);
  expect(events).toHaveBeenLastCalledWith({ enabled: true, error: null });
  expect((await getAppState()).actionsEnabled).toBe(true);
  stop();
  await setActionsEnabled(false);
  expect(events).toHaveBeenCalledOnce();
  expect(await setSettings({ hotkey: 'Ctrl+Shift+F9' })).toMatchObject({
    hotkey: 'Ctrl+Shift+F9',
  });
  expect((await getAppState()).settings.hotkey).toBe('Ctrl+Shift+F9');
  await expect(setSettings({ temperatureC: 99 })).rejects.toBe('invalid_temperature');
  await startPokeTest();
  expect(await stopPokeTest()).toEqual({ seen: 0, of: 10 });
  expect(await stopPokeTest()).toEqual({ seen: 0, of: 10 });
  await expect(openLogsFolder()).resolves.toBeUndefined();
  const sources = vi.fn();
  const unwatch = await watchSource(sources);
  await setSource('pc');
  expect(sources).toHaveBeenLastCalledWith(expect.objectContaining({ mode: 'pc' }));
  expect((await getAppState()).source.mode).toBe('pc');
  await setSource('phone');
  expect((await getAppState()).source.mode).toBe('phone');
  await expect(setSource('radio')).rejects.toBe('invalid_source');
  unwatch();
});
