import { useEffect, useRef, useState } from 'react';
import * as native from '../api/native';
import type { RecordingInfo } from '../api/types';
import { linkCopy, recordingCopy as text } from '../copy';
import { Modal } from './Connection';

export function shouldMarkPoke(event: KeyboardEvent, focused: boolean): boolean {
  const target = event.target;
  return (
    event.code === 'Space' &&
    !event.repeat &&
    focused &&
    !event.altKey &&
    !event.ctrlKey &&
    !event.metaKey &&
    !(
      target instanceof HTMLElement &&
      (target.isContentEditable || target.closest('input, textarea, select, button, a, dialog'))
    )
  );
}
export function useRecordings() {
  const [info, setInfo] = useState<RecordingInfo | null>(null);
  const [saved, setSaved] = useState<RecordingInfo[]>([]);
  const [dialog, setDialog] = useState(false);
  const [deleting, setDeleting] = useState<RecordingInfo | null>(null);
  const [seconds, setSeconds] = useState<30 | 60 | 120>(30);
  const [marks, setMarks] = useState(true);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const revision = useRef(0);
  const deleted = useRef(new Set<string>());
  const accept = (value: RecordingInfo | null) => {
    revision.current += 1;
    setInfo(value);
    if (value) setSaved((previous) => [value, ...previous.filter((info) => info.id !== value.id)]);
  };
  useEffect(() => {
    if (native.browserPreview) return;
    let active = true;
    let stop: (() => void) | undefined;
    const initialRevision = revision.current;
    void native
      .watchRecording((value) => {
        if (!active) return;
        accept(value);
      })
      .then(async (unlisten) => {
        if (!active) {
          unlisten();
          return;
        }
        stop = unlisten;
        const [current, recordings] = await Promise.allSettled([
          native.getRecording(),
          native.listRecordings(),
        ]);
        if (active) {
          if (recordings.status === 'fulfilled')
            setSaved((previous) => {
              const merged = new Map(recordings.value.map((info) => [info.id, info]));
              for (const info of previous) merged.set(info.id, info);
              return [...merged.values()].filter((info) => !deleted.current.has(info.id));
            });
          if (current.status === 'fulfilled' && revision.current === initialRevision)
            accept(current.value);
          if (current.status === 'rejected' || recordings.status === 'rejected')
            setError(text.loadFailed);
        }
      })
      .catch(() => {
        if (active) setError(text.loadFailed);
      });
    return () => {
      active = false;
      stop?.();
    };
  }, []);
  useEffect(() => {
    if (info?.state !== 'recording' || !info.markPokes) return;
    const key = (event: KeyboardEvent) => {
      if (!shouldMarkPoke(event, document.hasFocus())) return;
      event.preventDefault();
      void native.markPoke().catch(() => setError(text.saveFailed));
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [info?.state, info?.markPokes]);
  const start = async () => {
    revision.current += 1;
    setPending(true);
    setError('');
    try {
      accept(await native.startRecording(seconds, marks));
      setDialog(false);
    } catch (error) {
      setError(error === 'not_listening' ? text.notListening : text.saveFailed);
    } finally {
      setPending(false);
    }
  };
  const stop = async () => {
    revision.current += 1;
    setPending(true);
    setError('');
    try {
      accept(await native.stopRecording());
    } catch {
      setError(text.saveFailed);
    } finally {
      setPending(false);
    }
  };
  const remove = async () => {
    if (!deleting) return;
    setPending(true);
    setError('');
    try {
      await native.deleteRecording(deleting.id);
      deleted.current.add(deleting.id);
      const recordings = await native.listRecordings();
      setSaved(recordings);
      if (info?.id === deleting.id) accept(null);
      setDeleting(null);
    } catch {
      setError(text.deleteFailed);
    } finally {
      setPending(false);
    }
  };
  const open = async () => {
    try {
      await native.openRecordingsFolder();
    } catch {
      setError(text.openFailed);
    }
  };
  return {
    info,
    saved,
    dialog,
    setDialog,
    deleting,
    setDeleting,
    seconds,
    setSeconds,
    marks,
    setMarks,
    pending,
    error,
    start,
    stop,
    remove,
    open,
  };
}
export type RecordingsState = ReturnType<typeof useRecordings>;

export function RecordingStatus({ recording }: { recording: RecordingsState }) {
  const { info, error, pending } = recording;
  return (
    <>
      {info && (
        <section
          className={`recording-status ${info.state === 'recording' ? 'is-recording' : ''}`}
          aria-live="polite"
        >
          <div>
            <strong>
              {info.state === 'recording'
                ? text.banner(info.remainingSeconds)
                : info.state === 'saved'
                  ? text.saved
                  : text.partial}
            </strong>
            {info.state === 'recording' && info.markPokes && <p>{text.spaceHint}</p>}
          </div>
          <div className="button-row">
            {info.state === 'recording' ? (
              <button disabled={pending} onClick={() => void recording.stop()}>
                {text.stop}
              </button>
            ) : (
              <>
                <button onClick={() => void recording.open()}>{text.open}</button>
                <button onClick={() => recording.setDeleting(info)}>{text.delete}</button>
              </>
            )}
          </div>
        </section>
      )}
      {error && <p role="alert">{error}</p>}
    </>
  );
}
export function RecordingDialogs({ recording }: { recording: RecordingsState }) {
  return (
    <>
      {recording.dialog && (
        <Modal
          title={text.title}
          onCancel={() => {
            if (!recording.pending) recording.setDialog(false);
          }}
        >
          <p>{text.consent(recording.seconds)}</p>
          <label className="recording-duration">
            {text.duration}
            <select
              value={recording.seconds}
              disabled={recording.pending}
              onChange={(event) =>
                recording.setSeconds(Number(event.target.value) as 30 | 60 | 120)
              }
            >
              {[30, 60, 120].map((seconds) => (
                <option key={seconds} value={seconds}>
                  {text.seconds(seconds)}
                </option>
              ))}
            </select>
          </label>
          <label className="recording-checkbox">
            <input
              type="checkbox"
              checked={recording.marks}
              disabled={recording.pending}
              onChange={(event) => recording.setMarks(event.target.checked)}
            />
            {text.markPokes}
          </label>
          {recording.error && <p role="alert">{recording.error}</p>}
          <div className="dialog-buttons">
            <button
              className="primary"
              disabled={recording.pending}
              onClick={() => void recording.start()}
            >
              {text.start}
            </button>
            <button disabled={recording.pending} onClick={() => recording.setDialog(false)}>
              {linkCopy.cancel}
            </button>
          </div>
        </Modal>
      )}
      {recording.deleting && (
        <Modal
          title={text.deleteTitle}
          onCancel={() => {
            if (!recording.pending) recording.setDeleting(null);
          }}
        >
          <p>{recordingTime(recording.deleting.id)}</p>
          <p>{text.deleteMessage}</p>
          {recording.error && <p role="alert">{recording.error}</p>}
          <div className="dialog-buttons">
            <button disabled={recording.pending} onClick={() => void recording.remove()}>
              {text.delete}
            </button>
            <button disabled={recording.pending} onClick={() => recording.setDeleting(null)}>
              {linkCopy.cancel}
            </button>
          </div>
        </Modal>
      )}
    </>
  );
}
function recordingTime(id: string): string {
  const date = new Date(
    `${id.slice(0, 4)}-${id.slice(4, 6)}-${id.slice(6, 8)}T${id.slice(9, 11)}:${id.slice(11, 13)}:${id.slice(13, 15)}Z`,
  );
  return Number.isNaN(date.valueOf()) ? text.title : date.toLocaleString();
}
export function SavedRecordings({
  recording,
  canRecord,
}: {
  recording: RecordingsState;
  canRecord: boolean;
}) {
  return (
    <section className="saved-recordings">
      <h2>{text.recordings}</h2>
      <button
        disabled={!canRecord || recording.info?.state === 'recording'}
        onClick={() => recording.setDialog(true)}
      >
        {text.title}
      </button>
      {recording.saved.length === 0 && <p>{text.empty}</p>}
      {recording.saved
        .filter((info) => info.state !== 'recording')
        .map((info) => (
          <div className="saved-recording" key={info.id}>
            <div>
              <strong>{recordingTime(info.id)}</strong>
              <p>{info.state === 'saved' ? text.saved : text.partial}</p>
            </div>
            <button onClick={() => void recording.open()}>{text.open}</button>
            <button onClick={() => recording.setDeleting(info)}>{text.delete}</button>
          </div>
        ))}
    </section>
  );
}
