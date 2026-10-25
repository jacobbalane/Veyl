import { useEffect, useRef, useState, type InputHTMLAttributes, type ReactNode } from 'react';
import { setSettings } from '../api/native';
import type { SettingsPatch, SettingsView } from '../api/types';
import { hotkeyLabel, settingsCopy as text, settingsError } from '../copy';
import { readHotkey } from './hotkey';
import { SourceSettings, type SourceControls } from './Sources';

type Field = keyof SettingsView;

/** The saved settings, with a pending flag and an error per field while a change is saved. */
export function useSettings(initial?: SettingsView) {
  const [value, setValue] = useState<SettingsView | undefined>(initial);
  const [errors, setErrors] = useState<Partial<Record<Field, string>>>({});
  const [pending, setPending] = useState<Partial<Record<Field, boolean>>>({});
  useEffect(() => {
    if (initial) setValue(initial);
  }, [initial]);
  const mark = (fields: Field[], flag: boolean) =>
    setPending((old) => ({ ...old, ...Object.fromEntries(fields.map((field) => [field, flag])) }));
  const fail = (field: Field, message: string) =>
    setErrors((old) => ({ ...old, [field]: message }));
  const clear = (fields: Field[]) =>
    setErrors((old) => {
      const next = { ...old };
      for (const field of fields) delete next[field];
      return next;
    });
  /** Saves a change; on an error the control shows the saved value again and says why. */
  const save = async (patch: SettingsPatch): Promise<boolean> => {
    const fields = Object.keys(patch) as Field[];
    mark(fields, true);
    clear(fields);
    try {
      setValue(await setSettings(patch));
      return true;
    } catch (error) {
      // Rust saved the change before the engine refused it, so show what was saved.
      if (error === 'engine_busy' || error === 'engine_stopped')
        setValue((old) => (old ? { ...old, ...patch } : old));
      fail(fields[0], settingsError(error));
      return false;
    } finally {
      mark(fields, false);
    }
  };
  return { value, errors, pending, save, fail, clear };
}
export type SettingsState = ReturnType<typeof useSettings>;

function FieldError({ settings, field }: { settings: SettingsState; field: Field }) {
  const error = settings.errors[field];
  return error ? (
    <p role="alert" className="setting-error">
      {error}
    </p>
  ) : null;
}

function Row({
  settings,
  field,
  note,
  children,
}: {
  settings: SettingsState;
  field: Field;
  note?: string;
  children: ReactNode;
}) {
  return (
    <div className="setting">
      {children}
      {note && (
        <p id={`setting-${field}-note`} className="setting-note">
          {note}
        </p>
      )}
      <FieldError settings={settings} field={field} />
    </div>
  );
}

type ChoiceField = 'pokeAction' | 'sensitivity' | 'range';
function Choice<K extends ChoiceField>({
  settings,
  value,
  field,
  label,
  options,
}: {
  settings: SettingsState;
  value: SettingsView;
  field: K;
  label: string;
  options: Record<SettingsView[K], string>;
}) {
  return (
    <Row settings={settings} field={field}>
      <label>
        {label}
        <select
          value={value[field]}
          disabled={settings.pending[field]}
          onChange={(event) => void settings.save({ [field]: event.target.value } as SettingsPatch)}
        >
          {(Object.entries(options) as [string, string][]).map(([option, name]) => (
            <option key={option} value={option}>
              {name}
            </option>
          ))}
        </select>
      </label>
    </Row>
  );
}

function Toggle({
  settings,
  field,
  label,
  note,
  checked,
  patch,
}: {
  settings: SettingsState;
  field: Field;
  label: string;
  note?: string;
  checked: boolean;
  patch: (on: boolean) => SettingsPatch;
}) {
  return (
    <Row settings={settings} field={field} note={note}>
      <label>
        {label}
        <input
          type="checkbox"
          role="switch"
          className="switch"
          checked={checked}
          disabled={settings.pending[field]}
          aria-describedby={note ? `setting-${field}-note` : undefined}
          onChange={(event) => void settings.save(patch(event.target.checked))}
        />
      </label>
    </Row>
  );
}

/**
 * A text or number setting that saves when the field loses focus or on Enter, so a half-typed
 * name is never saved. Escape puts the saved value back.
 */
function Entry({
  settings,
  field,
  label,
  saved,
  note,
  suffix,
  input,
  commit,
}: {
  settings: SettingsState;
  field: Field;
  label: string;
  saved: string;
  note?: string;
  suffix?: string;
  input: InputHTMLAttributes<HTMLInputElement>;
  /** Saves the typed text, or returns false when it is not worth sending. */
  commit: (draft: string) => Promise<boolean> | boolean;
}) {
  const [draft, setDraft] = useState(saved);
  // The text last sent, so leaving the field after Enter doesn't send it a second time.
  const sent = useRef<string | null>(null);
  useEffect(() => setDraft(saved), [saved]);
  const submit = async () => {
    if (draft === saved || draft === sent.current) return;
    sent.current = draft;
    await commit(draft);
  };
  const id = `setting-${field}`;
  return (
    <Row settings={settings} field={field} note={note}>
      <div className="setting-line">
        <label htmlFor={id}>{label}</label>
        <span className="entry">
          <input
            {...input}
            id={id}
            value={draft}
            disabled={settings.pending[field]}
            aria-describedby={note ? `setting-${field}-note` : undefined}
            onChange={(event) => {
              sent.current = null;
              setDraft(event.target.value);
            }}
            onBlur={() => void submit()}
            onKeyDown={(event) => {
              if (event.key === 'Enter') void submit();
              if (event.key === 'Escape') {
                sent.current = null;
                setDraft(saved);
                settings.clear([field]);
              }
            }}
          />
          {suffix && <span aria-hidden="true">{suffix}</span>}
        </span>
      </div>
    </Row>
  );
}

/** "Change" listens for the next key combination; Escape or leaving the button cancels. */
export function HotkeyField({ settings, hotkey }: { settings: SettingsState; hotkey: string }) {
  const [recording, setRecording] = useState(false);
  const [message, setMessage] = useState('');
  const stop = () => {
    setRecording(false);
    setMessage('');
  };
  return (
    <Row settings={settings} field="hotkey">
      <div className="setting-line">
        <span id="setting-hotkey-label">{text.hotkey}</span>
        <span className="hotkey">
          <kbd>{hotkeyLabel(hotkey)}</kbd>
          <button
            className="hotkey-button"
            aria-describedby="setting-hotkey-label"
            aria-pressed={recording}
            disabled={settings.pending.hotkey}
            onClick={() => {
              if (recording) return;
              settings.clear(['hotkey']);
              setRecording(true);
            }}
            onBlur={stop}
            onKeyDown={(event) => {
              if (!recording) return;
              // While recording, every key belongs to the new combination, Tab and Enter too.
              event.preventDefault();
              event.stopPropagation();
              const press = readHotkey(event);
              switch (press.kind) {
                case 'cancel':
                  stop();
                  break;
                case 'bare':
                  setMessage(text.needsModifier);
                  break;
                case 'unsupported':
                  setMessage(text.unsupportedKey);
                  break;
                case 'hotkey':
                  stop();
                  if (press.hotkey !== hotkey) void settings.save({ hotkey: press.hotkey });
                  break;
              }
            }}
          >
            {recording ? text.recordHotkey : text.change}
          </button>
        </span>
      </div>
      {message && (
        <p role="alert" className="setting-error">
          {message}
        </p>
      )}
    </Row>
  );
}

export function SettingsPanel({
  settings,
  sources,
}: {
  settings: SettingsState;
  sources: SourceControls;
}) {
  const value = settings.value;
  return (
    <section className="panel settings">
      <h1>{text.title}</h1>
      <SourceSettings controls={sources} />
      {value && (
        <>
          <Choice
            settings={settings}
            value={value}
            field="pokeAction"
            label={text.pokeAction}
            options={text.pokeActions}
          />
          <Choice
            settings={settings}
            value={value}
            field="sensitivity"
            label={text.sensitivity}
            options={text.sensitivities}
          />
          <Choice
            settings={settings}
            value={value}
            field="range"
            label={text.range}
            options={text.ranges}
          />
          <HotkeyField settings={settings} hotkey={value.hotkey} />
          <Toggle
            settings={settings}
            field="actionsOnAtStart"
            label={text.actionsOnAtStart}
            checked={value.actionsOnAtStart}
            patch={(on) => ({ actionsOnAtStart: on })}
          />
          <Toggle
            settings={settings}
            field="startWithWindows"
            label={text.startWithWindows}
            checked={value.startWithWindows}
            patch={(on) => ({ startWithWindows: on })}
          />
          <Toggle
            settings={settings}
            field="closeToTray"
            label={text.closeToTray}
            checked={value.closeToTray}
            patch={(on) => ({ closeToTray: on })}
          />
          <Entry
            settings={settings}
            field="pcName"
            label={text.pcName}
            saved={value.pcName}
            input={{ maxLength: 40, autoComplete: 'off', spellCheck: false }}
            commit={(draft) => settings.save({ pcName: draft })}
          />
          <h2>{text.advanced}</h2>
          <Entry
            settings={settings}
            field="temperatureC"
            label={text.temperature}
            note={text.temperatureNote}
            suffix={text.celsius}
            saved={String(value.temperatureC)}
            input={{ type: 'number', min: -10, max: 45, step: 0.5, inputMode: 'decimal' }}
            commit={(draft) => {
              const temperature = Number(draft);
              // An empty or unreadable field would reach Rust as "no change".
              if (draft.trim() === '' || !Number.isFinite(temperature)) {
                settings.fail('temperatureC', settingsError('invalid_temperature'));
                return false;
              }
              return settings.save({ temperatureC: temperature });
            }}
          />
          <Toggle
            settings={settings}
            field="detailedLogs"
            label={text.detailedLogs}
            note={text.detailedLogsNote}
            checked={value.detailedLogs}
            patch={(on) => ({ detailedLogs: on })}
          />
          <Toggle
            settings={settings}
            field="keepRecordingsDays"
            label={text.keepRecordings}
            note={text.keepRecordingsNote}
            checked={value.keepRecordingsDays > 0}
            patch={(on) => ({ keepRecordingsDays: on ? 7 : 0 })}
          />
        </>
      )}
    </section>
  );
}
