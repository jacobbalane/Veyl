import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { emptySource, type SourceView } from '../api/types';
import { SourceChoice, SourceSettings, useSources } from './Sources';
import { setSource, setPcDevices, checkPcSound, cancelPcSoundCheck } from '../api/native';

const events = vi.hoisted(() => new Set<(source: SourceView) => void>());
vi.mock('../api/native', () => ({
  browserPreview: false,
  setSource: vi.fn(async () => {}),
  setPcDevices: vi.fn(async () => {}),
  checkPcSound: vi.fn(async () => {}),
  cancelPcSoundCheck: vi.fn(async () => {}),
  watchSource: async (callback: (source: SourceView) => void) => {
    events.add(callback);
    return () => {
      events.delete(callback);
    };
  },
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  events.clear();
});
test('the PC sound check starts only on a click, exposes cancellation and shows the measured result', async () => {
  render(<Controls initial={pc(1)} />);
  await waitFor(() => expect(events.size).toBe(1));
  expect(checkPcSound).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: "Check this PC's sound" }));
  await waitFor(() => expect(checkPcSound).toHaveBeenCalledOnce());
  const checking = pc(2);
  checking.check.state = 'checking';
  act(() => events.forEach((event) => event(checking)));
  expect(
    screen.getByRole('button', { name: "Check this PC's sound" }).hasAttribute('disabled'),
  ).toBe(true);
  fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(cancelPcSoundCheck).toHaveBeenCalledOnce());
  const good = pc(3);
  good.check.state = 'good';
  act(() => events.forEach((event) => event(good)));
  expect(screen.getByText('This PC hears its own sound well.')).toBeDefined();
});
function Controls({ initial = emptySource }: { initial?: SourceView }) {
  const controls = useSources(initial);
  return (
    <>
      {!controls.source.choiceDone && <SourceChoice controls={controls} />}
      <SourceSettings controls={controls} />
    </>
  );
}
function pc(revision: number): SourceView {
  const speaker = {
    id: 'Case-Sensitive',
    name: 'Desk speakers',
    formFactor: 'speakers',
    rawSupported: false,
    bluetooth: false,
    builtIn: false,
  };
  return {
    ...structuredClone(emptySource),
    mode: 'pc',
    state: 'running',
    choiceDone: true,
    revision,
    devices: { ...structuredClone(emptySource.devices), speakers: [speaker], error: null },
  };
}
test('source choice is deliberate and device pickers preserve default and missing endpoint choices', async () => {
  render(<Controls />);
  await waitFor(() => expect(events.size).toBe(1));
  fireEvent.click(screen.getByRole('button', { name: /With this PC/ }));
  await waitFor(() => expect(setSource).toHaveBeenCalledWith('pc'));
  act(() => events.forEach((event) => event(pc(1))));
  fireEvent.change(screen.getByLabelText('Speakers'), { target: { value: 'Case-Sensitive' } });
  await waitFor(() => expect(setPcDevices).toHaveBeenCalledWith('Case-Sensitive', null));
  const missing = pc(2);
  missing.speakerId = 'Case-Sensitive';
  missing.devices.speakers = [];
  missing.state = 'unavailable';
  missing.tip = 'pc_no_speaker';
  act(() => events.forEach((event) => event(missing)));
  expect(screen.getByRole('option', { name: 'Unavailable device' }).getAttribute('value')).toBe(
    'Case-Sensitive',
  );
  expect(screen.getByText('Choose speakers in Settings.')).toBeDefined();
  fireEvent.change(screen.getByLabelText('Speakers'), { target: { value: '' } });
  await waitFor(() => expect(setPcDevices).toHaveBeenLastCalledWith(null, null));
});
test('a delayed initial snapshot cannot replace a newer source event and unmount removes listeners', async () => {
  const view = render(<Controls />);
  await waitFor(() => expect(events.size).toBe(1));
  act(() => events.forEach((event) => event(pc(5))));
  view.rerender(<Controls initial={{ ...structuredClone(emptySource), revision: 2 }} />);
  expect((screen.getByLabelText('Sound source') as HTMLSelectElement).value).toBe('pc');
  expect(screen.queryByText('How do you want to use Veyl?')).toBeNull();
  view.unmount();
  expect(events.size).toBe(0);
});
