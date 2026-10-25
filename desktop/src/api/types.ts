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
  latency: LatencyView;
}
/** Time from a poke's turning point to its detection on the PC, over the last 50 pokes. */
export interface LatencyView {
  pokes: number;
  medianMs: number | null;
  p95Ms: number | null;
}
export interface Diagnostics {
  link: LinkSnapshot;
  levels: LevelsView;
  source: SourceView | null;
}
export interface AppState {
  version: string;
  pcName: string;
  /** Whether pokes run their action right now; only Rust changes it. */
  actionsEnabled: boolean;
  link: LinkSnapshot;
  source: SourceView;
  engine: EngineView;
  settings: SettingsView;
}
export type PokeAction = 'click' | 'play_pause' | 'next_track' | 'space' | 'none';
export type Sensitivity = 'low' | 'normal' | 'high';
export type DistanceRange = 'near' | 'normal' | 'far';
/** The settings the interface shows and changes through `set_settings`. */
export interface SettingsView {
  pcName: string;
  pokeAction: PokeAction;
  sensitivity: Sensitivity;
  range: DistanceRange;
  /** Modifiers `Ctrl`, `Alt`, `Shift`, `Super` and one key, joined with `+`, e.g. "Ctrl+Alt+V". */
  hotkey: string;
  actionsOnAtStart: boolean;
  startWithWindows: boolean;
  closeToTray: boolean;
  /** Applies from the next start. */
  detailedLogs: boolean;
  /** 0 keeps test recordings until they are deleted. */
  keepRecordingsDays: number;
  temperatureC: number;
  onboardingDone: boolean;
}
/** A change to some settings; Rust rejects unknown fields. */
export type SettingsPatch = Partial<SettingsView>;
/** Why `set_settings` refused a change. Other errors are shown as a general failure. */
export type SettingsError =
  | 'invalid_pc_name'
  | 'invalid_poke_action'
  | 'invalid_sensitivity'
  | 'invalid_range'
  | 'invalid_hotkey'
  | 'invalid_keep_days'
  | 'invalid_temperature'
  | 'hotkey_unavailable'
  | 'autostart_failed'
  | 'settings_save_failed'
  | 'engine_busy'
  | 'engine_stopped';
export const defaultSettings: SettingsView = {
  pcName: 'Windows PC',
  pokeAction: 'click',
  sensitivity: 'normal',
  range: 'normal',
  hotkey: 'Ctrl+Alt+V',
  actionsOnAtStart: false,
  startWithWindows: false,
  closeToTray: true,
  detailedLogs: false,
  keepRecordingsDays: 7,
  temperatureC: 20,
  onboardingDone: false,
};
/** 'veyl://actions': the switch changed (header, tray or hotkey), or Windows blocked an action. */
export interface ActionsEvent {
  enabled: boolean;
  error: 'blocked_elevated' | null;
}
/** What `stop_poke_test` counted: at most 10 pokes within 20 s. */
export interface PokeTestResult {
  seen: number;
  of: number;
}
export interface AudioDevice {
  id: string;
  name: string;
  formFactor: string;
  rawSupported: boolean;
  bluetooth: boolean;
  builtIn: boolean;
}
export interface AudioDevices {
  speakers: AudioDevice[];
  microphones: AudioDevice[];
  defaultSpeakerId: string | null;
  defaultMicrophoneId: string | null;
  hasBattery: boolean | null;
  recommendedSource: string;
  revision: number;
  error: string | null;
}
export interface CaptureDetails {
  format: { sampleRate: number; channels: number; encoding: string; frameStride: number };
  rawMode: boolean;
  eventDriven: boolean;
  bufferMs: number;
  proAudioThread: boolean;
}
export interface SourceView {
  mode: string;
  choiceDone: boolean;
  state: string;
  speakerId: string | null;
  microphoneId: string | null;
  speaker: AudioDevice | null;
  microphone: AudioDevice | null;
  reference: CaptureDetails | null;
  capture: CaptureDetails | null;
  devices: AudioDevices;
  check: { state: string; result: CheckResult | null; error: string | null };
  tip: string | null;
  error: string | null;
  revision: number;
}
export const emptySource: SourceView = {
  mode: 'phone',
  choiceDone: false,
  state: 'idle',
  speakerId: null,
  microphoneId: null,
  speaker: null,
  microphone: null,
  reference: null,
  capture: null,
  tip: null,
  error: null,
  revision: 0,
  check: { state: 'idle', result: null, error: null },
  devices: {
    speakers: [],
    microphones: [],
    defaultSpeakerId: null,
    defaultMicrophoneId: null,
    hasBattery: null,
    recommendedSource: 'phone',
    revision: 0,
    error: 'loading',
  },
};
export interface CheckResult {
  good: boolean;
  meanCoherence: number;
  coverage: number;
  bandwidthHz: number;
  lagMs: number;
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
  latency: { pokes: 0, medianMs: null, p95Ms: null },
};
export type EngineState = 'no_audio' | 'waiting_for_sound' | 'aligning' | 'settling' | 'running';
export type TipId =
  | 'capture_off'
  | 'not_speaker'
  | 'pc_headphones'
  | 'pc_no_mic'
  | 'capture_blocked'
  | 'nothing_playing'
  | 'mic_silenced'
  | 'clipping'
  | 'volume_low'
  | 'few_high_sounds'
  | 'gaps'
  | 'other_sounds'
  | 'unclear_echo'
  | 'noisy_room'
  | 'pc_bluetooth'
  | 'ready';
export interface HandView {
  present: boolean;
  distanceCm: number | null;
}
export interface Meter {
  score: number;
  label: 'great' | 'good' | 'fair' | 'poor' | 'none';
  high: number;
  detail: number;
  volume: number;
  steady: number;
  clarity: number;
  detailCm: number | null;
  tip: TipId;
}
/** About 30 per second through `subscribeLive`; the meter rides along about 10 times a second. */
export interface LiveFrame {
  t: number;
  state: EngineState;
  motion: number[];
  hand: HandView;
  meter: Meter | null;
}
/** Why pokes are not detected right now. */
export type Suspension = 'paused' | 'source' | 'starting' | 'sound' | 'settling';
export type EngineWarning = 'relocked' | 'model_jump' | 'impulsive_noise' | 'drift';
/** 'veyl://engine' and the engine part of the app state. */
export interface EngineView {
  state: EngineState;
  suspended: Suspension | null;
  warning: EngineWarning | null;
  hand: HandView;
  meter: Meter | null;
}
/** 'veyl://gesture' */
export interface GestureEvent {
  kind: 'poke';
  at: number;
  distanceCm: number;
  depthCm: number;
  confidence: number;
  acted: boolean;
  testOnly: boolean;
}
export const emptyEngine: EngineView = {
  state: 'no_audio',
  suspended: null,
  warning: null,
  hand: { present: false, distanceCm: null },
  meter: null,
};
