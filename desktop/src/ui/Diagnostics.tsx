import { useEffect, useState } from 'react';
import { getDiagnostics, copyDiagnostics, openLogsFolder } from '../api/native';
import type { Diagnostics as DiagnosticsValue, StreamLevels, CaptureDetails } from '../api/types';
import { connectionLabel } from './Connection';
import { helpCopy, linkCopy, liveCopy, pcCopy } from '../copy';
import { formatLatency } from './live-model';

function PcFormat({ name, details }: { name: string; details: CaptureDetails | null }) {
  return (
    <section>
      <h3>{name}</h3>
      {details ? (
        <dl className="diagnostic-values">
          <dt>{pcCopy.format}</dt>
          <dd>
            {details.format.sampleRate} Hz · {details.format.channels} ch ·{' '}
            {details.format.encoding}
          </dd>
          <dt>{pcCopy.raw}</dt>
          <dd>{details.rawMode ? pcCopy.on : pcCopy.off}</dd>
          <dt>{pcCopy.resampling}</dt>
          <dd>
            {details.format.sampleRate === 48000
              ? pcCopy.off
              : `${details.format.sampleRate} → 48000 Hz`}
          </dd>
          <dt>{pcCopy.buffer}</dt>
          <dd>{details.bufferMs.toFixed(1)} ms</dd>
          <dt>{pcCopy.scheduling}</dt>
          <dd>{details.proAudioThread ? pcCopy.on : pcCopy.off}</dd>
        </dl>
      ) : (
        <p>{linkCopy.unavailable}</p>
      )}
    </section>
  );
}

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
  const openLogs = async () => {
    try {
      await openLogsFolder();
    } catch {
      setMessage(helpCopy.logsFailed);
    }
  };
  const number = (value: number | null | undefined, suffix: string, digits = 1) =>
    value == null ? linkCopy.unavailable : `${value.toFixed(digits)} ${suffix}`;
  const pc = value?.source?.mode === 'pc';
  return (
    <section className="diagnostics">
      <h2>{linkCopy.diagnostics}</h2>
      <p>
        {value
          ? pc
            ? value.source?.state === 'running'
              ? pcCopy.listening
              : pcCopy.noSound
            : connectionLabel(value.link.connection, value.link.phones.nearby.length)
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
            <dd>{pc ? linkCopy.unavailable : value.link.phoneStats.droppedBlocks}</dd>
            <dt>{linkCopy.engineDrops}</dt>
            <dd>{value.levels.engineDrops}</dd>
            <dt>{linkCopy.roundTrip}</dt>
            <dd>{number(pc ? null : value.link.phoneStats.rttMs, 'ms')}</dd>
            <dt>{linkCopy.arrivalDelay}</dt>
            <dd>{number(value.levels.arrivalMs, 'ms')}</dd>
            <dt>{linkCopy.phoneQueue}</dt>
            <dd>{pc ? linkCopy.unavailable : `${value.link.phoneStats.queueMs} ms`}</dd>
            <dt>{linkCopy.sampleRate}</dt>
            <dd>{number(value.levels.sampleRate, 'Hz', 0)}</dd>
            <dt>{linkCopy.battery}</dt>
            <dd>{number(pc ? null : value.link.phoneStats.batteryPct, '%', 0)}</dd>
            <dt>{liveCopy.pokeDelay}</dt>
            <dd>{formatLatency(value.levels.latency)}</dd>
            <dt>{liveCopy.pokesMeasured}</dt>
            <dd>{value.levels.latency.pokes}</dd>
          </dl>
          {pc && value.source && (
            <>
              <PcFormat
                name={value.source.speaker?.name ?? pcCopy.speakers}
                details={value.source.reference}
              />
              <PcFormat
                name={value.source.microphone?.name ?? pcCopy.microphone}
                details={value.source.capture}
              />
              <dl className="diagnostic-values">
                <dt>{pcCopy.drift}</dt>
                <dd>{linkCopy.unavailable}</dd>
                <dt>{pcCopy.coherence}</dt>
                <dd>
                  {number(
                    value.source.check.result
                      ? value.source.check.result.meanCoherence * 100
                      : null,
                    '%',
                  )}
                </dd>
                <dt>{pcCopy.bandwidth}</dt>
                <dd>{number(value.source.check.result?.bandwidthHz, 'Hz', 0)}</dd>
              </dl>
            </>
          )}
        </>
      )}
      <div className="diagnostic-buttons">
        <button onClick={() => void openLogs()}>{helpCopy.openLogs}</button>
        <button onClick={() => void copy()}>{linkCopy.copyDiagnostics}</button>
      </div>
      {message && <p role="status">{message}</p>}
    </section>
  );
}
