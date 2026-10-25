import {
  emptyEngine,
  type EngineView,
  type GestureEvent,
  type HandView,
  type LatencyView,
  type LiveFrame,
  type Meter,
} from '../api/types';
import { copy, liveCopy } from '../copy';

/** The gesture log keeps this many pokes, newest first. */
export const gestureLimit = 10;

export interface LiveView {
  engine: EngineView;
  gestures: GestureEvent[];
  /** True once an engine event or a live frame arrived, so the app state can't replace it. */
  received: boolean;
}
export const emptyLive: LiveView = { engine: emptyEngine, gestures: [], received: false };

export type LiveAction =
  | { type: 'initial'; engine: EngineView }
  | { type: 'engine'; engine: EngineView }
  | { type: 'frame'; frame: LiveFrame }
  | { type: 'gesture'; gesture: GestureEvent };

/**
 * Engine events carry the state, suspension and warning; live frames carry the hand's distance
 * and the meter's values, which change too often for events. A frame only causes a render when
 * something the Live tab shows in words or numbers changed.
 */
export function liveReducer(view: LiveView, action: LiveAction): LiveView {
  switch (action.type) {
    case 'initial':
      return view.received ? view : { ...view, engine: action.engine };
    case 'engine':
      return { ...view, received: true, engine: action.engine };
    case 'frame': {
      const { engine } = view;
      const hand = sameHand(engine.hand, action.frame.hand) ? engine.hand : action.frame.hand;
      const meter =
        action.frame.meter && !sameMeter(engine.meter, action.frame.meter)
          ? action.frame.meter
          : engine.meter;
      if (view.received && hand === engine.hand && meter === engine.meter) return view;
      return { ...view, received: true, engine: { ...engine, hand, meter } };
    }
    case 'gesture':
      return { ...view, gestures: [action.gesture, ...view.gestures].slice(0, gestureLimit) };
  }
}

/** A meter value (0–1) as a whole number from 0 to 100. */
export function meterPercent(value: number): number {
  return Number.isFinite(value) ? Math.round(Math.min(Math.max(value, 0), 1) * 100) : 0;
}
export function meterLabel(meter: Meter | null): string {
  return liveCopy.labels[meter?.label ?? 'none'] ?? liveCopy.labels.none;
}
/** "Detail ≈ 1.8 cm", or nothing when the sound is too dull to measure detail. */
export function meterDetail(meter: Meter | null): string {
  return meter?.detailCm != null && Number.isFinite(meter.detailCm)
    ? liveCopy.detail(meter.detailCm.toFixed(1))
    : '';
}
export const meterBars = [
  ['high', liveCopy.high],
  ['volume', liveCopy.volume],
  ['steady', liveCopy.steady],
  ['clarity', liveCopy.clarity],
] as const;
/** Everything the meter card shows, so equal keys render the same card. */
function meterKey(meter: Meter): string {
  return [
    meter.score,
    meter.label,
    meter.tip,
    meterDetail(meter),
    ...meterBars.map(([key]) => meterPercent(meter[key])),
  ].join();
}
function sameMeter(a: Meter | null, b: Meter): boolean {
  return a != null && meterKey(a) === meterKey(b);
}

/** The hand's distance in whole centimetres, or null when no hand is seen. */
export function handCm(hand: HandView): number | null {
  return hand.present && hand.distanceCm != null && Number.isFinite(hand.distanceCm)
    ? Math.round(hand.distanceCm)
    : null;
}
function sameHand(a: HandView, b: HandView): boolean {
  return a.present === b.present && handCm(a) === handCm(b);
}
export function handLine(hand: HandView): string {
  const cm = handCm(hand);
  return cm == null ? copy.noHand : liveCopy.handSeen(cm);
}
export function gestureLine(gesture: GestureEvent): string {
  return liveCopy.poke(Math.round(gesture.distanceCm));
}

/** "148 / 212 ms" for Diagnostics, or a dash before the first measured poke. */
export function formatLatency(latency: LatencyView): string {
  return latency.medianMs == null || latency.p95Ms == null
    ? liveCopy.missing
    : `${Math.round(latency.medianMs)} / ${Math.round(latency.p95Ms)} ms`;
}
