// @vitest-environment node
import { expect, test } from 'vitest';
import { hotkeyLabel } from '../copy';
import { isHotkey, readHotkey } from './hotkey';

const press = (
  code: string,
  modifiers: Partial<Record<'ctrl' | 'alt' | 'shift' | 'meta', boolean>> = {},
) =>
  readHotkey({
    code,
    ctrlKey: !!modifiers.ctrl,
    altKey: !!modifiers.alt,
    shiftKey: !!modifiers.shift,
    metaKey: !!modifiers.meta,
  });

test('a key combination becomes the form the Rust shortcut parser accepts', () => {
  expect(press('KeyV', { ctrl: true, alt: true })).toEqual({
    kind: 'hotkey',
    hotkey: 'Ctrl+Alt+V',
  });
  expect(press('F9', { ctrl: true, shift: true })).toEqual({
    kind: 'hotkey',
    hotkey: 'Ctrl+Shift+F9',
  });
  expect(press('Space', { alt: true })).toEqual({ kind: 'hotkey', hotkey: 'Alt+Space' });
  expect(press('Digit1', { meta: true, shift: true })).toEqual({
    kind: 'hotkey',
    hotkey: 'Shift+Super+Digit1',
  });
  // Modifiers always come in the same order, whichever was pressed first.
  expect(press('KeyK', { shift: true, alt: true, ctrl: true, meta: true })).toEqual({
    kind: 'hotkey',
    hotkey: 'Ctrl+Alt+Shift+Super+K',
  });
  for (const hotkey of ['Ctrl+Alt+V', 'Ctrl+Shift+F9', 'Alt+Space', 'Shift+Super+Digit1'])
    expect(isHotkey(hotkey), hotkey).toBe(true);
});

test('Escape cancels, modifiers wait, and a bare or unknown key is refused', () => {
  expect(press('Escape')).toEqual({ kind: 'cancel' });
  expect(press('Escape', { ctrl: true })).toEqual({ kind: 'cancel' });
  expect(press('ControlLeft', { ctrl: true })).toEqual({ kind: 'waiting' });
  expect(press('AltRight', { alt: true, ctrl: true })).toEqual({ kind: 'waiting' });
  expect(press('MetaLeft', { meta: true })).toEqual({ kind: 'waiting' });
  expect(press('KeyV')).toEqual({ kind: 'bare' });
  expect(press('F9')).toEqual({ kind: 'bare' });
  expect(press('Tab')).toEqual({ kind: 'bare' });
  expect(press('IntlBackslash', { ctrl: true })).toEqual({ kind: 'unsupported' });
  expect(press('ContextMenu', { ctrl: true })).toEqual({ kind: 'unsupported' });
  expect(press('F25', { ctrl: true })).toEqual({ kind: 'unsupported' });
  for (const hotkey of ['V', 'Ctrl+', 'Ctrl+Alt', 'Ctrl+Ctrl+V', 'Ctrl+KeyV', 'Hyper+V', ''])
    expect(isHotkey(hotkey), hotkey).toBe(false);
});

test('hotkeys read as Windows users know them', () => {
  expect(hotkeyLabel('Ctrl+Alt+V')).toBe('Ctrl+Alt+V');
  expect(hotkeyLabel('Super+Shift+Digit1')).toBe('Win+Shift+1');
});
