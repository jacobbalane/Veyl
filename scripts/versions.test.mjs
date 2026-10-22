import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve, sep } from 'node:path';
import test from 'node:test';
import { bumpVersion, refreshLocks, runBump } from './bump-version.mjs';
import { checkVersion } from './check-version.mjs';
import { jsonSources, parseVersion, repositoryRoot } from './versions.mjs';

function fixture(t) {
  const base = join(repositoryRoot, '.cache', 'version-tests');
  mkdirSync(base, { recursive: true });
  const root = mkdtempSync(join(base, 'fixture with spaces-'));
  const write = (path, text) => {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text, 'utf8');
  };
  for (const path of jsonSources)
    write(path, JSON.stringify({ name: path, version: '0.0.1', private: true }));
  write(
    'Cargo.toml',
    '[workspace]\nmembers = []\n\n[workspace.package]\nversion = "0.0.1" # product\nlicense = "MIT"\n\n[profile.release]\nstrip = true\n',
  );
  write(
    'android/gradle.properties',
    '# Keep JVM settings.\r\nVERSION_NAME=0.0.1\r\nVERSION_CODE=1\r\norg.gradle.jvmargs=-Xmx2g\r\n',
  );
  write(
    'CHANGELOG.md',
    '# Changelog\n\n## Unreleased\n\n## 0.0.1 (2026-10-21)\n\nPreview build.\n',
  );
  t.after(() => {
    assert.ok(resolve(root).startsWith(`${resolve(base)}${sep}`));
    rmSync(root, { recursive: true, force: true });
  });
  return { root, write, read: (path) => readFileSync(join(root, path), 'utf8') };
}

test('versionCode maps versions uniquely within Android limits', () => {
  assert.deepEqual(parseVersion('0.1.0'), { version: '0.1.0', code: 100 });
  assert.equal(parseVersion('2.13.7').code, 21_307);
  assert.equal(parseVersion('210000.0.0').code, 2_100_000_000);
  for (const version of [
    '0.0.0',
    '01.2.3',
    'v1.2.3',
    '1.100.0',
    '1.0.100',
    '1.2.3-rc.1',
    '1.2.3+build',
    '210000.0.1',
    '9007199254740991.0.0',
    '1.2.3\n',
    undefined,
  ]) {
    assert.throws(() => parseVersion(version), undefined, String(version));
  }
});

test('checks every source and the Android versionCode', (t) => {
  const { root, write } = fixture(t);
  assert.equal(checkVersion(root, {}), '0.0.1');
  for (const path of jsonSources) {
    write(path, JSON.stringify({ version: '0.2.0' }));
    assert.throws(() => checkVersion(root, {}));
    write(path, JSON.stringify({ version: '0.0.1' }));
  }
  write('Cargo.toml', '[workspace.package]\nversion = "0.2.0"\n');
  assert.throws(() => checkVersion(root, {}), /Cargo.toml/);
  write('Cargo.toml', '[workspace.package]\nversion = "0.0.1"\n');
  write('android/gradle.properties', 'VERSION_NAME=0.2.0\nVERSION_CODE=1\n');
  assert.throws(() => checkVersion(root, {}), /gradle.properties/);
  write('android/gradle.properties', 'VERSION_NAME=0.0.1\nVERSION_CODE=2\n');
  assert.throws(() => checkVersion(root, {}), /VERSION_CODE/);
});

test('tag builds require the exact tag and a nonempty changelog section', (t) => {
  const { root, write } = fixture(t);
  const env = { GITHUB_REF_TYPE: 'tag', GITHUB_REF_NAME: 'v0.0.1' };
  assert.equal(checkVersion(root, env), '0.0.1');
  assert.throws(() => checkVersion(root, { ...env, GITHUB_REF_NAME: 'v0.1.0' }), /Tag must/);
  write('CHANGELOG.md', '# Changelog\n\n## Unreleased\n');
  assert.throws(() => checkVersion(root, env), /changelog section/);
  write('CHANGELOG.md', '# Changelog\n\n## 0.0.1 (2026-10-21)\n');
  assert.throws(() => checkVersion(root, env), /empty/);
  assert.equal(checkVersion(root, { GITHUB_REF_TYPE: 'branch', GITHUB_REF_NAME: 'main' }), '0.0.1');
});

test('version bump updates all sources without changing unrelated settings', (t) => {
  const { root, read } = fixture(t);
  assert.equal(bumpVersion('1.2.3', root).length, 6);
  assert.equal(checkVersion(root, {}), '1.2.3');
  for (const path of jsonSources) assert.equal(JSON.parse(read(path)).private, true);
  assert.match(read('Cargo.toml'), /version = "1.2.3" # product/);
  assert.match(read('Cargo.toml'), /strip = true/);
  assert.match(read('android/gradle.properties'), /VERSION_CODE=10203\r\n/);
  assert.match(read('android/gradle.properties'), /org.gradle.jvmargs=-Xmx2g/);
});

test('invalid or missing sources cause no partial writes', (t) => {
  const { root, write, read } = fixture(t);
  const before = read('package.json');
  assert.throws(() => bumpVersion('1.100.0', root));
  assert.equal(read('package.json'), before);
  write('android/gradle.properties', 'VERSION_NAME=0.0.1\n');
  assert.throws(() => bumpVersion('0.1.0', root), /VERSION_CODE/);
  assert.equal(read('package.json'), before);
  write('android/gradle.properties', 'VERSION_NAME=0.0.1\nVERSION_CODE=1\nVERSION_CODE=2\n');
  assert.throws(() => bumpVersion('0.1.0', root), /exactly one/);
  assert.equal(read('package.json'), before);
});

test('bump preserves JSON formatting and leaves nested version properties alone', (t) => {
  const { root, write, read } = fixture(t);
  const before =
    '{\n  "extra": { "version": "9.9.9" },\n  "version": "0.0.1",\n  "workspaces": ["desktop", "site"]\n}\n';
  write('package.json', before);
  bumpVersion('0.1.0', root);
  assert.equal(read('package.json'), before.replace('"version": "0.0.1"', '"version": "0.1.0"'));
});

test('a committed npm lock cannot drift from the workspaces', (t) => {
  const { root, write } = fixture(t);
  const lock = {
    version: '0.0.1',
    packages: {
      '': { version: '0.0.1' },
      desktop: { version: '0.0.1' },
      site: { version: '0.0.1' },
    },
  };
  write('package-lock.json', JSON.stringify(lock));
  assert.equal(checkVersion(root, {}), '0.0.1');
  lock.packages.site.version = '0.1.0';
  write('package-lock.json', JSON.stringify(lock));
  assert.throws(() => checkVersion(root, {}), /packages\[site\]/);
});

test('missing toolchains report refresh commands and never claim lockfiles are current', () => {
  const calls = [];
  const pending = refreshLocks('C:/workspace with spaces', {
    npmCli: 'C:/node with spaces/npm-cli.js',
    run: (command, args, options) => {
      calls.push({ command, args, options });
      return { error: new Error('missing') };
    },
  });
  assert.equal(pending.length, 2);
  assert.match(pending[0], /^cargo metadata/);
  assert.match(pending[1], /^npm install/);
  assert.equal(calls[1].command, process.execPath);
  assert.equal(calls[1].args[0], 'C:/node with spaces/npm-cli.js');
  assert.equal(calls[1].options.cwd, 'C:/workspace with spaces');
  assert.equal(calls[1].options.shell, undefined);
  assert.equal(calls[0].options.stdio[1], 'ignore');
});

test('successful lock refresh runs both tools; failed refresh remains pending', () => {
  assert.deepEqual(refreshLocks('.', { npmCli: 'npm-cli.js', run: () => ({ status: 0 }) }), []);
  assert.equal(refreshLocks('.', { npmCli: 'npm-cli.js', run: () => ({ status: 1 }) }).length, 2);
});

test('CLI rejects invalid arguments without writes or dependency commands', (t) => {
  const { root, read } = fixture(t);
  const before = read('package.json');
  const errors = [];
  const options = {
    root,
    log: () => assert.fail('Must not report a successful bump.'),
    error: (message) => errors.push(message),
    refresh: () => assert.fail('Must not run dependency commands.'),
  };
  assert.equal(runBump(['1.100.0'], options), 1);
  assert.match(errors[0], /Minor and patch/);
  assert.equal(runBump([], options), 1);
  assert.equal(runBump(['0.1.0', 'extra'], options), 1);
  assert.equal(read('package.json'), before);
});

test('CLI distinguishes a completed bump from a pending lock refresh', (t) => {
  const { root } = fixture(t);
  const output = [];
  const options = { root, log: (line) => output.push(line), error: (line) => output.push(line) };
  assert.equal(runBump(['0.1.0'], { ...options, refresh: () => [] }), 0);
  assert.equal(checkVersion(root, {}), '0.1.0');
  assert.equal(
    runBump(['0.2.0'], { ...options, refresh: () => ['cargo metadata --format-version 1'] }),
    1,
  );
  assert.equal(checkVersion(root, {}), '0.2.0');
  assert.ok(output.some((line) => line.includes('lockfile refresh is incomplete')));
});
