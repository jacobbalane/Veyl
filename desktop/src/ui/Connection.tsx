import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Smartphone, Plus, Unplug } from 'lucide-react';
import {
  browserPreview,
  connectPhone,
  pairPhone,
  confirmPairing,
  forgetPhone,
  disconnect,
} from '../api/native';
import type { LinkSnapshot, ConnectionState, PairingEvent, SavedPhone } from '../api/types';
import { copy, connectionError, linkCopy } from '../copy';

export function connectionLabel(state: ConnectionState, nearby: number): string {
  const phone = state.phoneName ?? '…';
  switch (state.state) {
    case 'connecting':
    case 'handshaking':
      return linkCopy.connecting(phone);
    case 'pairing':
      return linkCopy.pairing(phone);
    case 'reconnecting':
      return linkCopy.reconnecting(phone);
    case 'unreachable':
      return linkCopy.unreachable;
    case 'connected':
      return state.capture?.state === 'running'
        ? linkCopy.listening(phone)
        : linkCopy.connectedIdle(phone);
    default:
      return nearby ? linkCopy.found : copy.status;
  }
}

export function Modal({
  title,
  children,
  onCancel,
}: {
  title: string;
  children: ReactNode;
  onCancel: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => element?.close();
  }, []);
  return (
    <dialog
      ref={dialog}
      className="connection-dialog"
      aria-label={title}
      onCancel={(event) => {
        event.preventDefault();
        onCancel();
      }}
    >
      <h2>{title}</h2>
      {children}
    </dialog>
  );
}

export function PairingDialog({
  pairing,
  onError,
}: {
  pairing: PairingEvent | null;
  onError: (message: string) => void;
}) {
  const [pending, setPending] = useState(false);
  if (!pairing || !['connecting', 'compare', 'waiting_phone'].includes(pairing.stage)) return null;
  const confirm = async (matches: boolean) => {
    setPending(true);
    try {
      await confirmPairing(pairing.sessionId, matches);
    } catch {
      onError(linkCopy.failedRequest);
    } finally {
      setPending(false);
    }
  };
  return (
    <Modal title={linkCopy.pairTitle(pairing.phoneName)} onCancel={() => void confirm(false)}>
      <p>{linkCopy.codeInstructions}</p>
      <p className="pairing-code" aria-live="polite">
        {pairing.code ?? '······'}
      </p>
      {pairing.stage === 'waiting_phone' && <p role="status">{linkCopy.waitingPhone}</p>}
      <div className="dialog-buttons">
        <button
          className="primary"
          disabled={pending || pairing.stage !== 'compare'}
          onClick={() => void confirm(true)}
        >
          {linkCopy.codesMatch}
        </button>
        <button onClick={() => void confirm(false)}>{linkCopy.cancel}</button>
      </div>
    </Modal>
  );
}

export function Phones({ link }: { link: LinkSnapshot }) {
  const [working, setWorking] = useState(false);
  const [error, setError] = useState('');
  const [address, setAddress] = useState('');
  const [forget, setForget] = useState<SavedPhone | null>(null);
  const busy = working || ['connecting', 'handshaking', 'pairing'].includes(link.connection.state);
  const run = async (action: () => Promise<void>) => {
    setWorking(true);
    setError('');
    try {
      await action();
    } catch {
      setError(linkCopy.failedRequest);
    } finally {
      setWorking(false);
    }
  };
  const pairError =
    link.pairing?.stage === 'failed' ? connectionError(link.pairing.error, true) : '';
  return (
    <section className="panel phones">
      <h1>{linkCopy.yourPhones}</h1>
      {link.phones.paired.length === 0 && <p>{copy.phonesEmpty}</p>}
      <ul className="phone-list">
        {link.phones.paired.map((phone) => (
          <li key={phone.phoneId}>
            <Smartphone size={21} />
            <div className="phone-details">
              <strong>{phone.phoneName}</strong>
              <p>
                {phone.model} · {linkCopy.lastSeen(new Date(phone.lastSeen).toLocaleString())}
              </p>
              <small>{phone.lastAddress}</small>
            </div>
            <div className="phone-buttons">
              {link.connection.phoneId === phone.phoneId &&
              link.connection.state === 'connected' ? (
                <button onClick={() => void run(disconnect)}>
                  <Unplug size={14} />
                  {linkCopy.disconnect}
                </button>
              ) : (
                <button
                  disabled={busy || browserPreview}
                  onClick={() => void run(() => connectPhone({ phoneId: phone.phoneId }))}
                >
                  {linkCopy.connect}
                </button>
              )}
              <button disabled={working || browserPreview} onClick={() => setForget(phone)}>
                {linkCopy.forget}
              </button>
            </div>
          </li>
        ))}
      </ul>
      <h2>{linkCopy.nearby}</h2>
      {link.phones.nearby.length === 0 && <p>{copy.phonesEmpty}</p>}
      <ul className="phone-list">
        {link.phones.nearby.map((phone) => (
          <li key={phone.id}>
            <Smartphone size={21} />
            <div className="phone-details">
              <strong>{phone.name}</strong>
              <p>{phone.model}</p>
              <small>{phone.address}</small>
            </div>
            <button
              disabled={busy || browserPreview}
              onClick={() => void run(() => pairPhone({ nearbyId: phone.id }))}
            >
              {linkCopy.pair}
            </button>
          </li>
        ))}
      </ul>
      <form
        className="address-form"
        onSubmit={(event) => {
          event.preventDefault();
          void run(() => connectPhone({ address: address.trim() }));
        }}
      >
        <h2>
          <Plus size={16} />
          {linkCopy.addressTitle}
        </h2>
        <label htmlFor="phone-address">{linkCopy.addressHint}</label>
        <div className="address-input">
          <input
            id="phone-address"
            value={address}
            placeholder={linkCopy.addressExample}
            required
            autoCapitalize="none"
            autoComplete="off"
            spellCheck={false}
            onChange={(event) => setAddress(event.target.value)}
          />
          <button type="submit" disabled={busy || browserPreview}>
            {linkCopy.connect}
          </button>
        </div>
      </form>
      {link.connection.error === 'not_paired' && link.connection.address && (
        <button
          disabled={busy || browserPreview}
          onClick={() => void run(() => pairPhone({ address: link.connection.address! }))}
        >
          {linkCopy.pair}
        </button>
      )}
      {(error || pairError) && <p role="alert">{error || pairError}</p>}
      {link.pairing?.stage === 'done' && (
        <p role="status">{linkCopy.pairedWith(link.pairing.phoneName)}</p>
      )}
      {forget && (
        <Modal title={linkCopy.forgetTitle(forget.phoneName)} onCancel={() => setForget(null)}>
          <p>{linkCopy.forgetMessage(forget.phoneName)}</p>
          <div className="dialog-buttons">
            <button
              disabled={working}
              onClick={() =>
                void run(async () => {
                  await forgetPhone(forget.phoneId);
                  setForget(null);
                })
              }
            >
              {linkCopy.forget}
            </button>
            <button onClick={() => setForget(null)}>{linkCopy.cancel}</button>
          </div>
        </Modal>
      )}
    </section>
  );
}
