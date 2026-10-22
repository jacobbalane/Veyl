import { spawnSync } from 'node:child_process';
import {
  findNpmCli,
  isMain,
  prepareVersionUpdate,
  repositoryRoot,
  writeVersionUpdate,
} from './versions.mjs';

export function bumpVersion(version, root = repositoryRoot) {
  const changes = prepareVersionUpdate(root, version);
  writeVersionUpdate(root, changes);
  return [...changes.keys()];
}

export function refreshLocks(root, { run = spawnSync, npmCli = findNpmCli() } = {}) {
  const instructions = [];
  const refresh = (command, args, next) => {
    // Cargo's full workspace metadata can exceed Node's output buffer; it is not needed here.
    const result = run(command, args, {
      cwd: root,
      encoding: 'utf8',
      windowsHide: true,
      stdio: ['ignore', 'ignore', 'pipe'],
    });
    if (result.error || result.status !== 0) instructions.push(next);
  };
  refresh('cargo', ['metadata', '--format-version', '1'], 'cargo metadata --format-version 1');
  if (npmCli) {
    refresh(
      process.execPath,
      [npmCli, 'install', '--package-lock-only', '--ignore-scripts', '--no-audit', '--no-fund'],
      'npm install --package-lock-only --ignore-scripts --no-audit --no-fund',
    );
  } else {
    instructions.push('npm install --package-lock-only --ignore-scripts --no-audit --no-fund');
  }
  return instructions;
}

export function runBump(
  args,
  {
    root = repositoryRoot,
    log = console.log,
    error: report = console.error,
    refresh = refreshLocks,
  } = {},
) {
  try {
    if (args.length !== 1) throw new Error('Usage: node scripts/bump-version.mjs X.Y.Z');
    const version = args[0];
    const changed = bumpVersion(version, root);
    log(`Set ${changed.length} version sources to ${version}.`);
    const pending = refresh(root);
    if (pending.length) {
      report('Version sources changed, but lockfile refresh is incomplete. Run:');
      for (const command of pending) report(`  ${command}`);
      return 1;
    }
    return 0;
  } catch (error) {
    report(error.message);
    return 1;
  }
}

if (isMain(import.meta.url)) process.exitCode = runBump(process.argv.slice(2));
