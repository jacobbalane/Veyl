// @vitest-environment node
import { readFileSync } from 'node:fs';
import { expect, test } from 'vitest';
import {
  actionsCopy,
  copy,
  helpCopy,
  helpErrors,
  linkCopy,
  liveCopy,
  onboardingCopy,
  pcCopy,
  pokeAdvice,
  pokeTestCopy,
  recordingCopy,
  settingsCopy,
  settingsError,
  tipFix,
  tipIds,
  tipText,
} from './copy';

const uiCopy = readFileSync(new URL('../../docs/ui-copy.md', import.meta.url), 'utf8');

test('every tip has text in both modes', () => {
  expect(tipIds).toHaveLength(16);
  for (const tip of tipIds)
    for (const pc of [false, true]) {
      const text = tipText(tip, pc);
      expect(text, `${tip} ${pc ? 'PC' : 'phone'}`).not.toBe('');
      expect(uiCopy).toContain(text);
    }
  expect(tipText('not_speaker', true)).toBe(tipText('pc_headphones', true));
  expect(tipText('ready', false)).toBe('Ready. Poke toward your phone.');
  expect(tipText('ready', true)).toBe('Ready. Poke toward your microphone.');
});

test('the Live tab and Diagnostics use the words in docs/ui-copy.md', () => {
  const { labels, detail, handSeen, poke, ...plain } = liveCopy;
  const used = [
    ...Object.values(plain),
    ...Object.values(labels),
    detail('{x}'),
    handSeen('{d}'),
    poke('{d}'),
    copy.echoTitle,
    copy.echoAxis,
    copy.echoEmpty,
    copy.noHand,
    copy.pokesTitle,
    copy.pokesEmpty,
    copy.preview,
    pcCopy.axis,
    pcCopy.empty,
    recordingCopy.title,
  ];
  for (const text of used) expect(uiCopy, text).toContain(text);
});

test('actions, settings, the poke test, onboarding and Help use the words in docs/ui-copy.md', () => {
  const strings = (group: object): string[] =>
    Object.values(group).flatMap((value) =>
      typeof value === 'string'
        ? [value]
        : Array.isArray(value)
          ? value
          : typeof value === 'object'
            ? strings(value as object)
            : [],
    );
  const errorCodes = [
    'invalid_pc_name',
    'invalid_hotkey',
    'hotkey_unavailable',
    'invalid_temperature',
    'autostart_failed',
    'engine_busy',
    'settings_save_failed',
    'something_new',
  ];
  const used = [
    ...strings(actionsCopy),
    actionsCopy.tooltip('Ctrl+Alt+V'),
    ...strings(settingsCopy),
    ...errorCodes.map(settingsError),
    ...strings(pokeTestCopy),
    '{n} of 10',
    'Veyl saw {n} of 10 pokes.',
    ...[0, 6, 9].map(pokeAdvice),
    ...strings(onboardingCopy),
    onboardingCopy.actions('Ctrl+Alt+V'),
    ...strings(helpCopy),
    helpCopy.version('{v}'),
    ...tipIds.flatMap((tip) => [tipFix(tip, false), tipFix(tip, true)]),
    ...helpErrors.flatMap(({ problem, fix }) => [problem, fix]),
    linkCopy.screenLocked,
    copy.loadFailed,
    copy.windowFailed,
    copy.close,
    copy.tagline,
    copy.footer,
  ];
  for (const text of used) expect(uiCopy, text).toContain(text);
  // The tooltip and onboarding name the hotkey from Settings.
  expect(actionsCopy.tooltip('Super+Shift+Digit1')).toContain('Press Win+Shift+1 to switch');
  expect(settingsError('engine_stopped')).toBe(settingsError('engine_busy'));
  expect(pokeTestCopy.counter(7)).toBe('7 of 10');
  expect(pokeTestCopy.result(7)).toBe('Veyl saw 7 of 10 pokes.');
});
