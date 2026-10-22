import assert from 'node:assert/strict';
import test from 'node:test';
import { changelogSection } from './changelog-section.mjs';

test('extracts one release body and retains subsections, Markdown and newlines', () => {
  const text =
    '# Changelog\r\n\r\n## Unreleased\r\n\r\nFuture.\r\n\r\n## 0.1.0 (2026-11-01)\r\n\r\nPoke to click.\r\n\r\n### New\r\n\r\n- **Pokes.** Local control.\r\n\r\n## 0.0.1 (2026-10-21)\r\n\r\nPreview.\r\n';
  assert.equal(
    changelogSection(text, '0.1.0'),
    'Poke to click.\n\n### New\n\n- **Pokes.** Local control.\n',
  );
  assert.equal(changelogSection(text, '0.0.1'), 'Preview.\n');
});

test('rejects absent, empty, duplicate and partial version headings', () => {
  for (const text of [
    '## 0.1.01 (2026-11-01)\nWrong version.',
    '## 0.1.0 (2026-11-01)\n\n## 0.0.1 (2026-10-21)\nPreview.',
    '## 0.1.0 (2026-11-01)\n\n### New\n<!-- pending -->',
    '## 0.1.0 (2026-11-01)\nOne.\n\n## 0.1.0 (2026-11-02)\nTwo.',
  ])
    assert.throws(() => changelogSection(text, '0.1.0'));
});

test('release-looking headings inside fenced examples do not split release notes', () => {
  for (const fence of ['```', '~~~~']) {
    const body = `Notes.\n\n${fence}markdown\n## 0.0.1 (2026-10-21)\nAn example.\n${fence}\n\nMore notes.`;
    const text = `## 0.1.0 (2026-11-01)\n\n${body}\n\n## 0.0.1 (2026-10-21)\nPreview.`;
    assert.equal(changelogSection(text, '0.1.0'), `${body}\n`);
  }
});
