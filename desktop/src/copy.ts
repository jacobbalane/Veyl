// Wording comes from docs/ui-copy.md; the connection preview is recorded in decision 0006.
import type { EngineView, EngineWarning, Meter, TipId } from './api/types';

export const copy = {
  status: 'Looking for your phone…',
  preview: 'Preview — sample data',
  foundation: 'Connection preview',
  foundationDetail: 'You can pair devices and stream sound. Gesture control is coming soon.',
  echoTitle: 'What Veyl hears',
  echoEmpty: 'Move your hand above your phone to see it here.',
  echoAxis: 'Distance from phone',
  noHand: 'No hand seen',
  noSound: 'No sound',
  pokesTitle: 'Recent pokes',
  pokesEmpty: 'Pokes appear here.',
  phonesEmpty: 'Open Veyl on your phone to see it here.',
  actionsTooltip:
    'When actions are on, each poke clicks where your pointer is. Press Ctrl+Alt+V to switch them on or off.',
} as const;

export const pcCopy = {
  choose: 'How do you want to use Veyl?',
  source: 'Sound source',
  pc: 'With this PC',
  phone: 'With my phone',
  recommended: 'Recommended',
  pcDetail: "Your PC's speakers play and its microphone listens. No phone needed.",
  phoneDetail: 'Your phone plays and listens. Works with any PC.',
  speakers: 'Speakers',
  microphone: 'Microphone',
  defaultDevice: 'Default device',
  missingDevice: 'Unavailable device',
  listening: 'Listening through this PC',
  noSound: "This PC isn't hearing any sound",
  starting: "Starting this PC's sound…",
  axis: 'Distance from microphone',
  empty: 'Move your hand in front of your microphone to see it here.',
  phonesNote: "Veyl is using this PC's speakers and microphone. Switch to phone mode in Settings.",
  placementTitle: 'Placing your microphone',
  placement:
    'On a laptop, hold your hand about 20 cm in front of the screen. On a desktop, keep the microphone within about 50 cm of where you poke, with speakers on and audible.',
  saveFailed: "Veyl couldn't save your sound settings. Try again.",
  format: 'Format',
  raw: 'Raw mode',
  resampling: 'Resampling',
  buffer: 'Buffer latency',
  scheduling: 'Pro Audio scheduling',
  drift: 'Clock drift',
  on: 'On',
  off: 'Off',
  check: "Check this PC's sound",
  checking: "Checking this PC's sound…",
  checkNotice: 'Veyl plays a test sound for five seconds at your current volume.',
  checkGood: 'This PC hears its own sound well.',
  checkPoor:
    'This PC hears its sound poorly. Move the microphone closer to where you poke, or turn the volume up.',
  checkFailed: "The sound check didn't finish. Check your devices and try again.",
  cancel: 'Cancel',
  coherence: 'High-band coherence',
  bandwidth: 'High-band width',
} as const;
export function pcTip(code: string | null | undefined): string {
  switch (code) {
    case 'pc_headphones':
      return "Your PC's sound is going to headphones. Veyl needs speakers.";
    case 'pc_no_mic':
      return 'Choose a microphone in Settings.';
    case 'pc_no_speaker':
      return 'Choose speakers in Settings.';
    case 'mic_silenced':
      return 'Another app is using the microphone. Close it, or choose another microphone.';
    case 'pc_bluetooth':
      return 'Bluetooth speakers add delay. Built-in or wired speakers work better.';
    case 'device_invalidated':
      return 'Your audio device changed. Veyl is reconnecting it.';
    case 'pc_audio_error':
      return "Veyl couldn't open these audio devices. Check them in Settings.";
    default:
      return 'Play some music or a video on this PC.';
  }
}

// Tips from C.1. A tip that exists in only one mode uses that mode's text in both.
const tips: Record<TipId, { phone: string; pc: string }> = {
  capture_off: {
    phone: 'Tap Start in Veyl on your phone.',
    pc: "Veyl isn't listening. Choose your speakers and microphone in Settings.",
  },
  not_speaker: {
    phone: "Sound is going to headphones or Bluetooth. Veyl needs your phone's speaker.",
    pc: "Your PC's sound is going to headphones. Veyl needs speakers.",
  },
  pc_headphones: {
    phone: "Your PC's sound is going to headphones. Veyl needs speakers.",
    pc: "Your PC's sound is going to headphones. Veyl needs speakers.",
  },
  pc_no_mic: {
    phone: 'Choose a microphone in Settings.',
    pc: 'Choose a microphone in Settings.',
  },
  capture_blocked: {
    phone:
      "This app doesn't share its sound with Veyl. Try YouTube, YouTube Music, the Veyl player or the test sound.",
    pc: "Veyl can't copy this sound. Protected videos don't share it. Try music or another video.",
  },
  nothing_playing: {
    phone: 'Play some music or a video on your phone.',
    pc: 'Play some music or a video on this PC.',
  },
  mic_silenced: {
    phone: "Another app is using your phone's microphone.",
    pc: 'Another app is using the microphone. Close it, or choose another microphone.',
  },
  clipping: {
    phone: 'Turn the volume down a little. The sound is distorting.',
    pc: 'Turn the volume down a little. The sound is distorting.',
  },
  volume_low: { phone: "Turn your phone's volume up.", pc: "Turn your PC's volume up." },
  few_high_sounds: {
    phone: 'This track has few high sounds. A brighter song works better.',
    pc: 'This track has few high sounds. A brighter song works better.',
  },
  gaps: {
    phone: 'This track has many quiet moments.',
    pc: 'This track has many quiet moments.',
  },
  other_sounds: {
    phone: 'Pause other sounds while the test sound plays.',
    pc: 'Pause other sounds while the sound check runs.',
  },
  unclear_echo: {
    phone: "The echo is unclear. Keep the phone's speaker and microphones uncovered.",
    pc: 'The echo is unclear. Move the microphone closer to where you poke.',
  },
  noisy_room: {
    phone: 'Sudden noises near the phone make the echo hard to read.',
    pc: 'Sudden noises near the microphone make the echo hard to read.',
  },
  pc_bluetooth: {
    phone: 'Bluetooth speakers add delay. Built-in or wired speakers work better.',
    pc: 'Bluetooth speakers add delay. Built-in or wired speakers work better.',
  },
  ready: { phone: 'Ready. Poke toward your phone.', pc: 'Ready. Poke toward your microphone.' },
};
export const tipIds = Object.keys(tips) as TipId[];
export function tipText(tip: TipId, pc: boolean): string {
  const text = tips[tip] as (typeof tips)[TipId] | undefined;
  return text ? text[pc ? 'pc' : 'phone'] : '';
}

// C.2 for the Live tab, plus C.10 for engine feedback the plan leaves unnamed.
export const liveCopy = {
  soundQuality: 'Sound quality',
  labels: {
    great: 'Great',
    good: 'Good',
    fair: 'Fair',
    poor: 'Poor',
    none: 'No sound',
  } satisfies Record<Meter['label'], string>,
  high: 'High sounds',
  volume: 'Volume',
  steady: 'Steady',
  clarity: 'Clear echo',
  detail: (cm: string) => `Detail ≈ ${cm} cm`,
  handSeen: (cm: number | string) => `Hand seen at ${cm} cm`,
  poke: (cm: number | string) => `Poke · ${cm} cm`,
  test: 'test',
  marker: 'poke',
  testPokes: 'Test pokes',
  pokeTestSoon: 'The poke test is coming soon.',
  waiting: 'Waiting for sound…',
  aligning: 'Finding the echo…',
  settling: 'Learning the room…',
  weak: 'Sound too weak to detect pokes',
  paused: 'Paused',
  source: 'Waiting for sound Veyl can use',
  running: 'Watching for pokes',
  relocked: 'The echo moved. Veyl found it again.',
  modelJump: 'The room changed. Veyl is learning it again.',
  drift: 'The sound and the microphone keep slightly different time. Veyl is adjusting.',
  pokeDelay: 'Poke delay (median / p95)',
  pokesMeasured: 'Pokes measured',
  missing: '—',
} as const;

/** What the engine is doing, in words: a suspension reason first, then the state. */
export function engineStatus(engine: Pick<EngineView, 'state' | 'suspended'>): string {
  switch (engine.suspended) {
    case 'paused':
      return liveCopy.paused;
    case 'source':
      return liveCopy.source;
    case 'sound':
      return liveCopy.weak;
    case 'settling':
      return liveCopy.settling;
  }
  switch (engine.state) {
    case 'aligning':
      return liveCopy.aligning;
    case 'settling':
      return liveCopy.settling;
    case 'running':
      return liveCopy.running;
    default:
      return liveCopy.waiting;
  }
}
export function warningText(warning: EngineWarning | null, pc: boolean): string {
  switch (warning) {
    case 'relocked':
      return liveCopy.relocked;
    case 'model_jump':
      return liveCopy.modelJump;
    case 'impulsive_noise':
      return tipText('noisy_room', pc);
    case 'drift':
      return liveCopy.drift;
    default:
      return '';
  }
}

export const recordingCopy = {
  title: 'Record a test',
  consent: (seconds: number) =>
    `Veyl saves the next ${seconds} seconds of what it hears, so you can check how well it works. Recordings stay on this PC.`,
  markPokes: 'Mark pokes with the Space key',
  duration: 'Duration',
  seconds: (seconds: number) => `${seconds} seconds`,
  start: 'Start recording',
  stop: 'Stop recording',
  banner: (seconds: number) => `Recording · ${seconds} s left`,
  spaceHint: 'Press Space each time you poke',
  saved: 'Saved. Recordings stay on this PC.',
  partial: "Recording didn't finish. You can open or delete the partial files.",
  open: 'Open folder',
  delete: 'Delete',
  deleteTitle: 'Delete this recording?',
  deleteMessage: 'This permanently deletes this test from this PC.',
  recordings: 'Test recordings',
  empty: 'No test recordings yet.',
  notListening: 'Start listening before recording a test.',
  saveFailed: "Veyl couldn't save this test. Try again.",
  loadFailed: "Veyl couldn't load test recordings. Try again.",
  deleteFailed: "Veyl couldn't delete this recording. Try again.",
  openFailed: "Veyl couldn't open the recordings folder.",
} as const;

export const linkCopy = {
  yourPhones: 'Your phones',
  nearby: 'Nearby',
  pair: 'Pair',
  forget: 'Forget',
  connect: 'Connect',
  disconnect: 'Disconnect',
  cancel: 'Cancel',
  addressTitle: 'Add by address',
  addressHint: 'The address is shown on your phone in Veyl.',
  addressExample: '192.168.1.23:47820',
  codeInstructions: 'Check that your phone shows the same code:',
  codesMatch: 'Codes match',
  waitingPhone: 'Waiting for you to confirm on your phone…',
  pairTitle: (phone: string) => `Pair with ${phone}`,
  forgetTitle: (phone: string) => `Forget ${phone}?`,
  forgetMessage: (phone: string) =>
    `Veyl on this PC will stop connecting to ${phone}. You can pair again any time.`,
  pairedWith: (phone: string) => `Paired with ${phone}`,
  lastSeen: (time: string) => `Last seen ${time}`,
  connecting: (phone: string) => `Connecting to ${phone}…`,
  pairing: (phone: string) => `Pairing with ${phone}…`,
  reconnecting: (phone: string) => `Reconnecting to ${phone}…`,
  connectedIdle: (phone: string) => `${phone} connected · tap Start on your phone`,
  listening: (phone: string) => `Listening through ${phone}`,
  found: 'Phone found · pair it to start',
  unreachable: "Can't reach your phone",
  failedRequest: "Veyl couldn't complete that request. Try again.",
  askPhoneToStart: 'Ask phone to start',
  diagnostics: 'Diagnostics',
  soundCopy: 'Sound copy',
  microphone: 'Microphone',
  beforeFilter: 'Before filter',
  afterFilter: 'After filter',
  dropped: 'Dropped blocks',
  engineDrops: 'Engine drops',
  roundTrip: 'Round trip',
  arrivalDelay: 'Arrival delay',
  phoneQueue: 'Phone queue',
  sampleRate: 'Sample rate',
  battery: 'Phone battery',
  unavailable: 'Not available',
  copyDiagnostics: 'Copy diagnostics',
  copied: 'Diagnostics copied.',
  copyFailed: 'Veyl could not copy diagnostics.',
  blocksPerSecond: (name: string, n: string) => `${name}: ${n} blocks/s`,
  counts: (blocks: string, gaps: string) => `${blocks} blocks · ${gaps} gaps`,
} as const;

export function connectionError(code: string | null | undefined, pairing = false): string {
  switch (code) {
    case null:
    case undefined:
    case 'user_cancelled':
      return '';
    case 'not_in_foreground':
    case 'pairing_disabled':
      return 'Open Veyl on your phone, then try again.';
    case 'rate_limited':
      return 'Too many tries. Wait a few minutes, then try again.';
    case 'busy':
      return 'Your phone is connected to another PC.';
    case 'version':
      return 'Update Veyl on your phone and PC so both have the same version.';
    case 'not_paired':
    case 'bad_message':
      return "Couldn't verify your phone. If you reinstalled Veyl on it, pair again.";
    case 'pair_store_write_failed':
      return "Veyl couldn't save paired devices. Try again.";
    case 'pair_store_read_failed':
      return "Veyl couldn't load paired devices. Try again.";
    case 'invalid_address':
      return 'Enter the address shown on your phone, including its port.';
    default:
      return pairing ? "Pairing didn't finish. Try again." : "Can't reach your phone";
  }
}
