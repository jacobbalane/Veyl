import { Channel, invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  EngineView,
  GestureEvent,
  LiveFrame,
  SourceView,
  AppState,
  Diagnostics,
  LinkUpdate,
  ConnectionState,
  PhonesView,
  PairingEvent,
  Capture,
  PhoneTarget,
  RecordingInfo,
  ActionsEvent,
  PokeTestResult,
  SettingsPatch,
  SettingsView,
} from './types';
import type { PreviewView } from '../preview/fake-native';
export type { AppState } from './types';

export const browserPreview = !isTauri();
// Without Tauri, state and events come from sample data; the Windows app never loads it.
const preview = () => import('../preview/fake-native');

export async function getAppState(): Promise<AppState> {
  return browserPreview ? (await preview()).getAppState() : invoke<AppState>('get_app_state');
}
/** The tab, dialog or onboarding step the preview's page address asks for; null in the app. */
export async function previewView(): Promise<PreviewView | null> {
  return browserPreview ? (await preview()).previewView() : null;
}

/** Turns actions on or off; the new state arrives as a 'veyl://actions' event. */
export async function setActionsEnabled(enabled: boolean): Promise<void> {
  if (browserPreview) return (await preview()).setActionsEnabled(enabled);
  await invoke('set_actions_enabled', { enabled });
}
export async function watchActions(update: (event: ActionsEvent) => void): Promise<UnlistenFn> {
  return browserPreview
    ? (await preview()).watchActions(update)
    : listen<ActionsEvent>('veyl://actions', ({ payload }) => update(payload));
}
/** Saves a settings change and returns all settings; rejects with a `SettingsError` code. */
export async function setSettings(patch: SettingsPatch): Promise<SettingsView> {
  return browserPreview
    ? (await preview()).setSettings(patch)
    : invoke<SettingsView>('set_settings', { patch });
}
/** While a poke test runs, pokes are counted and actions never run. */
export async function startPokeTest(): Promise<void> {
  if (browserPreview) return (await preview()).startPokeTest();
  await invoke('start_poke_test');
}
export async function stopPokeTest(): Promise<PokeTestResult> {
  return browserPreview
    ? (await preview()).stopPokeTest()
    : invoke<PokeTestResult>('stop_poke_test');
}
export async function openLogsFolder(): Promise<void> {
  if (browserPreview) return (await preview()).openLogsFolder();
  await invoke('open_logs_folder');
}

export async function uiReady(): Promise<void> {
  if (!browserPreview) await invoke('ui_ready');
}

async function command(name: string, args?: Record<string, unknown>): Promise<void> {
  if (browserPreview) throw new Error('Open the Windows app to connect your devices.');
  await invoke(name, args);
}
export const pairPhone = (target: PhoneTarget) => command('pair_phone', { target });
export const connectPhone = (target: PhoneTarget) => command('connect_phone', { target });
export const confirmPairing = (sessionId: string, matches: boolean) =>
  command('confirm_pairing', { sessionId, matches });
export const disconnect = () => command('disconnect');
export const forgetPhone = (phoneId: string) => command('forget_phone', { phoneId });
export const requestPhoneStart = () => command('request_phone_start');
export const startRecording = (seconds: 30 | 60 | 120, markPokes: boolean) =>
  invoke<RecordingInfo>('start_recording', { seconds, markPokes });
export const stopRecording = () => invoke<RecordingInfo>('stop_recording');
export const markPoke = () => command('mark_poke');
export const deleteRecording = (id: string) => command('delete_recording', { id });
export const openRecordingsFolder = () => command('open_recordings_folder');
/** Veyl's website and repository open in the default browser; the window never navigates. */
export async function openLink(url: string): Promise<void> {
  if (browserPreview) window.open(url, '_blank', 'noopener,noreferrer');
  else await invoke('open_link', { url });
}
export const getRecording = async (): Promise<RecordingInfo | null> =>
  browserPreview ? null : invoke('get_recording');
export const listRecordings = async (): Promise<RecordingInfo[]> =>
  browserPreview ? [] : invoke('list_recordings');
export const watchRecording = async (
  update: (info: RecordingInfo | null) => void,
): Promise<UnlistenFn> =>
  browserPreview
    ? () => {}
    : listen<RecordingInfo | null>('veyl://recording', ({ payload }) => update(payload));
export async function getDiagnostics(): Promise<Diagnostics> {
  return browserPreview
    ? (await preview()).getDiagnostics()
    : invoke<Diagnostics>('get_diagnostics');
}
// The preview switches its sample data between phone and PC mode, so onboarding works there.
export const setSource = async (source: string) =>
  browserPreview ? (await preview()).setSource(source) : command('set_source', { source });
export const checkPcSound = () => command('check_pc_sound');
export const cancelPcSoundCheck = () => command('cancel_pc_sound_check');
export const setPcDevices = (speakerId: string | null, microphoneId: string | null) =>
  command('set_pc_devices', { speakerId, microphoneId });
export const watchSource = async (update: (source: SourceView) => void): Promise<UnlistenFn> =>
  browserPreview
    ? (await preview()).watchSource(update)
    : listen<SourceView>('veyl://source', ({ payload }) => update(payload));
export async function copyDiagnostics(): Promise<string> {
  return browserPreview
    ? JSON.stringify(await getDiagnostics(), null, 2)
    : invoke<string>('copy_diagnostics');
}
export async function watchLink(update: (value: LinkUpdate) => void): Promise<UnlistenFn> {
  if (browserPreview) return (await preview()).watchLink(update);
  const results = await Promise.allSettled([
    listen<ConnectionState>('veyl://connection', ({ payload }) => update({ connection: payload })),
    listen<PhonesView>('veyl://phones', ({ payload }) => update({ phones: payload })),
    listen<PairingEvent>('veyl://pairing', ({ payload }) => update({ pairing: payload })),
    listen<Capture>('veyl://capture', ({ payload }) => update({ capture: payload })),
  ]);
  const stops: UnlistenFn[] = [];
  for (const result of results) if (result.status === 'fulfilled') stops.push(result.value);
  const stop = () => stops.forEach((stop) => stop());
  if (results.some((result) => result.status === 'rejected')) {
    stop();
    throw new Error('Connection events are unavailable.');
  }
  return stop;
}
/** Live echo-view frames, about 30 per second; a new subscription replaces the previous one. */
export async function subscribeLive(update: (frame: LiveFrame) => void): Promise<void> {
  if (browserPreview) return (await preview()).subscribeLive(update);
  const channel = new Channel<LiveFrame>();
  channel.onmessage = update;
  await invoke('subscribe_live', { channel });
}
export const watchEngine = async (update: (engine: EngineView) => void): Promise<UnlistenFn> =>
  browserPreview
    ? (await preview()).watchEngine(update)
    : listen<EngineView>('veyl://engine', ({ payload }) => update(payload));
export const watchGestures = async (
  update: (gesture: GestureEvent) => void,
): Promise<UnlistenFn> =>
  browserPreview
    ? (await preview()).watchGestures(update)
    : listen<GestureEvent>('veyl://gesture', ({ payload }) => update(payload));
