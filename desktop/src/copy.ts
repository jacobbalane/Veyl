// Wording comes from docs/ui-copy.md; the connection preview is recorded in decision 0006.
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
