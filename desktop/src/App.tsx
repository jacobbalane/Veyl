import { useEffect, useState } from 'react';
import { AudioLines, CircleHelp, Radio, Settings, Smartphone } from 'lucide-react';
import mark from '../../assets/brand/veyl-mark.svg';
import { browserPreview, getAppState, uiReady, type AppState } from './api/native';
import { copy } from './copy';

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
  useEffect(() => {
    let active = true;
    getAppState()
      .then((state) => {
        if (active) setApp(state);
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
        <span className="connection" aria-live="polite">
          <i />
          {copy.status}
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
                <button disabled>Record a test</button>
              </div>
            </section>
          </div>
        )}
        {tab === 'Phones' && (
          <section className="panel placeholder">
            <Smartphone size={36} />
            <h1>Your phones</h1>
            <p>{copy.phonesEmpty}</p>
            <h2>Nearby</h2>
            <p>Pairing is coming in v0.1.</p>
          </section>
        )}
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
            <p>Listening, pairing and actions are coming in v0.1.</p>
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
          </section>
        )}
      </main>
      <footer>
        Touch the air. Your screen follows.<span>Sound stays home.</span>
      </footer>
    </div>
  );
}
