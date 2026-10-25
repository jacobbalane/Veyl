import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { emptyLevels, emptyLink, type TipId } from '../api/types';
import { helpErrors, helpTips, tipFix, tipIds, tipText } from '../copy';
import { Help, Troubleshooting } from './Help';
import { useRecordings } from './Recordings';

vi.mock('../api/native', () => ({
  // The preview skips loading saved recordings.
  browserPreview: true,
  getDiagnostics: async () => ({
    link: structuredClone(emptyLink),
    levels: structuredClone(emptyLevels),
    source: null,
  }),
  copyDiagnostics: async () => '',
  openLogsFolder: vi.fn(async () => {
    throw 'open_failed';
  }),
}));
afterEach(cleanup);

function Harness({ pc = false }: { pc?: boolean }) {
  const recordings = useRecordings();
  return <Help pc={pc} version="0.1.0" recordings={recordings} canRecord={false} />;
}

test('between both modes, Help has an entry with a fix for every tip', () => {
  const covered = new Set<TipId>([...helpTips(false), ...helpTips(true)]);
  expect([...covered].sort()).toEqual([...tipIds].sort());
  for (const pc of [false, true])
    for (const tip of helpTips(pc)) {
      expect(tipFix(tip, pc), `${tip} ${pc ? 'PC' : 'phone'}`).not.toBe('');
      expect(tipFix(tip, pc)).not.toBe(tipText(tip, pc));
    }
  expect(helpTips(false)).not.toContain('pc_headphones');
  expect(helpTips(true)).not.toContain('not_speaker');
  expect(tipFix('unclear_echo', true)).not.toBe(tipFix('unclear_echo', false));
});

test('each mode lists its tips and errors, each with its fix, and links to more help', () => {
  render(<Troubleshooting pc={false} />);
  const list = screen.getByRole('region', { name: "If something isn't working" });
  const entries = list.querySelectorAll('details');
  const phoneErrors = helpErrors.filter((entry) => entry.mode !== 'pc');
  expect(entries).toHaveLength(helpTips(false).length + phoneErrors.length);
  expect(
    within(list).getByText(
      "Sound is going to headphones or Bluetooth. Veyl needs your phone's speaker.",
    ),
  ).toBeDefined();
  expect(
    within(list).getByText('Windows blocked the click in an app running as administrator.'),
  ).toBeDefined();
  expect(
    within(list).queryByText("Veyl couldn't open these audio devices. Check them in Settings."),
  ).toBeNull();
  const summary = within(list).getByText("Turn your phone's volume up.");
  expect(summary.nextElementSibling?.textContent).toBe(tipFix('volume_low', false));
  expect(
    within(list).getByRole('link', { name: 'More help on the Veyl website' }).getAttribute('href'),
  ).toBe('https://jacobbalane.github.io/Veyl/docs/#troubleshooting');
  cleanup();
  render(<Troubleshooting pc />);
  const pc = screen.getByRole('region', { name: "If something isn't working" });
  expect(
    within(pc).getByText("Your PC's sound is going to headphones. Veyl needs speakers."),
  ).toBeDefined();
  expect(
    within(pc).getByText("Veyl couldn't open these audio devices. Check them in Settings."),
  ).toBeDefined();
  expect(within(pc).queryByText('Your phone is connected to another PC.')).toBeNull();
});

test('Help holds placement, privacy, diagnostics with Open logs, and About', async () => {
  render(<Harness pc />);
  const headings = screen
    .getAllByRole('heading', { level: 2 })
    .map((heading) => heading.textContent);
  // In PC mode the microphone's placement comes first.
  expect(headings.slice(0, 5)).toEqual([
    'Placing your microphone',
    'Placing your phone',
    "If something isn't working",
    'Privacy',
    'Diagnostics',
  ]);
  expect(screen.getByText('Hold your hand 10–30 cm above it.')).toBeDefined();
  expect(
    screen.getByText(
      'Sound stays on your devices. Veyl has no accounts, analytics or cloud services.',
    ),
  ).toBeDefined();
  fireEvent.click(screen.getByRole('button', { name: 'Open logs' }));
  expect(await screen.findByText("Veyl couldn't open the logs folder.")).toBeDefined();
  const about = screen.getByRole('region', { name: 'About' });
  expect(within(about).getByText(/Version 0\.1\.0/)).toBeDefined();
  expect(within(about).getByRole('link', { name: 'Source code' }).getAttribute('href')).toBe(
    'https://github.com/jacobbalane/Veyl',
  );
  expect(within(about).getByRole('link', { name: 'Licenses' }).getAttribute('href')).toBe(
    'https://github.com/jacobbalane/Veyl/blob/main/THIRD_PARTY_NOTICES.md',
  );
  expect(within(about).getByRole('link', { name: 'Website' }).getAttribute('href')).toBe(
    'https://jacobbalane.github.io/Veyl/',
  );
  expect(within(about).getByText('Free and open source (MIT)')).toBeDefined();
});
