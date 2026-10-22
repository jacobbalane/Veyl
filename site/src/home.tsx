import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ArrowUpRight, CameraOff, LockKeyhole, Radio } from 'lucide-react';
import { DownloadMenu, Footer, Header } from './chrome';
import '@fontsource-variable/manrope';
import './site.css';

function DeskIllustration() {
  return (
    <div
      className="desk-illustration"
      role="img"
      aria-label="A phone in front of a monitor. A hand pokes toward the phone, and echoes become a click."
    >
      <div className="screen">
        <div className="screen-label">
          <i />
          VEYL · POKE TO CLICK
        </div>
        <div className="screen-grid" />
        <div className="pointer">↖</div>
        <span className="click-ring" />
      </div>
      <div className="monitor-neck" />
      <div className="monitor-foot" />
      <div className="phone">
        <div className="phone-speaker" />
        <Radio size={25} />
        <div className="phone-dot" />
      </div>
      <div className="hand">
        <div />
        <span />
      </div>
      <div className="echo-arc arc-one" />
      <div className="echo-arc arc-two" />
      <div className="echo-arc arc-three" />
      <span className="illustration-caption">A peek at what’s coming in v0.1</span>
    </div>
  );
}
function Home() {
  const [mode, setMode] = useState<'pc' | 'phone'>('phone');
  const steps =
    mode === 'pc'
      ? ['Turn on your PC’s speakers.', 'Play anything on your PC.', 'Poke toward its microphone.']
      : ['Lay your phone by your monitor.', 'Play something on it.', 'Poke toward the phone.'];
  return (
    <>
      <Header />
      <main>
        <section className="hero wrap">
          <div className="hero-copy">
            <span className="release-note">
              <i />
              First release coming soon
            </span>
            <h1>
              Touch the air.
              <br />
              <em>
                Your screen
                <br className="wide-only" /> follows.
              </em>
            </h1>
            <p>
              Veyl turns your PC — or your Android phone — into a sonar. It listens to the echo of
              the music or video you’re playing and turns a poke in the air into a click.
            </p>
            <DownloadMenu />
            <small>Free and open source · Windows 10 or 11 · Android 8 or later (optional)</small>
            <p className="preview-note">
              Preview builds check installation. Listening and gestures are coming in v0.1.
            </p>
          </div>
          <DeskIllustration />
        </section>
        <div className="principles wrap">
          <span>
            <CameraOff size={18} />
            No camera
          </span>
          <span>
            <Radio size={18} />
            Your own sound
          </span>
          <span>
            <LockKeyhole size={18} />
            Sound stays home
          </span>
        </div>
        <section className="how wrap section" id="how">
          <div className="section-top">
            <div>
              <span className="eyebrow">01 / A small new habit</span>
              <h2>Play. Poke. Click.</h2>
            </div>
            <div className="mode-switch" role="group" aria-label="How to use Veyl">
              <button aria-pressed={mode === 'pc'} onClick={() => setMode('pc')}>
                With your PC
              </button>
              <button aria-pressed={mode === 'phone'} onClick={() => setMode('phone')}>
                With your phone
              </button>
            </div>
          </div>
          <div className="steps">
            {steps.map((step, index) => (
              <div key={step}>
                <span>0{index + 1}</span>
                <h3>{step}</h3>
              </div>
            ))}
          </div>
          <p className="section-note">
            Veyl sees your hand in the echo and clicks for you. Actions start off, so you can
            practise first.
          </p>
        </section>
        <section className="meter-section wrap section">
          <div className="meter-copy">
            <span className="eyebrow">02 / Know when it works</span>
            <h2>
              Your music is
              <br />
              the signal.
            </h2>
            <p>
              A bright, steady song gives Veyl more to hear. The live meter will tell you when the
              sound is good enough, and what to change when it isn’t.
            </p>
            <a className="text-link" href="docs/#meter">
              Meet the meter <ArrowUpRight size={17} />
            </a>
          </div>
          <div className="meter-example">
            <span className="sample-label">Illustration · sample data</span>
            <div className="meter-head">
              <strong>78</strong>
              <span>
                Great<small>Detail ≈ 1.8 cm</small>
              </span>
              <div className="meter-orbit" />
            </div>
            {[
              ['High sounds', 82],
              ['Volume', 74],
              ['Steady', 94],
              ['Clear echo', 68],
            ].map(([name, score]) => (
              <div className="sample-factor" key={name}>
                <span>{name}</span>
                <div>
                  <i style={{ width: `${score}%` }} />
                </div>
              </div>
            ))}
            <p>Ready. Poke toward your phone.</p>
          </div>
        </section>
        <section className="privacy-section section">
          <div className="wrap privacy-layout">
            <div>
              <span className="eyebrow">03 / Private by design</span>
              <h2>
                The echo travels.
                <br />
                <em>Your sound stays home.</em>
              </h2>
            </div>
            <div>
              <p>
                Phone audio is filtered before it leaves for your paired PC over an encrypted link.
                In PC mode, audio stays on the PC.
              </p>
              <p>
                There are no accounts, analytics or cloud services. Nothing saves sound unless you
                start a test recording on your PC.
              </p>
              <a className="text-link" href="docs/#privacy">
                Read the privacy details <ArrowUpRight size={17} />
              </a>
            </div>
          </div>
        </section>
        <section className="needs wrap section">
          <span className="eyebrow">04 / Use what you have</span>
          <h2>A Windows PC. Two ways in.</h2>
          <div className="needs-grid">
            <article>
              <span className="path-label">WITH THIS PC</span>
              <h3>Speakers + a microphone.</h3>
              <p>
                Most laptops have both. Play a song or video on the PC, then poke toward its
                microphone. No phone needed.
              </p>
              <small>Windows 10 version 1809+ or Windows 11 · x64 or ARM64</small>
            </article>
            <article>
              <span className="path-label">WITH MY PHONE</span>
              <h3>A phone by your monitor.</h3>
              <p>
                An Android phone plays and listens. Pair it with your PC on the same home network.
                Your PC can use wired Ethernet.
              </p>
              <small>Android 8+ · Copying other apps’ sound needs Android 10+</small>
            </article>
          </div>
        </section>
        <section className="roadmap wrap section">
          <div>
            <span className="eyebrow">05 / Starting with a poke</span>
            <h2>
              A small first step.
              <br />
              Room to grow.
            </h2>
          </div>
          <ol>
            <li>
              <b>v0.1 · Poke to click</b>
              <p>Phone and PC modes, the meter, the Veyl player and a test sound.</p>
              <span>IN DEVELOPMENT</span>
            </li>
            <li>
              <b>Next · More ways to move</b>
              <p>Finer distance, scrolling, mixed modes, an air pointer and the virtual screen.</p>
            </li>
          </ol>
        </section>
        <section className="faq wrap section">
          <span className="eyebrow">A few good questions</span>
          <h2>Before you try it.</h2>
          {[
            [
              'Will I hear anything extra?',
              'Veyl uses the sound you already play. Its optional test sound is there when you want a predictable signal.',
            ],
            [
              'Do I need a phone?',
              'PC mode uses your PC’s speakers and microphone. Phone mode is useful for desktops without speakers. Both are coming in v0.1.',
            ],
            [
              'Which apps work?',
              'On the PC, loopback can copy the sound of any app, including Spotify; protected video may be silent. On Android, YouTube and YouTube Music allow playback capture. Spotify blocks it, so use the Veyl player or PC mode.',
            ],
            [
              'Does it work with headphones?',
              'The sound must come from the phone’s speaker or your PC’s speakers so the microphone can hear echoes.',
            ],
            [
              'Do I need a camera?',
              'Veyl senses with sound. No camera or extra sensing hardware is needed.',
            ],
            [
              'Why does Android ask to record or cast?',
              'That is how Android shares app sound. Veyl uses the audio, never your screen. The Veyl player and test sound do not need that prompt.',
            ],
            [
              'Why do Windows and Android warn when I install?',
              'Veyl is distributed outside app stores, and the Windows preview is unsigned. Release files will include checksums and build attestations. Read the installation notes before opening a download.',
            ],
            ['Is it free?', 'Yes. Veyl is open source under the MIT license.'],
          ].map(([question, answer]) => (
            <details key={question}>
              <summary>
                {question}
                <span>+</span>
              </summary>
              <p>{answer}</p>
            </details>
          ))}
        </section>
        <section className="closing wrap section">
          <h2>
            A little less reaching.
            <br />
            <em>A little more air.</em>
          </h2>
          <DownloadMenu />
          <p>First release coming soon.</p>
          <p className="unsupported">Veyl runs on Windows PCs, with or without an Android phone.</p>
        </section>
      </main>
      <Footer />
    </>
  );
}
createRoot(document.getElementById('root')!).render(<Home />);
