import { ExternalLink } from 'lucide-react';
import {
  copy,
  helpCopy as text,
  helpErrors,
  helpTips,
  links,
  pcCopy,
  tipFix,
  tipText,
} from '../copy';
import { Diagnostics } from './Diagnostics';
import { SavedRecordings, type RecordingsState } from './Recordings';

function Link({ href, children }: { href: string; children: string }) {
  return (
    <a href={href} target="_blank" rel="noreferrer">
      {children}
      <ExternalLink size={12} aria-hidden="true" />
    </a>
  );
}

function Placement({ pc }: { pc: boolean }) {
  const phone = (
    <section key="phone" aria-labelledby="help-placement-phone">
      <h2 id="help-placement-phone">{text.placementPhone}</h2>
      <ul>
        {text.placementPhoneSteps.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ul>
    </section>
  );
  const computer = (
    <section key="pc" aria-labelledby="help-placement-pc">
      <h2 id="help-placement-pc">{pcCopy.placementTitle}</h2>
      <p>{pcCopy.placement}</p>
    </section>
  );
  // The way Veyl is used now comes first.
  return <>{pc ? [computer, phone] : [phone, computer]}</>;
}

/** One entry per tip and error for the current mode, each with a fix. */
export function Troubleshooting({ pc }: { pc: boolean }) {
  const entries = [
    ...helpTips(pc).map((tip) => ({ problem: tipText(tip, pc), fix: tipFix(tip, pc) })),
    ...helpErrors.filter((entry) => !entry.mode || entry.mode === (pc ? 'pc' : 'phone')),
  ];
  return (
    <section aria-labelledby="help-troubleshooting">
      <h2 id="help-troubleshooting">{text.troubleshooting}</h2>
      <div className="help-entries">
        {entries.map(({ problem, fix }) => (
          <details key={problem}>
            <summary>{problem}</summary>
            <p>{fix}</p>
          </details>
        ))}
      </div>
      <p>
        <Link href={links.troubleshooting}>{text.more}</Link>
      </p>
    </section>
  );
}

export function Help({
  pc,
  version,
  recordings,
  canRecord,
}: {
  pc: boolean;
  version: string;
  recordings: RecordingsState;
  canRecord: boolean;
}) {
  return (
    <section className="panel help">
      <h1 className="visually-hidden">Help</h1>
      <Placement pc={pc} />
      <Troubleshooting pc={pc} />
      <section aria-labelledby="help-privacy">
        <h2 id="help-privacy">{text.privacy}</h2>
        <p>{text.privacySummary}</p>
        <ul>
          {text.privacyDetails.map((detail) => (
            <li key={detail}>{detail}</li>
          ))}
        </ul>
      </section>
      <Diagnostics />
      <SavedRecordings recording={recordings} canRecord={canRecord} />
      <section aria-labelledby="help-about" className="about">
        <h2 id="help-about">{text.about}</h2>
        <p>
          <strong>Veyl</strong> · {text.version(version)}
        </p>
        <p>{copy.tagline}</p>
        <ul className="link-list">
          <li>
            <Link href={links.website}>{text.website}</Link>
          </li>
          <li>
            <Link href={links.source}>{text.source}</Link>
          </li>
          <li>
            <Link href={links.licenses}>{text.licenses}</Link>
          </li>
        </ul>
        <p>{text.license}</p>
      </section>
    </section>
  );
}
