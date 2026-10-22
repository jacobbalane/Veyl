import { invoke, isTauri } from '@tauri-apps/api/core';
import metadata from '../../package.json';

export interface AppState {
  version: string;
  pcName: string;
  actionsEnabled: boolean;
}

export const browserPreview = !isTauri();

export async function getAppState(): Promise<AppState> {
  return browserPreview
    ? { version: metadata.version, pcName: 'Windows PC', actionsEnabled: false }
    : invoke<AppState>('get_app_state');
}

export async function uiReady(): Promise<void> {
  if (!browserPreview) await invoke('ui_ready');
}
