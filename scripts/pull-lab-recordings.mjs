// Explicit local-only debug export. Never invoked by the app, CI or a release task.
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
const adb = sdk
  ? join(sdk, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb')
  : process.platform === 'win32'
    ? join(process.env.LOCALAPPDATA, 'Android', 'Sdk', 'platform-tools', 'adb.exe')
    : 'adb';
const destination = resolve(root, 'lab/recordings');
mkdirSync(destination, { recursive: true });
for (const name of ['reference.wav', 'microphone.wav']) {
  const result = spawnSync(
    adb,
    ['exec-out', 'run-as', 'io.github.jacobbalane.veyl.debug', 'cat', `files/capture-lab/${name}`],
    { maxBuffer: 4 * 1024 * 1024 },
  );
  if (result.error || result.status !== 0) {
    throw new Error(`Could not read ${name}. Connect the phone and finish a Lab recording first.`, {
      cause: result.error,
    });
  }
  const bytes = result.stdout;
  if (
    bytes.length < 44 ||
    bytes.toString('ascii', 0, 4) !== 'RIFF' ||
    bytes.toString('ascii', 8, 12) !== 'WAVE'
  ) {
    throw new Error(`${name} is not a finalized WAV. Stop or finish the recording first.`);
  }
  writeFileSync(join(destination, name), bytes);
  console.log(
    `Saved ${name} locally in lab/recordings (${bytes.length} bytes). Delete it after checking.`,
  );
}
