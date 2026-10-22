import { useEffect, useState, type AnchorHTMLAttributes, type ReactNode } from 'react';
import { ArrowDown, ChevronDown } from 'lucide-react';

export const REPO = 'https://github.com/jacobbalane/Veyl';
export const DOWNLOAD = `${REPO}/releases/latest`;
export type System = 'Windows' | 'WindowsArm' | 'Android';
export const FIRST_GUESS: System = /Android/i.test(navigator.userAgent) ? 'Android' : 'Windows';
type Hints = { getHighEntropyValues(keys: string[]): Promise<{ architecture?: string }> };
export async function detectSystem(): Promise<System> {
  if (FIRST_GUESS === 'Android') return 'Android';
  const hints = (navigator as Navigator & { userAgentData?: Hints }).userAgentData;
  const architecture = await hints?.getHighEntropyValues(['architecture']).then(
    (value) => value.architecture,
    () => undefined,
  );
  return architecture === 'arm' ? 'WindowsArm' : 'Windows';
}
export const LABELS: Record<System, string> = {
  Windows: 'Windows',
  WindowsArm: 'Windows (ARM)',
  Android: 'Android',
};
const DETAILS: Record<System, string> = {
  Windows: 'Windows 10 or 11 · x64',
  WindowsArm: 'Windows 10 or 11 · ARM64',
  Android: 'Android 8 or later',
};
const FILES: Record<System, RegExp> = {
  Windows: /_x64-setup\.exe$/,
  WindowsArm: /_arm64-setup\.exe$/,
  Android: /^Veyl_.*\.apk$/,
};
interface Asset {
  name: string;
  browser_download_url: string;
}
let latest: Promise<Asset[]> | undefined;

export function assetFor(system: System, assets: Asset[]): string | undefined {
  return assets.find(
    (asset) =>
      FILES[system].test(asset.name) &&
      asset.browser_download_url.startsWith(`${REPO}/releases/download/`),
  )?.browser_download_url;
}

export function useDownload(system: System) {
  const [href, setHref] = useState(DOWNLOAD);
  useEffect(() => {
    let live = true;
    setHref(DOWNLOAD);
    latest ??= fetch('https://api.github.com/repos/jacobbalane/Veyl/releases/latest')
      .then((response) =>
        response.ok ? response.json() : Promise.reject(new Error('No published release.')),
      )
      .then((release: { assets?: Asset[] }) =>
        Array.isArray(release.assets) ? release.assets : [],
      );
    latest.then(
      (assets) => {
        const url = assetFor(system, assets);
        if (live && url) setHref(url);
      },
      () => {},
    );
    return () => {
      live = false;
    };
  }, [system]);
  return href;
}

export function DownloadLink({
  system,
  children,
  ...rest
}: { system: System; children: ReactNode } & AnchorHTMLAttributes<HTMLAnchorElement>) {
  const href = useDownload(system);
  return (
    <a
      {...rest}
      href={href}
      {...(href === DOWNLOAD ? { target: '_blank', rel: 'noreferrer' } : {})}
    >
      {children}
    </a>
  );
}

export function DownloadMenu() {
  const [system, setSystem] = useState(FIRST_GUESS);
  useEffect(() => {
    let live = true;
    void detectSystem().then((value) => {
      if (live) setSystem(value);
    });
    return () => {
      live = false;
    };
  }, []);
  const systems: System[] = [
    system,
    ...(['Windows', 'WindowsArm', 'Android'] as const).filter((item) => item !== system),
  ];
  return (
    <div className="download-row">
      <DownloadLink system={system} className="primary-button">
        <ArrowDown size={18} />
        Download for {LABELS[system]}
      </DownloadLink>
      <details className="downloads">
        <summary aria-label="All downloads">
          <ChevronDown size={19} />
        </summary>
        <nav aria-label="All downloads">
          {systems.map((item) => (
            <DownloadLink key={item} system={item}>
              <b>{LABELS[item]}</b>
              <small>{DETAILS[item]}</small>
            </DownloadLink>
          ))}
        </nav>
      </details>
    </div>
  );
}

export function Header({ root = '', current }: { root?: string; current?: 'docs' | 'changelog' }) {
  return (
    <header className="site-header wrap">
      <a className="brand" href={root || './'}>
        <img src={`${root}favicon.svg`} width="38" height="38" alt="" />
        <span>veyl</span>
      </a>
      <nav aria-label="Main">
        <a href={`${root}docs/`} aria-current={current === 'docs' ? 'page' : undefined}>
          Docs
        </a>
        <a href={`${root}changelog/`} aria-current={current === 'changelog' ? 'page' : undefined}>
          Changelog
        </a>
        <a href={REPO}>GitHub ↗</a>
      </nav>
    </header>
  );
}
export function Footer({ root = '' }: { root?: string }) {
  return (
    <footer className="site-footer wrap">
      <div>
        <a className="brand" href={root || './'}>
          veyl
        </a>
        <p>Touch the air. Your screen follows.</p>
      </div>
      <nav aria-label="Project">
        <a href={REPO}>GitHub</a>
        <a href={`${root}docs/`}>Docs</a>
        <a href={`${root}changelog/`}>Changelog</a>
        <a href={`${REPO}/blob/main/LICENSE`}>License (MIT)</a>
        <a href={`${root}licenses/manrope.txt`}>Font license</a>
      </nav>
      <small>Not affiliated with Google, YouTube or Spotify.</small>
    </footer>
  );
}
