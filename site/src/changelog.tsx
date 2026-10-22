import { createRoot } from 'react-dom/client';
import { Footer, Header } from './chrome';
import { Markdown } from './markdown';
import changelog from '../../CHANGELOG.md?raw';
import '@fontsource-variable/manrope';
import './site.css';

createRoot(document.getElementById('root')!).render(
  <>
    <Header root="../" current="changelog" />
    <main className="wrap changelog prose">
      <span className="eyebrow">The work, as it happens</span>
      <Markdown text={changelog} />
    </main>
    <Footer root="../" />
  </>,
);
