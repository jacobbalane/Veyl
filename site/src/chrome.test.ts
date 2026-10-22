import { expect, test } from 'vitest';
import { assetFor, DOWNLOAD, REPO } from './chrome';

test('download selection distinguishes both Windows processors and the APK', () => {
  const names = ['Veyl_0.0.1_x64-setup.exe', 'Veyl_0.0.1_arm64-setup.exe', 'Veyl_0.0.1.apk'];
  const assets = names.map((name) => ({
    name,
    browser_download_url: `${REPO}/releases/download/v0.0.1/${name}`,
  }));
  expect(assetFor('Windows', assets)).toContain(names[0]);
  expect(assetFor('WindowsArm', assets)).toContain(names[1]);
  expect(assetFor('Android', assets)).toContain(names[2]);
  expect(
    assetFor('Android', [
      { name: names[2], browser_download_url: 'https://untrusted.example/app.apk' },
    ]),
  ).toBeUndefined();
  expect(
    assetFor('Windows', [{ name: `${names[0]}.sha256`, browser_download_url: DOWNLOAD }]),
  ).toBeUndefined();
  expect(assetFor('Android', [])).toBeUndefined();
});
