// The browser preview: without the Windows app, `api/native.ts` reads its state and events from
// here. Everything is sample data computed from the time since the page opened, so the same
// options and time always give the same values. Nothing here opens a device or the network.
import metadata from '../../package.json';
import {
  emptyLevels,
  emptyLink,
  emptySource,
  type AppState,
  type AudioDevice,
  type ConnectionState,
  type Diagnostics,
  type EngineState,
  type EngineView,
  type EngineWarning,
  type GestureEvent,
  type HandView,
  type LinkSnapshot,
  type LinkUpdate,
  type LiveFrame,
  type Meter,
  type SourceView,
  type StreamLevels,
  type Suspension,
  type TipId,
} from '../api/types';
import { tipIds } from '../copy';

const engineStates = Object.keys({
  no_audio: 0,
  waiting_for_sound: 0,
  aligning: 0,
  settling: 0,
  running: 0,
} satisfies Record<EngineState, 0>) as EngineState[];
const suspensions = Object.keys({
  paused: 0,
  source: 0,
  starting: 0,
  sound: 0,
  settling: 0,
} satisfies Record<Suspension, 0>) as Suspension[];
const warnings = Object.keys({
  relocked: 0,
  model_jump: 0,
  impulsive_noise: 0,
  drift: 0,
} satisfies Record<EngineWarning, 0>) as EngineWarning[];
const connections = [
  'searching',
  'nearby',
  'connecting',
  'pairing',
  'connected',
  'listening',
  'reconnecting',
  'unreachable',
] as const;
type PreviewConnection = (typeof connections)[number];
/** The values each URL parameter accepts, for docs and tests. */
export const previewChoices = {
  connection: connections,
  state: engineStates,
  suspended: suspensions,
  warning: warnings,
  tip: tipIds,
} as const;

export interface PreviewOptions {
  mode: 'phone' | 'pc';
  /**
   * Without other options the preview connects in its first second, then aligns and settles
   * until 3.5 s; any option shows its state from the start, ready for a screenshot.
   */
  walkthrough: boolean;
  /** Null means listening. */
  connection: PreviewConnection | null;
  /** Null means running. */
  state: EngineState | null;
  suspended: Suspension | null;
  warning: EngineWarning | null;
  /** Null shows the ready tip. */
  tip: TipId | null;
  /** Null lets the score drift between Good and Great. */
  score: number | null;
  hand: boolean;
}
/**
 * Reads options from the page address, for example `?state=aligning`, `?suspended=sound`,
 * `?warning=drift`, `?tip=volume_low`, `?score=40`, `?mode=pc`, `?connection=reconnecting` or
 * `?hand=none`. Unknown values fall back to the default.
 */
export function previewOptions(search: string): PreviewOptions {
  const params = new URLSearchParams(search);
  const pick = <T extends string>(name: string, values: readonly T[]): T | null => {
    const value = params.get(name);
    return value != null && (values as readonly string[]).includes(value) ? (value as T) : null;
  };
  const score = Number(params.get('score'));
  const options = {
    mode: params.get('mode') === 'pc' ? ('pc' as const) : ('phone' as const),
    connection: pick('connection', connections),
    state: pick('state', engineStates),
    suspended: pick('suspended', suspensions),
    warning: pick('warning', warnings),
    tip: pick('tip', tipIds),
    score:
      params.has('score') && Number.isFinite(score)
        ? Math.round(Math.min(Math.max(score, 0), 100))
        : null,
    hand: params.get('hand') !== 'none',
  };
  const { connection, state, suspended, warning, tip } = options;
  const chosen = [connection, state, suspended, warning, tip, options.score];
  return { ...options, walkthrough: options.hand && chosen.every((value) => value == null) };
}

/** A poke turns every 3 s, 1.5 s into each period, and is detected 180 ms later. */
export const pokeEvery = 3000;
const pokeTurn = 1500;
const pokeDelay = 180;
const pokeDepth = 6;
/** The walk-through connects after 1 s, aligns until 2 s and settles until 3.5 s. */
const connectAt = 1000;
const runAt = 3500;
const phone = {
  id: 'preview-phone',
  name: 'Sample phone',
  model: 'Android phone',
  address: '192.168.1.23:47820',
};
// Tips that mean Veyl has no usable sound score 0 with the label "No sound" (§6.8).
const silentTips: TipId[] = [
  'capture_off',
  'not_speaker',
  'pc_headphones',
  'pc_no_mic',
  'capture_blocked',
  'nothing_playing',
];

function connection(t: number, options: PreviewOptions): PreviewConnection {
  return options.connection ?? (options.walkthrough && t < connectAt ? 'connecting' : 'listening');
}
function listening(t: number, options: PreviewOptions): boolean {
  return options.mode === 'pc'
    ? !options.walkthrough || t >= connectAt
    : connection(t, options) === 'listening';
}
function engineState(t: number, options: PreviewOptions): EngineState {
  if (options.state) return options.state;
  if (!listening(t, options)) return 'no_audio';
  if (!options.walkthrough) return 'running';
  return t < 2000 ? 'aligning' : t < runAt ? 'settling' : 'running';
}
function suspension(t: number, options: PreviewOptions): Suspension | null {
  if (options.suspended) return options.suspended;
  // The engine marks every state before running as suspended.
  return engineState(t, options) === 'running' ? null : 'starting';
}
/** Whether the engine could see a hand: it runs and has sound to work with. */
function tracking(t: number, options: PreviewOptions): boolean {
  const suspended = suspension(t, options);
  return (
    options.hand &&
    engineState(t, options) === 'running' &&
    suspended !== 'source' &&
    suspended !== 'sound' &&
    suspended !== 'paused' &&
    !silentTips.includes(options.tip ?? 'ready')
  );
}

const round1 = (value: number) => Math.round(value * 10) / 10;
const wave = (t: number, period: number, phase: number) =>
  Math.sin((2 * Math.PI * t) / period + phase);
/** How far the hand moves toward the phone for a poke, 0 to 6 cm over 300 ms. */
function pokeDip(t: number): number {
  const phase = ((t % pokeEvery) + pokeEvery) % pokeEvery;
  return pokeDepth * Math.max(0, 1 - Math.abs(phase - pokeTurn) / 150);
}
/** The hand hovers between 17 and 23 cm. */
const hover = (t: number) => 20 + 3 * wave(t, 4100, 0);

export function previewHand(t: number, options: PreviewOptions): HandView {
  return tracking(t, options)
    ? { present: true, distanceCm: round1(hover(t) - pokeDip(t)) }
    : { present: false, distanceCm: null };
}

export function previewMeter(t: number, options: PreviewOptions): Meter | null {
  if (!listening(t, options) && options.state == null) return null;
  const tip = options.tip ?? 'ready';
  if (silentTips.includes(tip))
    return {
      score: 0,
      label: 'none',
      high: 0,
      detail: 0,
      volume: 0,
      steady: 0,
      clarity: 0,
      detailCm: null,
      tip,
    };
  const base = options.score == null ? null : options.score / 100;
  const factor = (normal: number, period: number, phase: number) =>
    Math.round(Math.min(Math.max((base ?? normal) + 0.05 * wave(t, period, phase), 0), 1) * 100) /
    100;
  const high = factor(0.8, 17000, 0);
  const detail = factor(0.72, 23000, 1);
  const volume = factor(0.86, 13000, 2);
  const steady = factor(0.92, 29000, 3);
  const clarity = factor(0.7, 19000, 4);
  const score =
    options.score ??
    Math.round(100 * (0.2 * high + 0.2 * detail + 0.2 * volume + 0.15 * steady + 0.25 * clarity));
  return {
    score,
    label: score >= 75 ? 'great' : score >= 55 ? 'good' : score >= 35 ? 'fair' : 'poor',
    high,
    detail,
    volume,
    steady,
    clarity,
    detailCm: round1(1.8 + 0.2 * wave(t, 23000, 1)),
    tip,
  };
}

/** A small, repeatable pseudo-random value from 0 to 1. */
function noise(a: number, b: number): number {
  const value = Math.sin(a * 12.9898 + b * 78.233) * 43758.5453;
  return value - Math.floor(value);
}
/** 60 motion values in dB: faint background, the hand's echo, and a bright band during a poke. */
export function previewMotion(t: number, hand: HandView): number[] {
  const step = Math.floor(t / (1000 / 30));
  return Array.from({ length: 60 }, (_, cm) => {
    let value = noise(step, cm) * 1.2;
    if (hand.present && hand.distanceCm != null) {
      const near = Math.max(0, 1 - Math.abs(cm + 0.5 - hand.distanceCm) / 3.5);
      const moving = pokeDip(t) / pokeDepth;
      value = Math.max(value, near * (10 + 18 * moving + 2 * noise(cm, step)));
    }
    return round1(Math.min(value, 30));
  });
}

export function previewFrame(t: number, index: number, options: PreviewOptions): LiveFrame {
  const hand = previewHand(t, options);
  return {
    t,
    state: engineState(t, options),
    motion: previewMotion(t, hand),
    hand,
    // The meter rides along with every third frame, about 10 times a second.
    meter: index % 3 === 0 ? previewMeter(t, options) : null,
  };
}

export function previewEngine(t: number, options: PreviewOptions): EngineView {
  return {
    state: engineState(t, options),
    suspended: suspension(t, options),
    warning: options.warning,
    hand: previewHand(t, options),
    meter: previewMeter(t, options),
  };
}

/** Pokes detected after `from` and up to `to`; only a running, unsuspended engine reports them. */
export function previewPokes(from: number, to: number, options: PreviewOptions): GestureEvent[] {
  const pokes: GestureEvent[] = [];
  for (let k = Math.max(0, Math.floor((from - pokeTurn - pokeDelay) / pokeEvery)); ; k += 1) {
    const turn = k * pokeEvery + pokeTurn;
    const at = turn + pokeDelay;
    if (at > to) break;
    if (at <= from || !tracking(turn, options) || suspension(turn, options) != null) continue;
    pokes.push({
      kind: 'poke',
      at,
      distanceCm: round1(hover(turn) - pokeDepth),
      depthCm: pokeDepth,
      confidence: 0.9,
      acted: false,
      testOnly: true,
    });
  }
  return pokes;
}

export function previewLink(t: number, options: PreviewOptions): LinkSnapshot {
  const link = structuredClone(emptyLink);
  const kind = connection(t, options);
  const saved = {
    phoneId: phone.id,
    phoneName: phone.name,
    model: phone.model,
    pairedAt: '2026-10-20T18:00:00Z',
    lastSeen: '2026-10-25T09:00:00Z',
    lastAddress: phone.address,
  };
  if (kind === 'nearby') {
    link.phones.nearby = [phone];
    return link;
  }
  link.phones.paired = [saved];
  if (kind === 'searching') return link;
  const state: Record<Exclude<PreviewConnection, 'searching' | 'nearby'>, string> = {
    connecting: 'connecting',
    pairing: 'pairing',
    connected: 'connected',
    listening: 'connected',
    reconnecting: 'reconnecting',
    unreachable: 'unreachable',
  };
  const value: ConnectionState = {
    state: state[kind],
    phoneId: phone.id,
    phoneName: phone.name,
    address: phone.address,
    capture:
      kind === 'listening'
        ? { state: 'running', source: 'capture', route: 'speaker', reason: null }
        : null,
    error: null,
    attempt: kind === 'reconnecting' ? 2 : 0,
  };
  link.connection = value;
  if (kind === 'pairing')
    link.pairing = {
      sessionId: 'preview-pairing',
      phoneName: phone.name,
      code: '123456',
      stage: 'compare',
      error: null,
    };
  if (kind === 'connected' || kind === 'listening')
    link.phoneStats = {
      batteryPct: 82,
      charging: false,
      queueMs: 20,
      droppedBlocks: 0,
      mediaVolume: 0.6,
      thermal: 'none',
      cpuPct: 4,
      rttMs: 6,
    };
  return link;
}

export function previewSource(t: number, options: PreviewOptions): SourceView {
  const source = structuredClone(emptySource);
  source.choiceDone = true;
  source.revision = 1;
  source.devices.error = null;
  if (options.mode !== 'pc') return source;
  const device = (id: string, name: string, formFactor: string): AudioDevice => ({
    id,
    name,
    formFactor,
    rawSupported: true,
    bluetooth: false,
    builtIn: true,
  });
  const speaker = device('preview-speakers', 'Speakers', 'speakers');
  const microphone = device('preview-microphone', 'Microphone', 'microphone');
  return {
    ...source,
    mode: 'pc',
    state: listening(t, options) ? 'running' : 'starting',
    speaker,
    microphone,
    devices: {
      speakers: [speaker],
      microphones: [microphone],
      defaultSpeakerId: speaker.id,
      defaultMicrophoneId: microphone.id,
      hasBattery: true,
      recommendedSource: 'pc',
      revision: 1,
      error: null,
    },
  };
}

export function previewDiagnostics(t: number, options: PreviewOptions): Diagnostics {
  const levels = structuredClone(emptyLevels);
  if (listening(t, options)) {
    const stream = (preDbfs: number, filteredDbfs: number): StreamLevels => ({
      blocks: Math.floor((t - (options.walkthrough ? connectAt : 0)) / 10),
      blocksPerSecond: 100,
      preDbfs,
      filteredDbfs,
      peak: 0.4,
      gaps: 0,
      latestFrame: null,
    });
    levels.reference = stream(-22.5, -36);
    levels.microphone = stream(-31.5, -44);
    levels.sampleRate = 48000;
    levels.arrivalMs = options.mode === 'pc' ? null : 42;
    levels.running = true;
    const pokes = Math.min(previewPokes(0, t, options).length, 50);
    levels.latency =
      pokes > 0 ? { pokes, medianMs: 148, p95Ms: 212 } : { pokes, medianMs: null, p95Ms: null };
  }
  return { link: previewLink(t, options), levels, source: previewSource(t, options) };
}

// The live preview: a clock from when this module loaded and the options in the page address.
const origin = performance.now();
const clock = () => performance.now() - origin;
const options = previewOptions(globalThis.location?.search ?? '');

/** Calls `tick` every `ms` until stopped; `tick` decides itself whether anything changed. */
function every(ms: number, tick: () => void): () => void {
  const timer = setInterval(tick, ms);
  return () => clearInterval(timer);
}

export async function getAppState(): Promise<AppState> {
  const t = clock();
  return {
    version: metadata.version,
    pcName: 'Windows PC',
    actionsEnabled: false,
    link: previewLink(t, options),
    source: previewSource(t, options),
    engine: previewEngine(t, options),
  };
}
export async function getDiagnostics(): Promise<Diagnostics> {
  return previewDiagnostics(clock(), options);
}
export async function watchLink(update: (value: LinkUpdate) => void): Promise<() => void> {
  let last = JSON.stringify(previewLink(clock(), options));
  return every(250, () => {
    const link = previewLink(clock(), options);
    const next = JSON.stringify(link);
    if (next === last) return;
    last = next;
    update({ phones: link.phones });
    update({ connection: link.connection });
  });
}
export async function watchSource(update: (source: SourceView) => void): Promise<() => void> {
  let last = previewSource(clock(), options).state;
  let revision = 1;
  return every(250, () => {
    const source = previewSource(clock(), options);
    if (source.state === last) return;
    last = source.state;
    revision += 1;
    source.revision = revision;
    update(source);
  });
}
let stopLive: (() => void) | undefined;
export async function subscribeLive(update: (frame: LiveFrame) => void): Promise<void> {
  stopLive?.();
  let index = 0;
  stopLive = every(1000 / 30, () => update(previewFrame(clock(), index++, options)));
}
export async function watchEngine(update: (engine: EngineView) => void): Promise<() => void> {
  // Like the Windows app, announce only what is shown in words; values travel in live frames.
  const key = (view: EngineView) =>
    [
      view.state,
      view.suspended,
      view.warning,
      view.hand.present,
      view.meter?.label,
      view.meter?.tip,
    ].join();
  let last = key(previewEngine(clock(), options));
  return every(100, () => {
    const view = previewEngine(clock(), options);
    if (key(view) === last) return;
    last = key(view);
    update(view);
  });
}
export async function watchGestures(update: (gesture: GestureEvent) => void): Promise<() => void> {
  let from = clock();
  return every(100, () => {
    const to = clock();
    previewPokes(from, to, options).forEach(update);
    from = to;
  });
}
