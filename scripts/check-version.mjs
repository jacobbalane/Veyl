import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { changelogSection } from './changelog-section.mjs';
import { isMain, parseVersion, readText, readVersions, repositoryRoot } from './versions.mjs';

export function checkVersion(root = repositoryRoot, env = process.env) {
  const { sources, androidCode } = readVersions(root);
  const { version, code } = parseVersion(sources[0].version);
  for (const source of sources) {
    parseVersion(source.version);
    if (source.version !== version) {
      throw new Error(`${source.path}: ${source.version}; expected ${version}.`);
    }
  }
  if (androidCode !== String(code))
    throw new Error(`VERSION_CODE: ${androidCode}; expected ${code}.`);
  if (existsSync(join(root, 'package-lock.json'))) {
    const lock = JSON.parse(readText(root, 'package-lock.json'));
    for (const [label, locked] of [
      ['version', lock.version],
      ['packages[root]', lock.packages?.['']?.version],
      ['packages[desktop]', lock.packages?.desktop?.version],
      ['packages[site]', lock.packages?.site?.version],
    ]) {
      if (locked !== version) {
        throw new Error(`package-lock.json ${label}: ${locked}; expected ${version}.`);
      }
    }
  }
  if (env.GITHUB_REF_TYPE === 'tag') {
    if (env.GITHUB_REF_NAME !== `v${version}`) throw new Error(`Tag must be v${version}.`);
    changelogSection(readText(root, 'CHANGELOG.md'), version);
  }
  return version;
}

if (isMain(import.meta.url)) {
  try {
    if (process.argv.length !== 2) throw new Error('Usage: node scripts/check-version.mjs');
    console.log(`Version sources agree: ${checkVersion()}`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
