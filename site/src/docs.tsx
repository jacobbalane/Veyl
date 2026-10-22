import { createRoot } from 'react-dom/client';
import { Footer, Header } from './chrome';
import { Markdown } from './markdown';
import gettingStarted from '../../docs/getting-started.md?raw';
import placement from '../../docs/placement.md?raw';
import meter from '../../docs/meter.md?raw';
import troubleshooting from '../../docs/troubleshooting.md?raw';
import privacy from '../../docs/privacy.md?raw';
import howItWorks from '../../docs/how-it-works.md?raw';
import development from '../../docs/development.md?raw';
import '@fontsource-variable/manrope';
import './site.css';

const documents = [
  ['getting-started', 'Getting started', gettingStarted],
  ['placement', 'Placement', placement],
  ['meter', 'The meter', meter],
  ['troubleshooting', 'Troubleshooting', troubleshooting],
  ['privacy', 'Privacy', privacy],
  ['how-it-works', 'How it works', howItWorks],
  ['development', 'Development', development],
];
createRoot(document.getElementById('root')!).render(
  <>
    <Header root="../" current="docs" />
    <main className="wrap docs-layout">
      <aside>
        <span className="eyebrow">Veyl docs</span>
        <nav aria-label="Documentation">
          {documents.map(([id, title]) => (
            <a key={id} href={`#${id}`}>
              {title}
            </a>
          ))}
        </nav>
      </aside>
      <div className="prose">
        <h1>Make yourself at home.</h1>
        <p className="doc-status">
          First release coming soon. These docs follow the implementation.
        </p>
        {documents.map(([id, , text]) => (
          <section key={id} id={id}>
            <Markdown text={text} />
          </section>
        ))}
      </div>
    </main>
    <Footer root="../" />
  </>,
);
