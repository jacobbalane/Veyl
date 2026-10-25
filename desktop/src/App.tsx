import { useEffect, useState } from 'react';
import { CircleHelp, Radio, Settings, Smartphone } from 'lucide-react';
import mark from '../../assets/brand/veyl-mark.svg';
import {
  browserPreview,
  getAppState,
  previewView,
  uiReady,
  watchLink,
  requestPhoneStart,
  type AppState,
} from './api/native';
import { emptyLink, type LinkSnapshot, type LinkUpdate } from './api/types';
import { copy, connectionError, linkCopy, pcCopy, pcTip, tipText } from './copy';
import { SourceChoice, useSources } from './ui/Sources';
import { Phones, PairingDialog, connectionLabel } from './ui/Connection';
import { RecordingStatus, RecordingDialogs, useRecordings } from './ui/Recordings';
import { Live, useLive } from './ui/Live';
import { ActionsNotices, ActionsSwitch, useActions } from './ui/Actions';
import { SettingsPanel, useSettings } from './ui/Settings';
import { PokeTestDialog } from './ui/PokeTest';
import { Onboarding, type OnboardingStep } from './ui/Onboarding';
import { Help } from './ui/Help';

export const tabs = [
  { name: 'Live', icon: Radio },
  { name: 'Phones', icon: Smartphone },
  { name: 'Settings', icon: Settings },
  { name: 'Help', icon: CircleHelp },
] as const;
export type Tab = (typeof tabs)[number]['name'];

export function App() {
  const [tab, setTab] = useState<Tab>('Live');
  const [app, setApp] = useState<AppState>();
  const [error, setError] = useState('');
  const [pokeTest, setPokeTest] = useState(false);
  // Closing onboarding early hides it until the next start; Finish saves that it is done.
  const [onboardingClosed, setOnboardingClosed] = useState(false);
  const [onboardingStep, setOnboardingStep] = useState<OnboardingStep>();
  const link = app?.link ?? emptyLink;
  const recordings = useRecordings();
  const sources = useSources(app?.source);
  const live = useLive(app?.engine);
  const settings = useSettings(app?.settings);
  const actions = useActions(app?.actionsEnabled);
  const pc = sources.source.mode === 'pc';
  const listening = pc
    ? sources.source.state === 'running'
    : link.connection.state === 'connected' && link.connection.capture?.state === 'running';
  const canRecord = !browserPreview && listening;
  const onboarding = !!settings.value && !settings.value.onboardingDone && !onboardingClosed;
  const hotkey = settings.value?.hotkey ?? 'Ctrl+Alt+V';
  const updateLink = (link: LinkSnapshot, update: LinkUpdate): LinkSnapshot =>
    'capture' in update
      ? { ...link, connection: { ...link.connection, capture: update.capture } }
      : { ...link, ...update };
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    let loaded = false;
    const buffered: LinkUpdate[] = [];
    void watchLink((update) => {
      if (!active) return;
      if (!loaded) buffered.push(update);
      else setApp((state) => (state ? { ...state, link: updateLink(state.link, update) } : state));
    })
      .then(async (unlisten) => {
        if (!active) {
          unlisten();
          return;
        }
        stop = unlisten;
        const [state, view] = await Promise.all([getAppState(), previewView()]);
        if (active) {
          for (const update of buffered) state.link = updateLink(state.link, update);
          loaded = true;
          // The browser preview can open on a tab, a dialog or an onboarding step.
          if (view?.tab) setTab(view.tab);
          if (view?.pokeTest) setPokeTest(true);
          if (view?.onboarding) setOnboardingStep(view.onboarding);
          setApp(state);
        }
      })
      .catch(() => {
        if (active) setError(copy.loadFailed);
      });
    // Showing the native window follows the first painted frame, even if state loading fails.
    const frame = requestAnimationFrame(() => {
      void uiReady().catch(() => setError(copy.windowFailed));
    });
    return () => {
      active = false;
      stop?.();
      cancelAnimationFrame(frame);
    };
  }, []);
  const capture = link.connection.capture;

  return (
    <div className="app-shell">
      <header className="app-header">
        <a
          href="#live"
          className="brand"
          onClick={() => setTab('Live')}
          aria-label={copy.brandLink}
        >
          <img src={mark} width="42" height="42" alt="" />
          <span>veyl</span>
        </a>
        <span
          className="connection"
          aria-live="polite"
          data-connected={pc ? listening : link.connection.state === 'connected'}
        >
          <i />
          {pc
            ? sources.source.state === 'running'
              ? pcCopy.listening
              : sources.source.state === 'starting'
                ? pcCopy.starting
                : pcCopy.noSound
            : connectionLabel(link.connection, link.phones.nearby.length)}
        </span>
        <ActionsSwitch actions={actions} hotkey={hotkey} ready={!!app} />
      </header>
      <nav className="tabs" aria-label={copy.navigation}>
        {tabs.map(({ name, icon: Icon }) => (
          <button
            key={name}
            aria-current={tab === name ? 'page' : undefined}
            onClick={() => setTab(name)}
          >
            <Icon size={17} aria-hidden="true" />
            {name}
          </button>
        ))}
        {browserPreview && <span className="preview-badge">{copy.preview}</span>}
      </nav>
      <ActionsNotices actions={actions} />
      <main>
        {error && <p role="alert">{error}</p>}
        {sources.error && tab !== 'Settings' && !onboarding && <p role="alert">{sources.error}</p>}
        {app && !sources.source.choiceDone && !onboarding && <SourceChoice controls={sources} />}
        <RecordingStatus recording={recordings} />
        {!pc && link.connection.error && connectionError(link.connection.error) && (
          <p role="alert">{connectionError(link.connection.error)}</p>
        )}
        {!pc && capture?.state === 'stopped' && capture.reason === 'screen_locked' && (
          <p role="alert">{linkCopy.screenLocked}</p>
        )}
        {tab === 'Live' && (
          <Live
            live={live}
            pc={pc}
            tip={pc ? pcTip(sources.source.tip) : tipText('capture_off', false)}
            canTest={listening}
            onTest={() => setPokeTest(true)}
            canRecord={canRecord && recordings.info?.state !== 'recording'}
            onRecord={() => recordings.setDialog(true)}
          >
            {!pc && link.connection.state === 'connected' && capture?.state !== 'running' && (
              <button
                disabled={browserPreview}
                onClick={() =>
                  void requestPhoneStart().catch(() => setError(linkCopy.failedRequest))
                }
              >
                {linkCopy.askPhoneToStart}
              </button>
            )}
          </Live>
        )}
        {tab === 'Phones' &&
          (pc ? (
            <section className="panel">
              <p>{pcCopy.phonesNote}</p>
            </section>
          ) : (
            <Phones link={link} />
          ))}
        {tab === 'Settings' && <SettingsPanel settings={settings} sources={sources} />}
        {tab === 'Help' && (
          <Help
            pc={pc}
            version={app?.version ?? '0.0.1'}
            recordings={recordings}
            canRecord={canRecord}
          />
        )}
      </main>
      {onboarding && (
        <Onboarding
          sources={sources}
          link={link}
          live={live}
          settings={settings}
          initialStep={onboardingStep}
          onClose={() => setOnboardingClosed(true)}
        />
      )}
      {!pc && (
        <PairingDialog key={link.pairing?.sessionId} pairing={link.pairing} onError={setError} />
      )}
      {pokeTest && <PokeTestDialog onClose={() => setPokeTest(false)} />}
      <RecordingDialogs recording={recordings} />
      <footer>
        {copy.tagline}
        <span>{copy.footer}</span>
      </footer>
    </div>
  );
}
