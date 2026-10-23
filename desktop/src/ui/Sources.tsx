import { useEffect, useState } from 'react';
import {
  browserPreview,
  setSource,
  setPcDevices,
  watchSource,
  checkPcSound,
  cancelPcSoundCheck,
} from '../api/native';
import { emptySource, type SourceView, type AudioDevice } from '../api/types';
import { pcCopy, pcTip } from '../copy';

export function useSources(initial?: SourceView) {
  const [source, update] = useState<SourceView>(initial ?? emptySource);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const receive = (next: SourceView) =>
    update((old) => (next.revision >= old.revision ? next : old));
  useEffect(() => {
    if (initial) receive(initial);
  }, [initial]);
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    void watchSource((next) => {
      if (active) receive(next);
    })
      .then((unlisten) => {
        if (active) stop = unlisten;
        else unlisten();
      })
      .catch(() => {
        if (active) setError(pcCopy.saveFailed);
      });
    return () => {
      active = false;
      stop?.();
    };
  }, []);
  async function change(action: () => Promise<void>, failure: string = pcCopy.saveFailed) {
    setBusy(true);
    setError('');
    try {
      await action();
    } catch {
      setError(failure);
    } finally {
      setBusy(false);
    }
  }
  return {
    source,
    busy,
    error,
    mode: (mode: string) => change(() => setSource(mode)),
    devices: (speaker: string | null, mic: string | null) =>
      change(() => setPcDevices(speaker, mic)),
    check: () => change(checkPcSound, pcCopy.checkFailed),
    cancelCheck: () => change(cancelPcSoundCheck, pcCopy.checkFailed),
  };
}
type Controls = ReturnType<typeof useSources>;
export function SourceChoice({ controls }: { controls: Controls }) {
  return (
    <section className="panel source-choice">
      <h2>{pcCopy.choose}</h2>
      <div className="source-options">
        {(['pc', 'phone'] as const).map((mode) => (
          <button
            key={mode}
            disabled={browserPreview || controls.busy}
            onClick={() => void controls.mode(mode)}
          >
            <strong>{pcCopy[mode]}</strong>
            {controls.source.devices.recommendedSource === mode && (
              <small>{pcCopy.recommended}</small>
            )}
            <span>{mode === 'pc' ? pcCopy.pcDetail : pcCopy.phoneDetail}</span>
          </button>
        ))}
      </div>
    </section>
  );
}
function Devices({
  label,
  value,
  devices,
  disabled,
  onChange,
}: {
  label: string;
  value: string | null;
  devices: AudioDevice[];
  disabled: boolean;
  onChange: (id: string | null) => void;
}) {
  return (
    <label>
      {label}
      <select
        value={value ?? ''}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value || null)}
      >
        <option value="">{pcCopy.defaultDevice}</option>
        {value && !devices.some((device) => device.id === value) && (
          <option value={value}>{pcCopy.missingDevice}</option>
        )}
        {devices.map((device) => (
          <option key={device.id} value={device.id}>
            {device.name}
          </option>
        ))}
      </select>
    </label>
  );
}
export function SourceSettings({ controls }: { controls: Controls }) {
  const { source, busy, error } = controls;
  return (
    <>
      <label>
        {pcCopy.source}
        <select
          disabled={browserPreview || busy}
          value={source.mode}
          onChange={(event) => void controls.mode(event.target.value)}
        >
          <option value="phone">{pcCopy.phone}</option>
          <option value="pc">{pcCopy.pc}</option>
        </select>
      </label>
      {source.mode === 'pc' && (
        <>
          <Devices
            label={pcCopy.speakers}
            devices={source.devices.speakers}
            value={source.speakerId}
            disabled={browserPreview || busy}
            onChange={(id) => void controls.devices(id, source.microphoneId)}
          />
          <Devices
            label={pcCopy.microphone}
            devices={source.devices.microphones}
            value={source.microphoneId}
            disabled={browserPreview || busy}
            onChange={(id) => void controls.devices(source.speakerId, id)}
          />
          <p role="status">{pcTip(source.tip)}</p>
          <p>{pcCopy.placement}</p>
          <p>{pcCopy.checkNotice}</p>
          <div className="button-row">
            <button
              disabled={
                browserPreview ||
                busy ||
                source.state !== 'running' ||
                source.check.state === 'checking'
              }
              onClick={() => void controls.check()}
            >
              {pcCopy.check}
            </button>
            {source.check.state === 'checking' && (
              <button disabled={busy} onClick={() => void controls.cancelCheck()}>
                {pcCopy.cancel}
              </button>
            )}
          </div>
          {source.check.state !== 'idle' && (
            <p role="status">
              {source.check.state === 'checking'
                ? pcCopy.checking
                : source.check.state === 'good'
                  ? pcCopy.checkGood
                  : source.check.state === 'poor'
                    ? pcCopy.checkPoor
                    : pcCopy.checkFailed}
            </p>
          )}
        </>
      )}
      {error && <p role="alert">{error}</p>}
    </>
  );
}
