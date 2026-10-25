// Turns a key press into a hotkey string the Rust side accepts: modifiers `Ctrl`, `Alt`, `Shift`
// and `Super` joined with `+`, then one key named after `KeyboardEvent.code` ("V", "F9",
// "Space", "Digit1"). Rust's shortcut parser reads these names without regard to case.

/** Keys besides letters and F1–F24 that the shortcut parser knows, by their `code`. */
const namedKeys = new Set([
  ...Array.from({ length: 10 }, (_, digit) => `Digit${digit}`),
  ...Array.from({ length: 10 }, (_, digit) => `Numpad${digit}`),
  'Backquote',
  'Backslash',
  'BracketLeft',
  'BracketRight',
  'Comma',
  'Equal',
  'Minus',
  'Period',
  'Quote',
  'Semicolon',
  'Slash',
  'Backspace',
  'Enter',
  'Space',
  'Tab',
  'Delete',
  'End',
  'Home',
  'Insert',
  'PageDown',
  'PageUp',
  'PrintScreen',
  'Pause',
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
  'ArrowUp',
  'NumpadAdd',
  'NumpadDecimal',
  'NumpadDivide',
  'NumpadEnter',
  'NumpadMultiply',
  'NumpadSubtract',
]);
const modifierKeys = new Set([
  'ControlLeft',
  'ControlRight',
  'AltLeft',
  'AltRight',
  'ShiftLeft',
  'ShiftRight',
  'MetaLeft',
  'MetaRight',
  'OSLeft',
  'OSRight',
]);

export type HotkeyPress =
  /** Escape: stop recording and keep the current hotkey. */
  | { kind: 'cancel' }
  /** Only modifiers so far; keep listening. */
  | { kind: 'waiting' }
  /** A key without Ctrl, Alt, Shift or the Windows key. */
  | { kind: 'bare' }
  /** A key the shortcut parser doesn't know. */
  | { kind: 'unsupported' }
  | { kind: 'hotkey'; hotkey: string };

type Press = Pick<KeyboardEvent, 'code' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>;

/** The key's name in a hotkey string, or null when Veyl can't use it. */
export function hotkeyKey(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return letter[1];
  if (/^F([1-9]|1\d|2[0-4])$/.test(code)) return code;
  return namedKeys.has(code) ? code : null;
}

export function readHotkey(press: Press): HotkeyPress {
  if (press.code === 'Escape') return { kind: 'cancel' };
  if (modifierKeys.has(press.code)) return { kind: 'waiting' };
  const key = hotkeyKey(press.code);
  if (!key) return { kind: 'unsupported' };
  const modifiers = [
    press.ctrlKey && 'Ctrl',
    press.altKey && 'Alt',
    press.shiftKey && 'Shift',
    press.metaKey && 'Super',
  ].filter((modifier): modifier is string => !!modifier);
  if (modifiers.length === 0) return { kind: 'bare' };
  return { kind: 'hotkey', hotkey: [...modifiers, key].join('+') };
}

/** Whether a hotkey in the form `readHotkey` writes has modifiers and one key Veyl can use. */
export function isHotkey(hotkey: string): boolean {
  const parts = hotkey.split('+');
  const key = parts.pop() ?? '';
  return (
    parts.length > 0 &&
    new Set(parts).size === parts.length &&
    parts.every((part) => ['Ctrl', 'Alt', 'Shift', 'Super'].includes(part)) &&
    (/^[A-Z]$/.test(key) || (hotkeyKey(key) != null && !key.startsWith('Key')))
  );
}
