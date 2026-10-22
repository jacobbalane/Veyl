export interface Capture {
  state: string;
  source: string;
  route: string;
  reason: string | null;
}
export interface ConnectionState {
  state: string;
  phoneId: string | null;
  phoneName: string | null;
  address: string | null;
  capture: Capture | null;
  error: string | null;
  attempt: number;
}
export interface SavedPhone {
  phoneId: string;
  phoneName: string;
  model: string;
  pairedAt: string;
  lastSeen: string;
  lastAddress: string;
}
export interface NearbyPhone {
  id: string;
  name: string;
  model: string;
  address: string;
}
export interface PhonesView {
  paired: SavedPhone[];
  nearby: NearbyPhone[];
}
export interface PairingEvent {
  sessionId: string;
  phoneName: string;
  code: string | null;
  stage: string;
  error: string | null;
}
export interface PhoneStats {
  batteryPct: number | null;
  charging: boolean;
  queueMs: number;
  droppedBlocks: number;
  mediaVolume: number | null;
  thermal: string | null;
  cpuPct: number | null;
  rttMs: number | null;
}
export interface LinkSnapshot {
  connection: ConnectionState;
  phones: PhonesView;
  pairing: PairingEvent | null;
  phoneStats: PhoneStats;
}
export interface StreamLevels {
  blocks: number;
  blocksPerSecond: number;
  preDbfs: number;
  filteredDbfs: number;
  peak: number;
  gaps: number;
  latestFrame: number | null;
}
export interface LevelsView {
  runId: number | null;
  reference: StreamLevels;
  microphone: StreamLevels;
  sampleRate: number | null;
  engineDrops: number;
  arrivalMs: number | null;
  running: boolean;
}
export interface Diagnostics {
  link: LinkSnapshot;
  levels: LevelsView;
}
export interface AppState {
  version: string;
  pcName: string;
  actionsEnabled: boolean;
  link: LinkSnapshot;
}
export interface RecordingInfo {
  id: string;
  seconds: number;
  markPokes: boolean;
  state: string;
  remainingSeconds: number;
  blocks: number[];
  error: string | null;
}
export type PhoneTarget = { nearbyId: string } | { address: string } | { phoneId: string };
export type LinkUpdate =
  | { connection: ConnectionState }
  | { phones: PhonesView }
  | { pairing: PairingEvent }
  | { capture: Capture };
export const emptyLink: LinkSnapshot = {
  connection: {
    state: 'browsing',
    phoneId: null,
    phoneName: null,
    address: null,
    capture: null,
    error: null,
    attempt: 0,
  },
  phones: { paired: [], nearby: [] },
  pairing: null,
  phoneStats: {
    batteryPct: null,
    charging: false,
    queueMs: 0,
    droppedBlocks: 0,
    mediaVolume: null,
    thermal: null,
    cpuPct: null,
    rttMs: null,
  },
};
const silentStream: StreamLevels = {
  blocks: 0,
  blocksPerSecond: 0,
  preDbfs: -120,
  filteredDbfs: -120,
  peak: 0,
  gaps: 0,
  latestFrame: null,
};
export const emptyLevels: LevelsView = {
  runId: null,
  reference: silentStream,
  microphone: silentStream,
  sampleRate: null,
  engineDrops: 0,
  arrivalMs: null,
  running: false,
};
