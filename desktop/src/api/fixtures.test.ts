import { expect, test } from 'vitest';
import appJson from './fixtures/app-state.json';
import diagnosticsJson from './fixtures/diagnostics.json';
import pairingJson from './fixtures/pairing.json';
import recordingJson from './fixtures/recording.json';
import checkJson from './fixtures/pc-check.json';
import engineJson from './fixtures/engine.json';
import liveJson from './fixtures/live-frame.json';
import gestureJson from './fixtures/gesture.json';
import actionsJson from './fixtures/actions.json';
import pokeTestJson from './fixtures/poke-test.json';
import type {
  AppState,
  Diagnostics,
  PairingEvent,
  RecordingInfo,
  CheckResult,
  EngineView,
  LiveFrame,
  GestureEvent,
  ActionsEvent,
  PokeTestResult,
  SettingsView,
} from './types';
import { defaultSettings } from './types';

test('Rust fixtures fit the typed desktop API and contain metadata only', () => {
  const app = appJson as AppState;
  const diagnostics: Diagnostics = diagnosticsJson;
  const pairing: PairingEvent = pairingJson;
  const recording: RecordingInfo = recordingJson;
  const check: CheckResult = checkJson;
  // JSON imports widen string unions, so the engine payloads are checked through their values.
  const engine = engineJson as EngineView;
  const live = liveJson as LiveFrame;
  const gesture = gestureJson as GestureEvent;
  expect(app.engine.state).toBe('no_audio');
  expect(engine.meter?.tip).toBe('few_high_sounds');
  expect(engine.hand.distanceCm).toBe(18.5);
  expect(live.motion).toHaveLength(60);
  expect(live.meter?.detailCm).toBe(1.75);
  expect(gesture.testOnly).toBe(true);
  expect(diagnostics.levels.latency.medianMs).toBeNull();
  expect(check.bandwidthHz).toBe(10000);
  expect(check.meanCoherence).toBe(0.75);
  expect(app.actionsEnabled).toBe(false);
  expect(app.link.connection.state).toBe('browsing');
  expect(diagnostics.levels.reference.preDbfs).toBe(-120);
  expect(diagnostics.link.pairing).toBeNull();
  expect(pairing.sessionId).toBe('test-comparison');
  expect(pairing.code).toMatch(/^\d{6}$/);
  expect(Object.keys(diagnostics.levels)).not.toContain('samples');
  expect(Object.keys(app.link)).not.toContain('keys');
  expect(recording.blocks).toEqual([1000, 1000]);
  expect(recording.remainingSeconds).toBe(20);
});

test('settings, the actions event and the poke test result match their Rust payloads', () => {
  // JSON imports widen string unions to strings; every other field keeps its exact type.
  type Json<T> = { [K in keyof T]: T[K] extends string ? string : T[K] };
  const json: Json<SettingsView> = appJson.settings;
  const settings = json as SettingsView;
  const actions = actionsJson as ActionsEvent;
  const pokeTest: PokeTestResult = pokeTestJson;
  // Every field the interface knows, and no other; Rust refuses unknown fields in a change.
  expect(Object.keys(settings).sort()).toEqual(Object.keys(defaultSettings).sort());
  expect(settings).toEqual({ ...defaultSettings, pcName: 'Test PC' });
  expect(['click', 'play_pause', 'next_track', 'space', 'none']).toContain(settings.pokeAction);
  expect(['low', 'normal', 'high']).toContain(settings.sensitivity);
  expect(['near', 'normal', 'far']).toContain(settings.range);
  expect(settings.hotkey).toBe('Ctrl+Alt+V');
  expect(settings.keepRecordingsDays).toBe(7);
  expect(settings.onboardingDone).toBe(false);
  expect(actions).toEqual({ enabled: true, error: 'blocked_elevated' });
  expect(pokeTest).toEqual({ seen: 8, of: 10 });
});
