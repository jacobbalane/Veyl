import { Fragment, type ReactNode } from 'react';

const DOCUMENTS = [
  'getting-started',
  'placement',
  'meter',
  'troubleshooting',
  'privacy',
  'how-it-works',
  'development',
];
export function safeHref(href: string): string | undefined {
  if (/^(https?:\/\/|#[\w-])/i.test(href)) return href;
  const doc = /^([\w-]+)\.md(?:#([\w-]+))?$/.exec(href);
  if (doc && DOCUMENTS.includes(doc[1])) return `#${doc[1]}`;
  if (/^(?:[\w.-]+\/)*[\w.-]+\.md(?:#[\w-]+)?$/.test(href))
    return `https://github.com/jacobbalane/Veyl/blob/main/docs/${href}`;
  return undefined;
}

function inline(text: string): ReactNode[] {
  const pattern = /(\[([^\]]+)\]\(([^)]+)\)|\*\*([^*]+)\*\*|`([^`]+)`)/g;
  const parts: ReactNode[] = [];
  let cursor = 0;
  for (const match of text.matchAll(pattern)) {
    parts.push(text.slice(cursor, match.index));
    if (match[2]) {
      const href = safeHref(match[3]);
      parts.push(
        href ? (
          <a key={match.index} href={href}>
            {match[2]}
          </a>
        ) : (
          match[2]
        ),
      );
    } else if (match[4]) parts.push(<strong key={match.index}>{match[4]}</strong>);
    else parts.push(<code key={match.index}>{match[5]}</code>);
    cursor = match.index + match[0].length;
  }
  parts.push(text.slice(cursor));
  return parts;
}
function cells(line: string) {
  return line
    .trim()
    .replace(/^\||\|$/g, '')
    .split(/(?<!\\)\|/)
    .map((cell) => cell.trim().replaceAll('\\|', '|'));
}
const isBlock = (line: string) => /^(#{1,6}\s|```|~~~|\s*[-*]\s|\s*\d+\.\s|\s*>|\s*\|)/.test(line);

// React escapes every text node; source HTML is never inserted into the page.
export function Markdown({ text }: { text: string }) {
  const lines = text.replaceAll('\r\n', '\n').split('\n');
  const blocks: ReactNode[] = [];
  let index = 0;
  while (index < lines.length) {
    const start = index;
    const line = lines[index];
    if (!line.trim() || /^---+$/.test(line)) {
      index++;
      continue;
    }
    const fence = /^(```|~~~)/.exec(line);
    if (fence) {
      const content: string[] = [];
      index++;
      while (index < lines.length && !lines[index].startsWith(fence[1]))
        content.push(lines[index++]);
      index++;
      blocks.push(
        <pre key={start}>
          <code>{content.join('\n')}</code>
        </pre>,
      );
      continue;
    }
    const heading = /^(#{1,6})\s+(.*)$/.exec(line);
    if (heading) {
      const title = inline(heading[2]);
      const level = Math.min(heading[1].length + 1, 6);
      blocks.push(
        level === 2 ? (
          <h2 key={start}>{title}</h2>
        ) : level === 3 ? (
          <h3 key={start}>{title}</h3>
        ) : (
          <h4 key={start}>{title}</h4>
        ),
      );
      index++;
      continue;
    }
    if (
      line.trim().startsWith('|') &&
      /^\|?[\s:|-]+\|[\s:|-]*$/.test(lines[index + 1]?.trim() ?? '')
    ) {
      const headers = cells(line);
      index += 2;
      const rows: string[][] = [];
      while (index < lines.length && lines[index].trim().startsWith('|'))
        rows.push(cells(lines[index++]));
      blocks.push(
        <div className="table-scroll" key={start}>
          <table>
            <thead>
              <tr>
                {headers.map((cell, i) => (
                  <th key={i}>{inline(cell)}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((row, i) => (
                <tr key={i}>
                  {row.map((cell, j) => (
                    <td key={j}>{inline(cell)}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>,
      );
      continue;
    }
    if (/^\s*([-*]|\d+\.)\s/.test(line)) {
      const ordered = /^\s*\d+\./.test(line);
      const items: ReactNode[] = [];
      while (index < lines.length && /^\s*([-*]|\d+\.)\s/.test(lines[index])) {
        let item = lines[index++].replace(/^\s*([-*]|\d+\.)\s+/, '');
        while (index < lines.length && /^\s{2,}\S/.test(lines[index]) && !isBlock(lines[index]))
          item += ` ${lines[index++].trim()}`;
        items.push(<li key={index}>{inline(item)}</li>);
      }
      blocks.push(ordered ? <ol key={start}>{items}</ol> : <ul key={start}>{items}</ul>);
      continue;
    }
    if (/^\s*>/.test(line)) {
      const quote: string[] = [];
      while (index < lines.length && /^\s*>/.test(lines[index]))
        quote.push(lines[index++].replace(/^\s*>\s?/, ''));
      blocks.push(<blockquote key={start}>{inline(quote.join(' '))}</blockquote>);
      continue;
    }
    const paragraph = [line];
    index++;
    while (index < lines.length && lines[index].trim() && !isBlock(lines[index]))
      paragraph.push(lines[index++]);
    blocks.push(<p key={start}>{inline(paragraph.join(' '))}</p>);
  }
  return <Fragment>{blocks}</Fragment>;
}
