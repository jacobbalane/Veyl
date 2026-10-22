import { invoke, isTauri } from '@tauri-apps/api/core';
import metadata from '../../package.json';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  emptyLink,
  emptyLevels,
  emptySource,
  type SourceView,
  type AppState,
  type Diagnostics,
  type LinkUpdate,
  type ConnectionState,
  type PhonesView,
  type PairingEvent,
  type Capture,
  type PhoneTarget,
  type RecordingInfo,
} from './types';
export type { AppState } from './types';

export const browserPreview = !isTauri();

export async function getAppState(): Promise<AppState> {
  return browserPreview
    ? {
        version: metadata.version,
        pcName: 'Windows PC',
        actionsEnabled: false,
        link: structuredClone(emptyLink),
        source: structuredClone(emptySource),
      }
    : invoke<AppState>('get_app_state');
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
    ? {
        link: structuredClone(emptyLink),
        levels: structuredClone(emptyLevels),
        source: structuredClone(emptySource),
      }
    : invoke<Diagnostics>('get_diagnostics');
}
export const setSource = (source: string) => command('set_source', { source });
export const setPcDevices = (speakerId: string | null, microphoneId: string | null) =>
  command('set_pc_devices', { speakerId, microphoneId });
export const watchSource = async (update: (source: SourceView) => void): Promise<UnlistenFn> =>
  browserPreview ? () => {} : listen<SourceView>('veyl://source', ({ payload }) => update(payload));
export async function copyDiagnostics(): Promise<string> {
  return browserPreview
    ? JSON.stringify(await getDiagnostics(), null, 2)
    : invoke<string>('copy_diagnostics');
}
export async function watchLink(update: (value: LinkUpdate) => void): Promise<UnlistenFn> {
  if (browserPreview) return () => {};
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
