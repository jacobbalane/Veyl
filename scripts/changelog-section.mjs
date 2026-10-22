import { isMain, parseVersion, readText, repositoryRoot } from './versions.mjs';

export function changelogSection(markdown, version) {
  parseVersion(version);
  const lines = markdown.replaceAll('\r\n', '\n').split('\n');
  const sections = [];
  let fence;
  for (let index = 0; index < lines.length; index++) {
    const line = lines[index];
    const marker = /^ {0,3}(`{3,}|~{3,})/.exec(line)?.[1];
    if (fence) {
      if (new RegExp(`^ {0,3}${fence.character}{${fence.length},}[ \\t]*$`).test(line)) {
        fence = undefined;
      }
      continue;
    }
    if (marker) {
      fence = { character: marker[0], length: marker.length };
      continue;
    }
    if (/^##\s+/.test(line)) sections.push({ index, heading: line });
  }
  const matching = sections.filter(({ heading }) => heading.startsWith(`## ${version} (`));
  if (matching.length !== 1) throw new Error(`Expected one changelog section for ${version}.`);
  const start = matching[0].index;
  const end = sections.find(({ index }) => index > start)?.index ?? lines.length;
  const body = lines
    .slice(start + 1, end)
    .join('\n')
    .trim();
  const prose = body
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/^#{1,6} .*$/gm, '')
    .trim();
  if (!prose) throw new Error(`Changelog section for ${version} is empty.`);
  return `${body}\n`;
}

if (isMain(import.meta.url)) {
  try {
    if (process.argv.length !== 3)
      throw new Error('Usage: node scripts/changelog-section.mjs X.Y.Z');
    process.stdout.write(
      changelogSection(readText(repositoryRoot, 'CHANGELOG.md'), process.argv[2]),
    );
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
