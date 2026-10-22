import { expect, test } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { Markdown, safeHref } from './markdown';

test('Markdown preserves code, tables and lists while escaping source HTML', () => {
  const html = renderToStaticMarkup(
    <Markdown
      text={
        '# Heading\n\n<script>alert(1)</script>\n\n- **One**\n- `two`\n\n| Name | Value |\n| --- | --- |\n| Test | 42 |\n\n```text\n## code heading\n```'
      }
    />,
  );
  expect(html).toContain('<h2>Heading</h2>');
  expect(html).toContain('&lt;script&gt;');
  expect(html).not.toContain('<script>');
  expect(html).toContain('<strong>One</strong>');
  expect(html).toContain('<table>');
  expect(html).toContain('<pre><code>## code heading</code></pre>');
});

test('documentation links resolve within the site and unsafe URLs are rejected', () => {
  expect(safeHref('privacy.md')).toBe('#privacy');
  expect(safeHref('progress.md')).toBe(
    'https://github.com/jacobbalane/Veyl/blob/main/docs/progress.md',
  );
  expect(safeHref('javascript:alert(1)')).toBeUndefined();
  expect(safeHref('//untrusted.example')).toBeUndefined();
  expect(safeHref('https://github.com/jacobbalane/Veyl')).toBe(
    'https://github.com/jacobbalane/Veyl',
  );
});
