import { useEffect, useReducer, useRef, useState, type ReactNode } from 'react';
import { AudioLines } from 'lucide-react';
import { subscribeLive, watchEngine, watchGestures } from '../api/native';
import type { EngineView, Meter } from '../api/types';
import { copy, engineStatus, liveCopy, pcCopy, recordingCopy, tipText, warningText } from '../copy';
import { drawEcho, EchoHistory } from './echo-view';
import {
  emptyLive,
  gestureLine,
  handLine,
  liveReducer,
  meterBars,
  meterDetail,
  meterLabel,
  meterPercent,
} from './live-model';

/** Engine events, gestures and live frames for the Live tab, kept while other tabs are open. */
export function useLive(initial?: EngineView) {
  const [view, dispatch] = useReducer(liveReducer, emptyLive);
  const [history] = useState(() => new EchoHistory());
  useEffect(() => {
    if (initial) dispatch({ type: 'initial', engine: initial });
  }, [initial]);
  useEffect(() => {
    let active = true;
    const stops: (() => void)[] = [];
    const keep = (stop: () => void) => {
      if (active) stops.push(stop);
      else stop();
    };
    // If a subscription fails, the Live tab keeps its empty view instead of guessing.
    void watchEngine((engine) => {
      if (!active) return;
      if (engine.state === 'no_audio') history.clear();
      dispatch({ type: 'engine', engine });
    })
      .then(keep)
      .catch(() => {});
    void watchGestures((gesture) => {
      if (!active) return;
      history.mark(gesture.at);
      dispatch({ type: 'gesture', gesture });
    })
      .then(keep)
      .catch(() => {});
    // A new subscription replaces the previous one, so there is nothing to stop.
    void subscribeLive((frame) => {
      if (!active) return;
      history.push(frame, performance.now());
      dispatch({ type: 'frame', frame });
    }).catch(() => {});
    return () => {
      active = false;
      stops.forEach((stop) => stop());
    };
  }, [history]);
  return { ...view, history };
}
export type LiveState = ReturnType<typeof useLive>;

const reducedMotion = () =>
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/**
 * Draws the echo view. Normally it redraws every display frame and slides the picture between
 * live frames; with reduced motion it redraws ten times a second and only moves when a frame
 * arrives.
 */
function EchoCanvas({ history }: { history: EchoHistory }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    let frame = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let drawn = { version: 0, now: Number.NaN, width: 0, height: 0 };
    const draw = (reduced: boolean) => {
      if (history.version === 0) return;
      const width = element.clientWidth;
      const height = element.clientHeight;
      // Slide at most one frame interval past the newest frame, so a late frame never jumps back.
      const now = reduced
        ? history.latest
        : history.latest + Math.min(performance.now() - history.arrived, 1000 / 30);
      if (
        drawn.version === history.version &&
        drawn.now === now &&
        drawn.width === width &&
        drawn.height === height
      )
        return;
      const context = element.getContext('2d');
      if (!context || width === 0 || height === 0) return;
      const scale = window.devicePixelRatio || 1;
      if (element.width !== Math.round(width * scale)) element.width = Math.round(width * scale);
      if (element.height !== Math.round(height * scale))
        element.height = Math.round(height * scale);
      context.setTransform(scale, 0, 0, scale, 0, 0);
      const style = getComputedStyle(element);
      drawEcho(context, history, now, width, height, {
        aqua: style.getPropertyValue('--aqua').trim() || '#4fd8c4',
        text: style.getPropertyValue('--text').trim() || '#e8eef5',
        label: style.getPropertyValue('--accent-text').trim() || '#4fd8c4',
      });
      drawn = { version: history.version, now, width, height };
    };
    const loop = () => {
      const reduced = reducedMotion();
      if (reduced) timer = setTimeout(loop, 100);
      else frame = requestAnimationFrame(loop);
      draw(reduced);
    };
    loop();
    return () => {
      cancelAnimationFrame(frame);
      clearTimeout(timer);
    };
  }, [history]);
  return <canvas ref={canvas} className="echo-canvas" aria-hidden="true" />;
}

function ScoreRing({ meter }: { meter: Meter | null }) {
  const score = meter?.score ?? 0;
  const label = meterLabel(meter);
  return (
    <div className="score" data-label={meter?.label ?? 'none'}>
      <svg viewBox="0 0 120 120" aria-hidden="true">
        <circle className="score-track" cx="60" cy="60" r="54" pathLength={100} />
        <circle
          className="score-value"
          cx="60"
          cy="60"
          r="54"
          pathLength={100}
          strokeDasharray={`${Math.min(Math.max(score, 0), 100)} 100`}
          transform="rotate(-90 60 60)"
        />
      </svg>
      <span>
        <strong>{score}</strong>
        <small>{label}</small>
      </span>
    </div>
  );
}

function MeterCard({
  meter,
  tip,
  children,
}: {
  meter: Meter | null;
  tip: string;
  children?: ReactNode;
}) {
  const detail = meterDetail(meter);
  return (
    <section className="panel meter" aria-label={liveCopy.soundQuality}>
      <div className="eyebrow">{liveCopy.soundQuality}</div>
      <ScoreRing meter={meter} />
      {meterBars.map(([key, name]) => {
        const value = meter ? meterPercent(meter[key]) : 0;
        return (
          <div className="meter-factor" key={key}>
            <span>{name}</span>
            <span className="meter-value">{value}</span>
            <div
              role="meter"
              aria-label={name}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={value}
            >
              <i style={{ width: `${value}%` }} />
            </div>
          </div>
        );
      })}
      {detail && <p className="meter-detail">{detail}</p>}
      <p className="tip">{tip}</p>
      {children}
    </section>
  );
}

export function Live({
  live,
  pc,
  tip,
  canTest,
  onTest,
  canRecord,
  onRecord,
  children,
}: {
  live: LiveState;
  pc: boolean;
  /** The tip to show before the engine sends a meter. */
  tip: string;
  /** The poke test needs a listening source. */
  canTest: boolean;
  onTest: () => void;
  canRecord: boolean;
  onRecord: () => void;
  /** Extra controls for the meter card, such as asking the phone to start. */
  children?: ReactNode;
}) {
  const { engine, gestures, history } = live;
  const warning = warningText(engine.warning, pc);
  const newest = gestures[0]?.at;
  return (
    <div className="live-grid">
      <MeterCard meter={engine.meter} tip={engine.meter ? tipText(engine.meter.tip, pc) : tip}>
        {children}
      </MeterCard>
      <section className="panel echo">
        <div className="section-title">
          <h1>{copy.echoTitle}</h1>
          <span className="hand-line">{handLine(engine.hand)}</span>
        </div>
        <div className="echo-map">
          <EchoCanvas history={history} />
          <span className="axis">{pc ? pcCopy.axis : copy.echoAxis}</span>
          <div className="distance-labels" aria-hidden="true">
            <span>60 cm</span>
            <span>40</span>
            <span>20</span>
            <span>0</span>
          </div>
          {!engine.hand.present && (
            <div className="echo-empty">
              <AudioLines size={35} aria-hidden="true" />
              <p>{pc ? pcCopy.empty : copy.echoEmpty}</p>
            </div>
          )}
        </div>
        <div className="engine-status" role="status" aria-live="polite">
          <strong>{engineStatus(engine)}</strong>
          {warning && <span>{warning}</span>}
        </div>
      </section>
      <section className="panel poke-log">
        <h2 id="recent-pokes">{copy.pokesTitle}</h2>
        <ol aria-labelledby="recent-pokes" aria-live="polite">
          {gestures.length === 0 ? (
            <li className="empty">{copy.pokesEmpty}</li>
          ) : (
            gestures.map((gesture) => (
              <li
                key={gesture.at}
                className={gesture.at === newest ? 'fresh' : ''}
                data-acted={gesture.acted}
              >
                {gestureLine(gesture)}
                {gesture.testOnly && <small>{liveCopy.test}</small>}
              </li>
            ))
          )}
        </ol>
        <div className="button-row">
          <button disabled={!canTest} onClick={onTest}>
            {liveCopy.testPokes}
          </button>
          <button disabled={!canRecord} onClick={onRecord}>
            {recordingCopy.title}
          </button>
        </div>
      </section>
    </div>
  );
}
