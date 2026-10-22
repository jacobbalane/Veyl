import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const repositoryRoot = fileURLToPath(new URL('..', import.meta.url));
export const jsonSources = [
  'package.json',
  'desktop/package.json',
  'site/package.json',
  'desktop/src-tauri/tauri.conf.json',
];

export function isMain(url) {
  return Boolean(process.argv[1] && resolve(process.argv[1]) === fileURLToPath(url));
}

export function parseVersion(version) {
  if (typeof version !== 'string' || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
    throw new Error(
      'Use a release version such as 0.1.0, without a v prefix or prerelease suffix.',
    );
  }
  const [major, minor, patch] = version.split('.').map(Number);
  const code = major * 10_000 + minor * 100 + patch;
  if (
    minor >= 100 ||
    patch >= 100 ||
    !Number.isSafeInteger(code) ||
    code < 1 ||
    code > 2_100_000_000
  ) {
    throw new Error('Minor and patch must be below 100; Android versionCode must be 1–2100000000.');
  }
  return { version, code };
}

export function readText(root, path) {
  return readFileSync(join(root, path), 'utf8');
}

function oneMatch(text, expression, label) {
  const matches = [...text.matchAll(expression)];
  if (matches.length !== 1) throw new Error(`Expected exactly one ${label}.`);
  return matches[0];
}

function cargoSection(text) {
  return oneMatch(
    text,
    /^\[workspace\.package\][ \t]*\r?\n([\s\S]*?)(?=^\[|$(?![\s\S]))/gm,
    '[workspace.package] section in Cargo.toml',
  );
}

export function cargoVersion(text) {
  return oneMatch(
    cargoSection(text)[1],
    /^version[ \t]*=[ \t]*"([^"\r\n]+)"[ \t]*(?:#[^\r\n]*)?\r?$/gm,
    'workspace version in Cargo.toml',
  )[1];
}

export function property(text, name) {
  return oneMatch(
    text,
    new RegExp(`^${name}[ \\t]*=[ \\t]*([^\\r\\n]*)\\r?$`, 'gm'),
    name,
  )[1].trim();
}

export function readVersions(root = repositoryRoot) {
  const sources = jsonSources.map((path) => ({
    path,
    version: JSON.parse(readText(root, path)).version,
  }));
  sources.push({ path: 'Cargo.toml', version: cargoVersion(readText(root, 'Cargo.toml')) });
  const properties = readText(root, 'android/gradle.properties');
  sources.push({
    path: 'android/gradle.properties',
    version: property(properties, 'VERSION_NAME'),
  });
  return { sources, androidCode: property(properties, 'VERSION_CODE') };
}

function replaceJsonVersion(text, version) {
  // Keep formatting intact so a version bump cannot fail the repository's formatting check.
  let depth = 0;
  const edits = [];
  for (const token of text.matchAll(/"(?:[^"\\]|\\.)*"|[{}\[\]]/g)) {
    if (token[0] === '{' || token[0] === '[') depth++;
    else if (token[0] === '}' || token[0] === ']') depth--;
    else if (depth === 1 && JSON.parse(token[0]) === 'version') {
      const offset = token.index + token[0].length;
      const value = /^(\s*:\s*)"(?:[^"\\]|\\.)*"/.exec(text.slice(offset));
      if (value) edits.push({ offset, value });
    }
  }
  if (edits.length !== 1) throw new Error('Expected exactly one top-level JSON version.');
  const { offset, value } = edits[0];
  return (
    text.slice(0, offset) +
    value[1] +
    JSON.stringify(version) +
    text.slice(offset + value[0].length)
  );
}

// Validate every input before writing so a missing source cannot leave half a version bump.
export function prepareVersionUpdate(root, version) {
  const { code } = parseVersion(version);
  const { sources, androidCode } = readVersions(root);
  for (const source of sources) parseVersion(source.version);
  if (!/^[1-9]\d*$/.test(androidCode)) throw new Error('Invalid existing VERSION_CODE.');
  const changes = new Map();
  for (const path of jsonSources) {
    changes.set(path, replaceJsonVersion(readText(root, path), version));
  }
  const cargo = readText(root, 'Cargo.toml');
  const section = cargoSection(cargo);
  const updated = section[0].replace(/^(version[ \t]*=[ \t]*)"[^"\r\n]+"/m, `$1"${version}"`);
  changes.set(
    'Cargo.toml',
    cargo.slice(0, section.index) + updated + cargo.slice(section.index + section[0].length),
  );
  const properties = readText(root, 'android/gradle.properties')
    .replace(/^(VERSION_NAME[ \t]*=[ \t]*)[^\r\n]*/m, `$1${version}`)
    .replace(/^(VERSION_CODE[ \t]*=[ \t]*)[^\r\n]*/m, `$1${code}`);
  changes.set('android/gradle.properties', properties);
  return changes;
}

export function writeVersionUpdate(root, changes) {
  for (const [path, contents] of changes) writeFileSync(join(root, path), contents, 'utf8');
}

// Run npm through Node so paths with spaces work on Windows without a command shell.
export function findNpmCli(env = process.env, nodeExecutable = process.execPath) {
  const search = [
    env.npm_execpath,
    join(dirname(nodeExecutable), 'node_modules/npm/bin/npm-cli.js'),
    ...(env.PATH ?? env.Path ?? '')
      .split(process.platform === 'win32' ? ';' : ':')
      .filter(Boolean)
      .map((directory) => join(directory, 'node_modules/npm/bin/npm-cli.js')),
  ];
  return search.find((path) => path && /npm-cli\.js$/.test(path) && existsSync(path));
}
