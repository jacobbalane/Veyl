import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Smartphone, X } from 'lucide-react';
import mark from '../../../assets/brand/veyl-mark.svg';
import { browserPreview, pairPhone } from '../api/native';
import type { LinkSnapshot } from '../api/types';
import {
  connectionError,
  copy,
  engineStatus,
  hotkeyLabel,
  linkCopy,
  onboardingCopy as text,
  pcCopy,
} from '../copy';
import { connectionLabel } from './Connection';
import type { LiveState } from './Live';
import { handLine } from './live-model';
import { keepFocusInside } from './focus';
import { usePokeTest } from './PokeTest';
import type { SettingsState } from './Settings';
import { PcDevices, SourceOptions, type SourceControls } from './Sources';
import { onboardingPath, type OnboardingStep } from './onboarding-steps';
export type { OnboardingStep } from './onboarding-steps';

/** Onboarding's practice step: a live poke test, so actions stay off while the user tries. */
function Practice({ pc, live }: { pc: boolean; live: LiveState }) {
  const { seen } = usePokeTest();
  return (
    <>
      <p>{pc ? text.practicePc : text.practicePhone}</p>
      <div className="practice" role="status" aria-live="polite" data-seen={seen > 0}>
        {seen > 0 ? (
          <strong className="practice-nice">{text.nice}</strong>
        ) : (
          <>
            <strong>{handLine(live.engine.hand)}</strong>
            <span>{engineStatus(live.engine)}</span>
          </>
        )}
      </div>
    </>
  );
}

/** Onboarding's pairing step: nearby phones with "Pair", and how the connection is doing. */
function Pairing({ link }: { link: LinkSnapshot }) {
  const [working, setWorking] = useState(false);
  const [error, setError] = useState('');
  const busy = working || ['connecting', 'handshaking', 'pairing'].includes(link.connection.state);
  const pairError =
    link.pairing?.stage === 'failed' ? connectionError(link.pairing.error, true) : '';
  return (
    <>
      <p>{text.pair}</p>
      {link.phones.nearby.length > 0 && (
        <ul className="phone-list">
          {link.phones.nearby.map((phone) => (
            <li key={phone.id}>
              <Smartphone size={21} aria-hidden="true" />
              <div className="phone-details">
                <strong>{phone.name}</strong>
                <p>{phone.model}</p>
              </div>
              <button
                disabled={busy || browserPreview}
                onClick={() => {
                  setWorking(true);
                  setError('');
                  void pairPhone({ nearbyId: phone.id })
                    .catch(() => setError(linkCopy.failedRequest))
                    .finally(() => setWorking(false));
                }}
              >
                {linkCopy.pair}
              </button>
            </li>
          ))}
        </ul>
      )}
      <p className="onboarding-status" role="status">
        {link.pairing?.stage === 'done'
          ? linkCopy.pairedWith(link.pairing.phoneName)
          : connectionLabel(link.connection, link.phones.nearby.length)}
      </p>
      {(error || pairError) && <p role="alert">{error || pairError}</p>}
    </>
  );
}

/**
 * The first-run guide (§7.12): welcome, how to use Veyl, the phone or PC path, a practice poke
 * and the actions switch. Finish saves `onboardingDone`; closing it early shows it again at the
 * next start.
 */
export function Onboarding({
  sources,
  link,
  live,
  settings,
  initialStep = 'welcome',
  onClose,
}: {
  sources: SourceControls;
  link: LinkSnapshot;
  live: LiveState;
  settings: SettingsState;
  initialStep?: OnboardingStep;
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const [step, setStep] = useState<OnboardingStep>(initialStep);
  const [chosen, setChosen] = useState<'phone' | 'pc' | null>(null);
  const mode = chosen ?? (sources.source.mode === 'pc' ? 'pc' : 'phone');
  const path = onboardingPath(mode);
  const index = Math.max(path.indexOf(step), 0);
  const hotkey = settings.value?.hotkey ?? 'Ctrl+Alt+V';
  const go = (offset: number) =>
    setStep(path[Math.min(Math.max(index + offset, 0), path.length - 1)]);
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => element?.close();
  }, []);
  // Screen readers and keyboards start each step at its title.
  useEffect(() => heading.current?.focus(), [step]);
  const finish = async () => {
    if (await settings.save({ onboardingDone: true })) onClose();
  };
  const next = (label: string = text.next) => (
    <button className="primary" onClick={() => go(1)}>
      {label}
    </button>
  );
  const steps: Record<OnboardingStep, { title: string; body: ReactNode; action?: ReactNode }> = {
    welcome: {
      title: text.welcomeTitle,
      body: (
        <>
          <img className="onboarding-mark" src={mark} width="72" height="72" alt="" />
          <p>{text.welcome}</p>
        </>
      ),
      action: next(text.start),
    },
    choose: {
      title: pcCopy.choose,
      body: (
        <>
          <SourceOptions
            controls={sources}
            onChosen={(choice) => {
              setChosen(choice);
              setStep(onboardingPath(choice)[2]);
            }}
          />
          {sources.error && <p role="alert">{sources.error}</p>}
        </>
      ),
    },
    'phone-app': {
      title: text.phoneAppTitle,
      body: (
        <>
          <p>{text.phoneApp}</p>
          {/* On a narrow window the address wraps after its slash, never inside a word. */}
          <p className="onboarding-url">
            {text.website.slice(0, text.website.indexOf('/') + 1)}
            <wbr />
            {text.website.slice(text.website.indexOf('/') + 1)}
          </p>
        </>
      ),
      action: next(),
    },
    pair: { title: text.pairTitle, body: <Pairing link={link} />, action: next() },
    place: { title: text.placeTitle, body: <p>{text.place}</p>, action: next() },
    'phone-play': {
      title: text.playTitle,
      body: (
        <>
          <p>{text.phonePlay}</p>
          <p className="onboarding-status" role="status">
            {connectionLabel(link.connection, link.phones.nearby.length)}
          </p>
        </>
      ),
      action: next(),
    },
    'pc-devices': {
      title: text.pcDevicesTitle,
      body: (
        <div className="onboarding-devices">
          <PcDevices controls={sources} placement={false} />
          {sources.error && <p role="alert">{sources.error}</p>}
        </div>
      ),
      action: next(),
    },
    'pc-play': {
      title: text.playTitle,
      body: (
        <>
          <p>{text.pcPlay}</p>
          <p className="onboarding-status" role="status">
            {sources.source.state === 'running'
              ? pcCopy.listening
              : sources.source.state === 'starting'
                ? pcCopy.starting
                : pcCopy.noSound}
          </p>
        </>
      ),
      action: next(),
    },
    practice: {
      title: text.practiceTitle,
      body: <Practice pc={mode === 'pc'} live={live} />,
      action: next(),
    },
    actions: {
      title: text.actionsTitle,
      body: (
        <>
          <p>{text.actions(hotkey)}</p>
          <p className="onboarding-hotkey">
            <kbd>{hotkeyLabel(hotkey)}</kbd>
          </p>
          {settings.errors.onboardingDone && <p role="alert">{settings.errors.onboardingDone}</p>}
        </>
      ),
      action: (
        <button
          className="primary"
          disabled={settings.pending.onboardingDone}
          onClick={() => void finish()}
        >
          {text.finish}
        </button>
      ),
    },
  };
  const current = steps[path[index]];
  return (
    <dialog
      ref={dialog}
      className="onboarding"
      aria-labelledby="onboarding-title"
      onKeyDown={keepFocusInside}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <div className="onboarding-card" data-step={path[index]}>
        <div className="onboarding-top">
          <ol className="onboarding-progress" aria-hidden="true">
            {path.map((id, position) => (
              <li
                key={id}
                data-state={position < index ? 'done' : position === index ? 'now' : ''}
              />
            ))}
          </ol>
          <button className="icon-button" aria-label={copy.close} onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </div>
        <h2 id="onboarding-title" ref={heading} tabIndex={-1}>
          {current.title}
        </h2>
        <div className="onboarding-body">{current.body}</div>
        <div className="dialog-buttons">
          {index > 0 && <button onClick={() => go(-1)}>{text.back}</button>}
          {current.action}
        </div>
      </div>
    </dialog>
  );
}
