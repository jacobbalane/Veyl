import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeAll, expect, test, vi } from 'vitest';
import { RecordingDialogs, RecordingStatus, SavedRecordings, useRecordings } from './Recordings';
import * as native from '../api/native';
import type { RecordingInfo } from '../api/types';

const saved: RecordingInfo = {
  id: '20261022-120000-00000000',
  seconds: 30,
  markPokes: true,
  state: 'saved',
  remainingSeconds: 0,
  blocks: [20, 20],
  error: null,
};
vi.mock('../api/native', () => ({
  browserPreview: false,
  watchRecording: vi.fn(async () => () => {}),
  getRecording: vi.fn(async () => null),
  listRecordings: vi.fn(async () => [saved]),
  startRecording: vi.fn(async (seconds: number, markPokes: boolean) => ({
    ...saved,
    seconds,
    markPokes,
    state: 'recording',
    remainingSeconds: seconds,
  })),
  stopRecording: vi.fn(async () => saved),
  markPoke: vi.fn(async () => {}),
  deleteRecording: vi.fn(async () => {}),
  openRecordingsFolder: vi.fn(async () => {}),
}));
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.clearAllMocks();
});
function Harness() {
  const recording = useRecordings();
  return (
    <>
      <RecordingStatus recording={recording} />
      <SavedRecordings recording={recording} canRecord />
      <RecordingDialogs recording={recording} />
      <input aria-label="Notes" />
    </>
  );
}
test('opening or cancelling consent never starts a recording and duration and marks are explicit', async () => {
  render(<Harness />);
  fireEvent.click(screen.getByRole('button', { name: 'Record a test' }));
  let dialog = screen.getByRole('dialog', { name: 'Record a test' });
  expect(native.startRecording).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  expect(native.startRecording).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Record a test' }));
  dialog = screen.getByRole('dialog', { name: 'Record a test' });
  fireEvent.change(within(dialog).getByLabelText('Duration'), { target: { value: '60' } });
  fireEvent.click(within(dialog).getByLabelText('Mark pokes with the Space key'));
  expect(within(dialog).getByText(/next 60 seconds/)).toBeDefined();
  fireEvent.click(within(dialog).getByRole('button', { name: 'Start recording' }));
  await waitFor(() => expect(native.startRecording).toHaveBeenCalledWith(60, false));
  expect(await screen.findByText('Recording · 60 s left')).toBeDefined();
  expect(screen.queryByText('Press Space each time you poke')).toBeNull();
});
test('Space marks require active consent and focus, ignore repeats and leave text inputs alone', async () => {
  const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(true);
  render(<Harness />);
  fireEvent.keyDown(document.body, { code: 'Space' });
  expect(native.markPoke).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Record a test' }));
  fireEvent.click(screen.getByRole('button', { name: 'Start recording' }));
  await screen.findByText('Recording · 30 s left');
  fireEvent.keyDown(document.body, { code: 'Space' });
  fireEvent.keyDown(document.body, { code: 'Space', repeat: true });
  fireEvent.keyDown(screen.getByLabelText('Notes'), { code: 'Space' });
  focus.mockReturnValue(false);
  fireEvent.keyDown(document.body, { code: 'Space' });
  expect(native.markPoke).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole('button', { name: 'Stop recording' }));
  await waitFor(() => expect(native.stopRecording).toHaveBeenCalledTimes(1));
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Stop recording' })).toBeNull());
  focus.mockReturnValue(true);
  fireEvent.keyDown(document.body, { code: 'Space' });
  expect(native.markPoke).toHaveBeenCalledTimes(1);
});
test('deletion requires confirmation and opening the folder is a separate action', async () => {
  render(<Harness />);
  await screen.findByText('Saved. Recordings stay on this PC.');
  fireEvent.click(screen.getByRole('button', { name: 'Open folder' }));
  expect(native.openRecordingsFolder).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
  let dialog = screen.getByRole('dialog', { name: 'Delete this recording?' });
  fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  expect(native.deleteRecording).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
  dialog = screen.getByRole('dialog', { name: 'Delete this recording?' });
  fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));
  await waitFor(() => expect(native.deleteRecording).toHaveBeenCalledWith(saved.id));
});
