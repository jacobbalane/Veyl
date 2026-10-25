// Captures the desktop interface's browser preview for the website (plan §8.6). The preview shows
// sample data only. Usage: node scripts/capture-screens.mjs. Set VEYL_BROWSER=chromium to use
// Playwright's own browser instead of Microsoft Edge.
import { spawn } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { isMain, repositoryRoot } from './versions.mjs';

const port = 1421;
export const screensDirectory = join(repositoryRoot, 'site', 'public', 'screens');

/** Each screen: a file name, the preview's address options and the colour scheme. */
export const shots = [
  { name: 'live-phone-dark', query: { score: '78' }, scheme: 'dark' },
  { name: 'live-phone-light', query: { score: '78' }, scheme: 'light' },
  { name: 'live-pc-dark', query: { mode: 'pc', score: '72' }, scheme: 'dark' },
  { name: 'live-tip-dark', query: { tip: 'few_high_sounds', score: '41' }, scheme: 'dark' },
];

export function shotUrl(shot) {
  const query = new URLSearchParams(shot.query).toString();
  return `http://127.0.0.1:${port}/${query ? `?${query}` : ''}`;
}

async function waitForServer(url, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      if ((await fetch(url)).ok) return;
    } catch {
      // The server is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error('The desktop preview did not start.');
}

async function capture() {
  const { chromium } = await import('@playwright/test');
  const server = spawn(
    process.execPath,
    [
      join(repositoryRoot, 'node_modules', 'vite', 'bin', 'vite.js'),
      '--host',
      '127.0.0.1',
      '--port',
      String(port),
      '--strictPort',
    ],
    { cwd: join(repositoryRoot, 'desktop'), stdio: 'ignore' },
  );
  const browser = await waitForServer(`http://127.0.0.1:${port}/`, 30_000).then(() =>
    chromium.launch(
      process.env.VEYL_BROWSER === 'chromium'
        ? {}
        : { channel: process.env.VEYL_BROWSER ?? 'msedge' },
    ),
  );
  try {
    await mkdir(screensDirectory, { recursive: true });
    for (const shot of shots) {
      const page = await browser.newPage({
        viewport: { width: 1280, height: 800 },
        deviceScaleFactor: 1,
        colorScheme: shot.scheme,
        reducedMotion: 'reduce',
      });
      await page.goto(shotUrl(shot));
      await page.waitForSelector('.echo-canvas');
      // Let the echo view fill with a few seconds of the preview's hand and pokes.
      await page.waitForTimeout(4500);
      await page.screenshot({ path: join(screensDirectory, `${shot.name}.png`) });
      await page.close();
      console.log(`Saved site/public/screens/${shot.name}.png`);
    }
  } finally {
    await browser.close();
    server.kill();
  }
}

if (isMain(import.meta.url)) {
  capture().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
