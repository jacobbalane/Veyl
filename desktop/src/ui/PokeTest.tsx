import { useEffect, useState } from 'react';
import { startPokeTest, stopPokeTest, watchGestures } from '../api/native';
import type { PokeTestResult } from '../api/types';
import { copy, pokeAdvice, pokeTestCopy as text } from '../copy';
import { Modal } from './Connection';

/** The poke test counts at most this many pokes, within this time (Rust counts the same). */
export const pokeTestPokes = 10;
export const pokeTestTime = 20_000;

/**
 * Runs a poke test while mounted: Rust counts pokes and never runs actions, and the test pokes
 * that arrive are counted here for a live counter. With a limit or a time, the test ends by
 * itself and `result` holds Rust's count. Unmounting always stops the test.
 */
export function usePokeTest({ limit, time }: { limit?: number; time?: number } = {}) {
  const [seen, setSeen] = useState(0);
  const [result, setResult] = useState<PokeTestResult | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    let stopped = false;
    let count = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let unlisten: (() => void) | undefined;
    const stop = async (show: boolean) => {
      if (stopped) return;
      stopped = true;
      clearTimeout(timer);
      unlisten?.();
      let value: PokeTestResult;
      try {
        value = await stopPokeTest();
      } catch {
        value = { seen: count, of: limit ?? pokeTestPokes };
      }
      if (active && show) setResult(value);
    };
    // Listen first, so no poke between starting and listening goes uncounted.
    void watchGestures((gesture) => {
      if (stopped || !gesture.testOnly) return;
      count = limit == null ? count + 1 : Math.min(count + 1, limit);
      if (active) setSeen(count);
      if (limit != null && count >= limit) void stop(true);
    })
      .then(
        (stopListening) => {
          if (stopped) stopListening();
          else unlisten = stopListening;
        },
        () => {},
      )
      .then(async () => {
        if (stopped) return;
        try {
          await startPokeTest();
        } catch {
          if (active) setFailed(true);
          return;
        }
        if (time != null && !stopped) timer = setTimeout(() => void stop(true), time);
      });
    return () => {
      active = false;
      void stop(false);
    };
  }, [limit, time]);
  return { seen, result, failed };
}

/** "Test pokes": ten pokes in 20 s, then how many Veyl saw and what to try. */
export function PokeTestDialog({ onClose }: { onClose: () => void }) {
  const { seen, result, failed } = usePokeTest({ limit: pokeTestPokes, time: pokeTestTime });
  const done = result != null || failed;
  return (
    <Modal title={text.title} onCancel={onClose} className="poke-test">
      {failed ? (
        <p role="alert">{text.failed}</p>
      ) : result ? (
        <div role="status" className="poke-result">
          <strong>{text.result(result.seen, result.of)}</strong>
          <p>{pokeAdvice(result.seen)}</p>
        </div>
      ) : (
        <>
          <p>{text.instructions}</p>
          <p className="poke-counter" role="status" aria-live="polite">
            {text.counter(seen, pokeTestPokes)}
          </p>
        </>
      )}
      <div className="dialog-buttons">
        <button className={done ? 'primary' : undefined} onClick={onClose}>
          {done ? copy.close : text.cancel}
        </button>
      </div>
    </Modal>
  );
}
