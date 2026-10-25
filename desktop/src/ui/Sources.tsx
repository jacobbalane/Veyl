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
  /** Runs a change and says whether it worked; a failure shows in words. */
  async function change(
    action: () => Promise<void>,
    failure: string = pcCopy.saveFailed,
  ): Promise<boolean> {
    setBusy(true);
    setError('');
    try {
      await action();
      return true;
    } catch {
      setError(failure);
      return false;
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
export type SourceControls = ReturnType<typeof useSources>;
type Controls = SourceControls;
/**
 * The two ways to use Veyl, with the recommended one marked (§7.14). Choosing one saves it; the
 * user always decides. The browser preview switches its sample data instead.
 */
export function SourceOptions({
  controls,
  onChosen,
}: {
  controls: Controls;
  onChosen?: (mode: 'pc' | 'phone') => void;
}) {
  const { source } = controls;
  return (
    <div className="source-options">
      {(['pc', 'phone'] as const).map((mode) => (
        <button
          key={mode}
          disabled={controls.busy}
          aria-pressed={source.choiceDone ? source.mode === mode : undefined}
          onClick={() =>
            void controls.mode(mode).then((chosen) => {
              if (chosen) onChosen?.(mode);
            })
          }
        >
          <strong>{pcCopy[mode]}</strong>
          {source.devices.recommendedSource === mode && <small>{pcCopy.recommended}</small>}
          <span>{mode === 'pc' ? pcCopy.pcDetail : pcCopy.phoneDetail}</span>
        </button>
      ))}
    </div>
  );
}
export function SourceChoice({ controls }: { controls: Controls }) {
  return (
    <section className="panel source-choice">
      <h2>{pcCopy.choose}</h2>
      <SourceOptions controls={controls} />
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
/** Speakers, microphone and the optional sound check for PC mode. */
export function PcDevices({
  controls,
  placement = true,
}: {
  controls: Controls;
  placement?: boolean;
}) {
  const { source, busy } = controls;
  return (
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
      {placement && <p>{pcCopy.placement}</p>}
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
  );
}
export function SourceSettings({ controls }: { controls: Controls }) {
  const { source, busy, error } = controls;
  return (
    <>
      <label>
        {pcCopy.source}
        <select
          disabled={busy}
          value={source.mode}
          onChange={(event) => void controls.mode(event.target.value)}
        >
          <option value="phone">{pcCopy.phone}</option>
          <option value="pc">{pcCopy.pc}</option>
        </select>
      </label>
      {source.mode === 'pc' && <PcDevices controls={controls} />}
      {error && <p role="alert">{error}</p>}
    </>
  );
}
