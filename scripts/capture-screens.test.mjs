import { test } from 'node:test';
import assert from 'node:assert/strict';
import { shotUrl, shots } from './capture-screens.mjs';

test('screens have unique names and address only the preview with known options', () => {
  const options = new Set([
    'state',
    'suspended',
    'warning',
    'tip',
    'score',
    'mode',
    'connection',
    'hand',
    'tab',
    'dialog',
    'onboarding',
    'actions',
  ]);
  assert.equal(new Set(shots.map((shot) => shot.name)).size, shots.length);
  for (const shot of shots) {
    assert.match(shot.name, /^[a-z0-9-]+$/);
    assert.ok(['dark', 'light'].includes(shot.scheme));
    const url = new URL(shotUrl(shot));
    assert.equal(url.origin, 'http://127.0.0.1:1421');
    for (const key of url.searchParams.keys()) assert.ok(options.has(key), key);
  }
});
