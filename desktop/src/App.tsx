import { useEffect, useState } from 'react';
import { AudioLines, CircleHelp, Radio, Settings, Smartphone } from 'lucide-react';
import mark from '../../assets/brand/veyl-mark.svg';
import {
  browserPreview,
  getAppState,
  uiReady,
  watchLink,
  requestPhoneStart,
  type AppState,
} from './api/native';
import { emptyLink, type LinkSnapshot, type LinkUpdate } from './api/types';
import { copy, connectionError, linkCopy } from './copy';
import { Phones, PairingDialog, connectionLabel } from './ui/Connection';
import { Diagnostics } from './ui/Diagnostics';
import { RecordingStatus, RecordingDialogs, SavedRecordings, useRecordings } from './ui/Recordings';
import { recordingCopy } from './copy';

const tabs = [
  { name: 'Live', icon: Radio },
  { name: 'Phones', icon: Smartphone },
  { name: 'Settings', icon: Settings },
  { name: 'Help', icon: CircleHelp },
] as const;
type Tab = (typeof tabs)[number]['name'];

export function App() {
  const [tab, setTab] = useState<Tab>('Live');
  const [app, setApp] = useState<AppState>();
  const [error, setError] = useState('');
  const link = app?.link ?? emptyLink;
  const recordings = useRecordings();
  const canRecord =
    !browserPreview &&
    link.connection.state === 'connected' &&
    link.connection.capture?.state === 'running';
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
        const state = await getAppState();
        if (active) {
          for (const update of buffered) state.link = updateLink(state.link, update);
          loaded = true;
          setApp(state);
        }
      })
      .catch(() => {
        if (active) setError('Veyl could not load its settings.');
      });
    // Showing the native window follows the first painted frame, even if state loading fails.
    const frame = requestAnimationFrame(() => {
      void uiReady().catch(() => setError('Veyl could not open its window.'));
    });
    return () => {
      active = false;
      stop?.();
      cancelAnimationFrame(frame);
    };
  }, []);

  return (
    <div className="app-shell">
      <header className="app-header">
        <a
          href="#live"
          className="brand"
          onClick={() => setTab('Live')}
          aria-label="Veyl live view"
        >
          <img src={mark} width="42" height="42" alt="" />
          <span>veyl</span>
        </a>
        <span
          className="connection"
          aria-live="polite"
          data-connected={link.connection.state === 'connected'}
        >
          <i />
          {connectionLabel(link.connection, link.phones.nearby.length)}
        </span>
        <div className="actions" title={copy.actionsTooltip}>
          <span>
            Actions <b>Off</b>
          </span>
          <button role="switch" aria-label="Actions" aria-checked="false" disabled>
            <i />
          </button>
          <kbd>Ctrl+Alt+V</kbd>
        </div>
      </header>
      <nav className="tabs" aria-label="Veyl views">
        {tabs.map(({ name, icon: Icon }) => (
          <button
            key={name}
            aria-current={tab === name ? 'page' : undefined}
            onClick={() => setTab(name)}
          >
            <Icon size={17} />
            {name}
          </button>
        ))}
        {browserPreview && <span className="preview-badge">{copy.preview}</span>}
      </nav>
      <main>
        <div className="foundation">
          <AudioLines size={20} />
          <div>
            <strong>{copy.foundation}</strong>
            <p>{copy.foundationDetail}</p>
          </div>
          <span>v{app?.version ?? '0.0.1'}</span>
        </div>
        {error && <p role="alert">{error}</p>}
        <RecordingStatus recording={recordings} />
        {link.connection.error && connectionError(link.connection.error) && (
          <p role="alert">{connectionError(link.connection.error)}</p>
        )}
        {tab === 'Live' && (
          <div className="live-grid">
            <section className="panel meter">
              <div className="eyebrow">Sound quality</div>
              <div className="score">
                <span>
                  0<small>{copy.noSound}</small>
                </span>
              </div>
              {['High sounds', 'Volume', 'Steady', 'Clear echo'].map((name) => (
                <div className="meter-factor" key={name}>
                  <span>{name}</span>
                  <div />
                </div>
              ))}
              <p className="tip">Tap Start in Veyl on your phone.</p>
              {link.connection.state === 'connected' &&
                link.connection.capture?.state !== 'running' && (
                  <button
                    disabled={browserPreview}
                    onClick={() =>
                      void requestPhoneStart().catch(() => setError(linkCopy.failedRequest))
                    }
                  >
                    {linkCopy.askPhoneToStart}
                  </button>
                )}
            </section>
            <section className="panel echo">
              <div className="section-title">
                <h1>{copy.echoTitle}</h1>
                <span>{copy.noHand}</span>
              </div>
              <div className="echo-map">
                <span className="axis">{copy.echoAxis}</span>
                <div className="distance-labels">
                  <span>60 cm</span>
                  <span>40</span>
                  <span>20</span>
                  <span>0</span>
                </div>
                <div className="echo-empty">
                  <AudioLines size={35} />
                  <p>{copy.echoEmpty}</p>
                </div>
              </div>
            </section>
            <section className="panel poke-log" aria-live="polite">
              <h2>{copy.pokesTitle}</h2>
              <p>{copy.pokesEmpty}</p>
              <div className="button-row">
                <button disabled>Test pokes</button>
                <button
                  disabled={!canRecord || recordings.info?.state === 'recording'}
                  onClick={() => recordings.setDialog(true)}
                >
                  {recordingCopy.title}
                </button>
              </div>
            </section>
          </div>
        )}
        {tab === 'Phones' && <Phones link={link} />}
        {tab === 'Settings' && (
          <section className="panel settings">
            <h1>Settings</h1>
            <label>
              Sound source
              <select disabled>
                <option>With my phone</option>
                <option>With this PC</option>
              </select>
            </label>
            <label>
              Poke action
              <select disabled>
                <option>Click</option>
                <option>Play or pause</option>
                <option>Next track</option>
                <option>Space bar</option>
                <option>Nothing</option>
              </select>
            </label>
            <label>
              Sensitivity
              <select disabled>
                <option>Normal</option>
              </select>
            </label>
            <label>
              Distance range
              <select disabled>
                <option>Normal</option>
              </select>
            </label>
            <label>
              PC name
              <input value={app?.pcName ?? ''} disabled />
            </label>
            <p>{copy.foundationDetail}</p>
          </section>
        )}
        {tab === 'Help' && (
          <section className="panel help">
            <h1>About</h1>
            <p>Veyl · Version {app?.version ?? '0.0.1'}</p>
            <p>Touch the air. Your screen follows.</p>
            <h2>Privacy</h2>
            <p>Sound stays on your devices. Veyl has no accounts, analytics or cloud services.</p>
            <h2>Placing your phone</h2>
            <p>
              Lay it flat, screen up, in front of your monitor. Keep its speaker and microphones
              uncovered.
            </p>
            <a href="https://github.com/jacobbalane/Veyl" target="_blank" rel="noreferrer">
              Source code
            </a>
            <p>Free and open source (MIT)</p>
            <Diagnostics />
            <SavedRecordings recording={recordings} canRecord={canRecord} />
          </section>
        )}
      </main>
      <PairingDialog key={link.pairing?.sessionId} pairing={link.pairing} onError={setError} />
      <RecordingDialogs recording={recordings} />
      <footer>
        Touch the air. Your screen follows.<span>Sound stays home.</span>
      </footer>
    </div>
  );
}
