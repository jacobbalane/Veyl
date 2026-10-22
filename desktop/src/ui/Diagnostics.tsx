import { useEffect, useState } from 'react';
import { getDiagnostics, copyDiagnostics } from '../api/native';
import type { Diagnostics as DiagnosticsValue, StreamLevels } from '../api/types';
import { connectionLabel } from './Connection';
import { linkCopy } from '../copy';

function Level({ name, level }: { name: string; level: StreamLevels }) {
  return (
    <div className="signal-level">
      <strong>{linkCopy.blocksPerSecond(name, level.blocksPerSecond.toFixed(1))}</strong>
      <p>
        {linkCopy.beforeFilter}: {level.preDbfs.toFixed(1)} dBFS · {linkCopy.afterFilter}:{' '}
        {level.filteredDbfs.toFixed(1)} dBFS
      </p>
      <meter
        aria-label={`${name} · ${linkCopy.afterFilter}`}
        min={-120}
        max={0}
        value={level.filteredDbfs}
      />
      <small>{linkCopy.counts(level.blocks.toLocaleString(), level.gaps.toLocaleString())}</small>
    </div>
  );
}
export function Diagnostics() {
  const [value, setValue] = useState<DiagnosticsValue>();
  const [message, setMessage] = useState('');
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      try {
        const next = await getDiagnostics();
        if (active) setValue(next);
      } catch {
        if (active) setMessage(linkCopy.failedRequest);
      } finally {
        if (active) timer = setTimeout(() => void refresh(), 1000);
      }
    };
    void refresh();
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, []);
  const copy = async () => {
    try {
      const text = await copyDiagnostics();
      await navigator.clipboard.writeText(text);
      setMessage(linkCopy.copied);
    } catch {
      setMessage(linkCopy.copyFailed);
    }
  };
  const number = (value: number | null | undefined, suffix: string, digits = 1) =>
    value == null ? linkCopy.unavailable : `${value.toFixed(digits)} ${suffix}`;
  return (
    <section className="diagnostics">
      <h2>{linkCopy.diagnostics}</h2>
      <p>
        {value
          ? connectionLabel(value.link.connection, value.link.phones.nearby.length)
          : linkCopy.unavailable}
      </p>
      {value && (
        <>
          <div className="signal-grid">
            <Level name={linkCopy.soundCopy} level={value.levels.reference} />
            <Level name={linkCopy.microphone} level={value.levels.microphone} />
          </div>
          <dl className="diagnostic-values">
            <dt>{linkCopy.dropped}</dt>
            <dd>{value.link.phoneStats.droppedBlocks}</dd>
            <dt>{linkCopy.engineDrops}</dt>
            <dd>{value.levels.engineDrops}</dd>
            <dt>{linkCopy.roundTrip}</dt>
            <dd>{number(value.link.phoneStats.rttMs, 'ms')}</dd>
            <dt>{linkCopy.arrivalDelay}</dt>
            <dd>{number(value.levels.arrivalMs, 'ms')}</dd>
            <dt>{linkCopy.phoneQueue}</dt>
            <dd>{value.link.phoneStats.queueMs} ms</dd>
            <dt>{linkCopy.sampleRate}</dt>
            <dd>{number(value.levels.sampleRate, 'Hz', 0)}</dd>
            <dt>{linkCopy.battery}</dt>
            <dd>{number(value.link.phoneStats.batteryPct, '%', 0)}</dd>
          </dl>
        </>
      )}
      <button onClick={() => void copy()}>{linkCopy.copyDiagnostics}</button>
      {message && <p role="status">{message}</p>}
    </section>
  );
}
