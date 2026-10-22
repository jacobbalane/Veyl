import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { Resvg } from '@resvg/resvg-js';
import { run } from '@tauri-apps/cli';
import opentype from 'opentype.js';
import { format, resolveConfig } from 'prettier';
import { repositoryRoot as root } from './versions.mjs';

function write(path, data) {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), data);
}
function copy(from, to) {
  mkdirSync(dirname(join(root, to)), { recursive: true });
  copyFileSync(join(root, from), join(root, to));
}

const mark = readFileSync(join(root, 'assets/brand/veyl-mark.svg'), 'utf8');
for (const size of [32, 180, 512, 1024]) {
  write(
    `assets/brand/veyl-mark-${size}.png`,
    new Resvg(mark, { fitTo: { mode: 'width', value: size } }).render().asPng(),
  );
}

// Outlines preserve Manrope ExtraBold even when the viewer has no fonts installed.
const bytes = readFileSync(join(root, 'assets/fonts/manrope-variable.ttf'));
const font = opentype.parse(
  bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength),
);
font.variation.set({ wght: 800 });
const letters = font.getPath('veyl', 0, 0, 240);
const box = letters.getBoundingBox();
const wordmark = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${box.x1} ${box.y1} ${box.x2 - box.x1} ${box.y2 - box.y1}" role="img" aria-label="veyl"><path fill="#4FD8C4" d="${letters.toPathData(3)}"/></svg>\n`;
write(
  'assets/brand/veyl-wordmark.svg',
  await format(wordmark, { ...(await resolveConfig(root)), parser: 'html' }),
);
copy('assets/brand/veyl-mark.svg', 'site/public/favicon.svg');
copy('assets/brand/veyl-mark-32.png', 'site/public/favicon.png');
copy('assets/brand/veyl-mark-180.png', 'site/public/apple-touch-icon.png');

await run(
  ['icon', join(root, 'assets/brand/veyl-mark-1024.png'), '-o', join(root, '.cache/tauri-icons')],
  'tauri',
);
for (const file of ['icon.ico', '32x32.png', '128x128.png', '128x128@2x.png']) {
  copy(`.cache/tauri-icons/${file}`, `desktop/src-tauri/icons/${file}`);
}

const res = 'android/app/src/main/res';
const vector = (
  color,
) => `<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="108" android:viewportHeight="108">
  <path android:strokeColor="${color}" android:strokeWidth="3" android:strokeLineCap="round" android:pathData="M30,48 Q54,24 78,48 M39,54 Q54,39 69,54"/>
  <path android:fillColor="${color}" android:pathData="M59,67 A5,5 0,1 1,49 67 A5,5 0,1 1,59 67"/>
</vector>\n`;
write(`${res}/drawable/ic_launcher_foreground.xml`, vector('#4FD8C4'));
write(`${res}/drawable/ic_launcher_monochrome.xml`, vector('#FFFFFF'));
write(
  `${res}/values/icon_colors.xml`,
  '<resources><color name="ic_launcher_background">#0B0F14</color></resources>\n',
);
for (const name of ['ic_launcher', 'ic_launcher_round']) {
  const layers =
    '<background android:drawable="@color/ic_launcher_background"/><foreground android:drawable="@drawable/ic_launcher_foreground"/>';
  write(
    `${res}/mipmap-anydpi/${name}.xml`,
    `<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">${layers}<monochrome android:drawable="@drawable/ic_launcher_monochrome"/></adaptive-icon>\n`,
  );
}
copy('assets/fonts/manrope-variable.ttf', `${res}/font/manrope_variable.ttf`);
copy('assets/fonts/Manrope-LICENSE.txt', 'android/app/src/main/assets/licenses/manrope.txt');
copy(
  'node_modules/@fontsource-variable/manrope/LICENSE',
  'desktop/src/assets/fonts/Manrope-LICENSE.txt',
);
copy('node_modules/@fontsource-variable/manrope/LICENSE', 'site/public/licenses/manrope.txt');
function textPath(text, x, y, size, weight, color) {
  font.variation.set({ wght: weight });
  return `<path fill="${color}" d="${font.getPath(text, x, y, size).toPathData(3)}"/>`;
}
const og = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="#0B0F14"/>
  <circle cx="1070" cy="520" r="300" fill="none" stroke="#1E2833" stroke-width="2"/>
  <circle cx="1070" cy="520" r="220" fill="none" stroke="#1E2833" stroke-width="2"/>
  <g transform="translate(80 70) scale(.16)">${mark.replace(/<\/?svg[^>]*>/g, '')}</g>
  ${textPath('Touch the air.', 80, 320, 64, 800, '#E8EEF5')}
  ${textPath('Your screen follows.', 80, 402, 64, 800, '#4FD8C4')}
  ${textPath('Veyl · First release coming soon', 80, 525, 24, 500, '#94A3B8')}
</svg>`;
write(
  'site/public/og.png',
  new Resvg(og, {
    font: { fontFiles: [join(root, 'assets/fonts/manrope-variable.ttf')], loadSystemFonts: false },
  })
    .render()
    .asPng(),
);
console.log('Generated Veyl wordmark, desktop icons, Android adaptive icons and website assets.');
