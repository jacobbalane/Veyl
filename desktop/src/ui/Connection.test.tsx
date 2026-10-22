import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeAll, expect, test, vi } from 'vitest';
import { Phones, PairingDialog, connectionLabel } from './Connection';
import { emptyLink, type PairingEvent } from '../api/types';
import { connectPhone, pairPhone, confirmPairing, forgetPhone } from '../api/native';

vi.mock('../api/native', () => ({
  browserPreview: false,
  connectPhone: vi.fn(async () => {}),
  pairPhone: vi.fn(async () => {}),
  confirmPairing: vi.fn(async () => {}),
  forgetPhone: vi.fn(async () => {}),
  disconnect: vi.fn(async () => {}),
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
  vi.clearAllMocks();
});

test('a nearby phone pairs by discovery identity and manual entry connects by address', async () => {
  const link = structuredClone(emptyLink);
  link.phones.nearby = [
    { id: '01020304', name: 'Desk phone', model: 'Test', address: '192.168.1.23:47820' },
  ];
  render(<Phones link={link} />);
  fireEvent.click(screen.getByRole('button', { name: 'Pair' }));
  await waitFor(() => expect(pairPhone).toHaveBeenCalledWith({ nearbyId: '01020304' }));
  const input = screen.getByLabelText('The address is shown on your phone in Veyl.');
  fireEvent.change(input, { target: { value: ' 192.168.1.23:47820 ' } });
  fireEvent.submit(input.closest('form')!);
  await waitFor(() => expect(connectPhone).toHaveBeenCalledWith({ address: '192.168.1.23:47820' }));
});
test('code confirmation is tied to its comparison and waiting still permits cancel', async () => {
  const pairing: PairingEvent = {
    sessionId: 'current-comparison',
    phoneName: 'Desk phone',
    code: '123456',
    stage: 'compare',
    error: null,
  };
  const onError = vi.fn();
  const view = render(<PairingDialog pairing={pairing} onError={onError} />);
  fireEvent.click(screen.getByRole('button', { name: 'Codes match' }));
  await waitFor(() => expect(confirmPairing).toHaveBeenCalledWith('current-comparison', true));
  view.rerender(
    <PairingDialog pairing={{ ...pairing, stage: 'waiting_phone' }} onError={onError} />,
  );
  expect(screen.getByRole('button', { name: 'Codes match' }).hasAttribute('disabled')).toBe(true);
  fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(confirmPairing).toHaveBeenCalledWith('current-comparison', false));
  expect(onError).not.toHaveBeenCalled();
});
test('forgetting requires the named-device dialog and cancel keeps the pairing', async () => {
  const link = structuredClone(emptyLink);
  link.phones.paired = [
    {
      phoneId: 'paired-id',
      phoneName: 'Desk phone',
      model: 'Test',
      pairedAt: '2026-10-22T00:00:00Z',
      lastSeen: '2026-10-22T00:00:00Z',
      lastAddress: '192.168.1.23:47820',
    },
  ];
  render(<Phones link={link} />);
  fireEvent.click(screen.getByRole('button', { name: 'Forget' }));
  let dialog = screen.getByRole('dialog', { name: 'Forget Desk phone?' });
  fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  expect(forgetPhone).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Forget' }));
  dialog = screen.getByRole('dialog', { name: 'Forget Desk phone?' });
  fireEvent.click(within(dialog).getByRole('button', { name: 'Forget' }));
  await waitFor(() => expect(forgetPhone).toHaveBeenCalledWith('paired-id'));
});
test('connection wording distinguishes a connected phone from active listening', () => {
  const idle = { ...emptyLink.connection, state: 'connected', phoneName: 'Desk phone' };
  expect(connectionLabel(idle, 0)).toBe('Desk phone connected · tap Start on your phone');
  expect(
    connectionLabel(
      {
        ...idle,
        capture: { state: 'running', source: 'test_sound', route: 'speaker', reason: null },
      },
      0,
    ),
  ).toBe('Listening through Desk phone');
});
