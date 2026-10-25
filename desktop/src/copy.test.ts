// @vitest-environment node
import { readFileSync } from 'node:fs';
import { expect, test } from 'vitest';
import { copy, liveCopy, pcCopy, recordingCopy, tipIds, tipText } from './copy';

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
