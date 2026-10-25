// Wording comes from docs/ui-copy.md.
import type {
  DistanceRange,
  EngineView,
  EngineWarning,
  Meter,
  PokeAction,
  Sensitivity,
  TipId,
} from './api/types';

export const copy = {
  status: 'Looking for your phone…',
  preview: 'Preview — sample data',
  echoTitle: 'What Veyl hears',
  echoEmpty: 'Move your hand above your phone to see it here.',
  echoAxis: 'Distance from phone',
  noHand: 'No hand seen',
  noSound: 'No sound',
  pokesTitle: 'Recent pokes',
  pokesEmpty: 'Pokes appear here.',
  phonesEmpty: 'Open Veyl on your phone to see it here.',
  loadFailed: 'Veyl could not load its settings.',
  windowFailed: 'Veyl could not open its window.',
  brandLink: 'Veyl live view',
  navigation: 'Veyl views',
  tagline: 'Touch the air. Your screen follows.',
  footer: 'Sound stays home.',
  close: 'Close',
} as const;

/** A hotkey as Windows users know it: the Windows key instead of "Super", "1" for "Digit1". */
export function hotkeyLabel(hotkey: string): string {
  return hotkey
    .split('+')
    .map((part) => (part === 'Super' ? 'Win' : part.replace(/^Digit(\d)$/, '$1')))
    .join('+');
}

// C.2: the actions switch in the header, its tooltip and toasts, and §7.7's blocked click.
export const actionsCopy = {
  name: 'Actions',
  on: 'On',
  off: 'Off',
  toastOn: 'Actions on',
  toastOff: 'Actions off',
  tooltip: (hotkey: string) =>
    `When actions are on, each poke clicks where your pointer is. Press ${hotkeyLabel(hotkey)} to switch them on or off.`,
  blocked: 'Windows blocked the click in an app running as administrator.',
} as const;

export const settingsCopy = {
  title: 'Settings',
  pokeAction: 'Poke action',
  pokeActions: {
    click: 'Click',
    play_pause: 'Play or pause',
    next_track: 'Next track',
    space: 'Space bar',
    none: 'Nothing',
  } satisfies Record<PokeAction, string>,
  sensitivity: 'Sensitivity',
  sensitivities: { low: 'Low', normal: 'Normal', high: 'High' } satisfies Record<
    Sensitivity,
    string
  >,
  range: 'Distance range',
  ranges: { near: 'Near', normal: 'Normal', far: 'Far' } satisfies Record<DistanceRange, string>,
  hotkey: 'Actions hotkey',
  change: 'Change',
  recordHotkey: 'Press a key combination, or Esc to cancel.',
  needsModifier: 'Use Ctrl, Alt, Shift or the Windows key together with another key.',
  unsupportedKey: "Veyl can't use this key. Choose another one.",
  actionsOnAtStart: 'Turn actions on when Veyl starts',
  startWithWindows: 'Start Veyl with Windows',
  closeToTray: 'Keep Veyl running in the tray when I close the window',
  pcName: 'PC name',
  advanced: 'Advanced',
  temperature: 'Room temperature',
  temperatureNote: 'Veyl uses it to measure distance.',
  celsius: '°C',
  detailedLogs: 'Detailed logs',
  detailedLogsNote: 'Detailed logs start the next time Veyl starts.',
  keepRecordings: 'Keep test recordings for 7 days',
  keepRecordingsNote: 'When this is off, test recordings stay until you delete them.',
} as const;
/** A `set_settings` error code in words; anything unexpected is a general failure. */
export function settingsError(code: unknown): string {
  switch (code) {
    case 'invalid_pc_name':
      return 'Enter a PC name of up to 40 characters.';
    case 'invalid_hotkey':
      return settingsCopy.needsModifier;
    case 'hotkey_unavailable':
      return 'Another app already uses this key combination. Choose another one.';
    case 'invalid_temperature':
      return 'Enter a room temperature from -10 to 45 °C.';
    case 'autostart_failed':
      return "Windows didn't let Veyl change starting with Windows. Try again.";
    case 'engine_busy':
    case 'engine_stopped':
      return "Veyl saved this setting but couldn't apply it yet. Restart Veyl to apply it.";
    default:
      return "Veyl couldn't save your settings. Try again.";
  }
}

export const pokeTestCopy = {
  title: 'Test pokes',
  instructions: 'Poke 10 times, about once a second.',
  counter: (seen: number, of = 10) => `${seen} of ${of}`,
  result: (seen: number, of = 10) => `Veyl saw ${seen} of ${of} pokes.`,
  few: 'Try a brighter song, more volume, or Sensitivity: High.',
  some: 'Good. Sensitivity: High may catch more.',
  most: 'Great.',
  failed: "Veyl couldn't start the poke test. Try again.",
  cancel: 'Cancel',
} as const;
/** Advice after a poke test: fewer than 6 pokes, 6 to 8, or 9 and 10. */
export function pokeAdvice(seen: number): string {
  return seen >= 9 ? pokeTestCopy.most : seen >= 6 ? pokeTestCopy.some : pokeTestCopy.few;
}

// C.2 onboarding 1–7, with the PC path's two steps.
export const onboardingCopy = {
  welcomeTitle: 'Welcome to Veyl',
  welcome: 'Control your PC by poking the air. Veyl listens to the echo of the music you play.',
  start: 'Get started',
  next: 'Next',
  back: 'Back',
  finish: 'Finish',
  phoneAppTitle: 'Get Veyl on your phone',
  phoneApp:
    'On your Android phone, open jacobbalane.github.io/Veyl and tap Download for Android. Then open Veyl.',
  website: 'jacobbalane.github.io/Veyl',
  pairTitle: 'Pair your phone',
  pair: 'Your phone appears here when Veyl is open on it.',
  placeTitle: 'Place your phone',
  place:
    'Lay it flat, screen up, in front of your monitor. Keep its speaker and microphones uncovered. Set the volume to about half or more.',
  playTitle: 'Play something',
  phonePlay:
    'Play a song or video on your phone — YouTube or YouTube Music work best — then tap Start in Veyl on the phone.',
  pcDevicesTitle: 'Choose speakers and a microphone',
  pcPlay: 'Play a song or video on this PC. Any app works.',
  practiceTitle: 'Try a poke',
  practicePhone: 'Hold your hand about 20 cm above your phone. Poke quickly toward it and back.',
  practicePc:
    'Hold your hand about 20 cm in front of your microphone. Poke quickly toward it and back.',
  nice: 'Nice! Veyl saw that.',
  actionsTitle: "Turn on actions when you're ready",
  actions: (hotkey: string) =>
    `When actions are on, each poke clicks where your pointer is. Press ${hotkeyLabel(hotkey)} any time to switch them on or off.`,
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
  screenLocked:
    'Your phone stopped listening because its screen locked. Open Veyl on your phone and tap Start.',
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

export const links = {
  website: 'https://jacobbalane.github.io/Veyl/',
  troubleshooting: 'https://jacobbalane.github.io/Veyl/docs/#troubleshooting',
  source: 'https://github.com/jacobbalane/Veyl',
  licenses: 'https://github.com/jacobbalane/Veyl/blob/main/THIRD_PARTY_NOTICES.md',
} as const;

export const helpCopy = {
  placementPhone: 'Placing your phone',
  // C.3's placement guide, shared with the phone app.
  placementPhoneSteps: [
    'Lay it flat, screen up, in front of your monitor.',
    'Keep the speaker and microphones uncovered.',
    'Set the volume to about half or more.',
    'Hold your hand 10–30 cm above it.',
  ],
  troubleshooting: "If something isn't working",
  more: 'More help on the Veyl website',
  privacy: 'Privacy',
  privacySummary: 'Sound stays on your devices. Veyl has no accounts, analytics or cloud services.',
  privacyDetails: [
    'With your phone, sound goes only to your paired PC, encrypted, after Veyl removes the low sounds where voices are.',
    'With this PC, sound never leaves this PC.',
    'Veyl saves sound only when you record a test. Test recordings stay on this PC.',
    'Veyl goes online only to open a link you choose.',
  ],
  diagnostics: 'Diagnostics',
  openLogs: 'Open logs',
  logsFailed: "Veyl couldn't open the logs folder.",
  about: 'About',
  version: (version: string) => `Version ${version}`,
  website: 'Website',
  source: 'Source code',
  licenses: 'Licenses',
  license: 'Free and open source (MIT)',
} as const;

// Help's fix for each tip. A fix that fits both modes is given once.
const tipFixes: Record<TipId, { phone: string; pc?: string }> = {
  capture_off: {
    phone:
      "Open Veyl on your phone, play something and tap Start. Keep the phone's screen on while Veyl listens.",
    pc: 'In Settings, set Sound source to With this PC and choose your speakers and microphone. Default device follows Windows.',
  },
  not_speaker: {
    phone:
      'Disconnect headphones and Bluetooth speakers from your phone, so the sound plays from its own speaker.',
  },
  pc_headphones: {
    phone:
      "Switch Windows' sound to speakers, or choose speakers in Settings. Veyl can't hear echoes from headphones.",
  },
  pc_no_mic: {
    phone: "Connect a microphone, or choose one in Settings. A laptop's built-in microphone works.",
  },
  capture_blocked: {
    phone:
      "Some apps don't let other apps copy their sound. Play YouTube, YouTube Music, the Veyl player or the test sound instead.",
    pc: 'Protected videos and some streaming apps share only silence. Play music or a video from another app or site.',
  },
  nothing_playing: {
    phone:
      'Start music or a video and keep it playing. Veyl sees your hand in the echo of that sound.',
  },
  mic_silenced: {
    phone:
      'Close calls, voice recorders and other apps that use the microphone, then tap Start again.',
    pc: 'Close calls and other apps that use the microphone, or choose another microphone in Settings.',
  },
  clipping: {
    phone: 'Lower the volume until the sound is clean. Distorted sound makes a messy echo.',
  },
  volume_low: { phone: 'Set the volume to about half or more. Quiet sound makes a weak echo.' },
  few_high_sounds: {
    phone:
      'Choose a song with cymbals, hi-hats or bright synths. Veyl hears your hand through high sounds.',
  },
  gaps: { phone: "Choose a song without long quiet parts. Veyl can't see your hand in silence." },
  other_sounds: {
    phone: 'Pause other music and videos until the test sound or the sound check ends.',
  },
  unclear_echo: {
    phone:
      'Take the phone out of a thick case, lay it flat and keep its speaker and microphones uncovered.',
    pc: 'Move the microphone closer to where you poke, with nothing between it and your hand.',
  },
  noisy_room: {
    phone:
      'Wait for typing, tapping or talking nearby to stop. Veyl ignores pokes during sudden noises.',
  },
  pc_bluetooth: {
    phone:
      'Use built-in or wired speakers if you can. Bluetooth speakers still work, with a little more delay.',
  },
  ready: {
    phone:
      "If pokes don't register, try Test pokes on the Live tab, or Sensitivity: High in Settings.",
  },
};
export function tipFix(tip: TipId, pc: boolean): string {
  const fix = tipFixes[tip] as (typeof tipFixes)[TipId] | undefined;
  return fix ? ((pc ? fix.pc : undefined) ?? fix.phone) : '';
}
/** The tips Help lists in each mode; tips that only the other mode shows are left out. */
export function helpTips(pc: boolean): TipId[] {
  const other: TipId[] = pc ? ['not_speaker'] : ['pc_headphones', 'pc_no_mic', 'pc_bluetooth'];
  return tipIds.filter((tip) => !other.includes(tip));
}
/** Help's entries for the errors in C.2, C.7 and C.9, each with a fix. */
export const helpErrors: readonly {
  problem: string;
  fix: string;
  /** Shown only in this mode. */
  mode?: 'phone' | 'pc';
}[] = [
  {
    problem: "Can't reach your phone",
    fix: 'Check that your phone and PC use the same network and that Veyl is open on the phone. You can also enter the address shown on your phone on the Phones tab.',
    mode: 'phone',
  },
  {
    problem: 'Open Veyl on your phone, then try again.',
    fix: "Pairing works only while Veyl is open on your phone's screen.",
    mode: 'phone',
  },
  {
    problem: 'Too many tries. Wait a few minutes, then try again.',
    fix: "After several pairing tries that didn't finish, Veyl pauses pairing for a few minutes to keep other people out.",
    mode: 'phone',
  },
  {
    problem: "Pairing didn't finish. Try again.",
    fix: 'Keep Veyl open on your phone while you pair, and check that both screens show the same code.',
    mode: 'phone',
  },
  {
    problem: 'Your phone is connected to another PC.',
    fix: 'Disconnect it on the other PC or close Veyl there, then connect again.',
    mode: 'phone',
  },
  {
    problem: "Couldn't verify your phone. If you reinstalled Veyl on it, pair again.",
    fix: 'Forget the phone on the Phones tab, then pair it again.',
    mode: 'phone',
  },
  {
    problem: 'Update Veyl on your phone and PC so both have the same version.',
    fix: 'Download the latest Veyl for both from the Veyl website.',
    mode: 'phone',
  },
  {
    problem:
      'Your phone stopped listening because its screen locked. Open Veyl on your phone and tap Start.',
    fix: 'Keep Veyl open on your phone while you use it, or set a longer screen timeout.',
    mode: 'phone',
  },
  {
    problem: 'Windows blocked the click in an app running as administrator.',
    fix: "Windows doesn't let apps click in apps that run as administrator. Use Veyl with other apps, or open that app without administrator rights.",
  },
  {
    problem: "Veyl couldn't open these audio devices. Check them in Settings.",
    fix: 'Reconnect the device or choose another one in Settings, and close apps that use it on their own.',
    mode: 'pc',
  },
];
