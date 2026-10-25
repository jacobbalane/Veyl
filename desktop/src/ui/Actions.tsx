import { useEffect, useReducer, useState } from 'react';
import { X } from 'lucide-react';
import { setActionsEnabled, watchActions } from '../api/native';
import type { ActionsEvent } from '../api/types';
import { actionsCopy, copy, hotkeyLabel, linkCopy } from '../copy';

export interface ActionsView {
  /** Only Rust turns actions on, so this follows its events and never a click alone. */
  enabled: boolean;
  /** Windows blocked an action in an app running as administrator. */
  blocked: boolean;
  /** "Actions on" or "Actions off" after a change, from any source. */
  toast: { text: string; id: number } | null;
  /** True once an event arrived, so the app state can't replace it. */
  received: boolean;
  /** True once the app state or an event said whether actions are on; only later changes toast. */
  known: boolean;
  /** False when Veyl can't follow the switch, which then stays as it is. */
  available: boolean;
  /** How many toasts were shown, so each new one gets its own id. */
  toasts: number;
}
export const emptyActions: ActionsView = {
  enabled: false,
  blocked: false,
  toast: null,
  received: false,
  known: false,
  available: true,
  toasts: 0,
};
export type ActionsAction =
  | { type: 'initial'; enabled: boolean }
  | { type: 'event'; event: ActionsEvent }
  | { type: 'hide-toast'; id: number }
  | { type: 'dismiss' }
  | { type: 'unavailable' };

export function actionsReducer(view: ActionsView, action: ActionsAction): ActionsView {
  switch (action.type) {
    case 'initial':
      return view.received ? view : { ...view, known: true, enabled: action.enabled };
    case 'event': {
      const { enabled, error } = action.event;
      const changed = enabled !== view.enabled;
      const toast = view.known && changed;
      return {
        ...view,
        received: true,
        known: true,
        enabled,
        // A new block shows the message; switching actions clears an old one.
        blocked: error === 'blocked_elevated' || (!changed && view.blocked),
        toast: toast
          ? { text: enabled ? actionsCopy.toastOn : actionsCopy.toastOff, id: view.toasts + 1 }
          : view.toast,
        toasts: toast ? view.toasts + 1 : view.toasts,
      };
    }
    case 'hide-toast':
      return view.toast?.id === action.id ? { ...view, toast: null } : view;
    case 'dismiss':
      return { ...view, blocked: false };
    case 'unavailable':
      return { ...view, available: false };
  }
}

/** How long a toast stays, in milliseconds. */
export const toastTime = 2500;

/** The actions switch: follows 'veyl://actions' from the header, the tray and the hotkey. */
export function useActions(initial?: boolean) {
  const [view, dispatch] = useReducer(actionsReducer, emptyActions);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => {
    if (initial != null) dispatch({ type: 'initial', enabled: initial });
  }, [initial]);
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    void watchActions((event) => {
      if (active) dispatch({ type: 'event', event });
    })
      .then((unlisten) => {
        if (active) stop = unlisten;
        else unlisten();
      })
      .catch(() => {
        if (active) dispatch({ type: 'unavailable' });
      });
    return () => {
      active = false;
      stop?.();
    };
  }, []);
  useEffect(() => {
    if (!view.toast) return;
    const id = view.toast.id;
    const timer = setTimeout(() => dispatch({ type: 'hide-toast', id }), toastTime);
    return () => clearTimeout(timer);
  }, [view.toast]);
  const toggle = async () => {
    setPending(true);
    setError('');
    try {
      await setActionsEnabled(!view.enabled);
    } catch {
      setError(linkCopy.failedRequest);
    } finally {
      setPending(false);
    }
  };
  return { ...view, pending, error, toggle, dismiss: () => dispatch({ type: 'dismiss' }) };
}
export type ActionsState = ReturnType<typeof useActions>;

export function ActionsSwitch({
  actions,
  hotkey,
  ready,
}: {
  actions: ActionsState;
  hotkey: string;
  /** False until the app state has loaded. */
  ready: boolean;
}) {
  const { enabled } = actions;
  return (
    <div className="actions">
      <span aria-hidden="true">
        {actionsCopy.name} <b data-on={enabled}>{enabled ? actionsCopy.on : actionsCopy.off}</b>
      </span>
      <button
        role="switch"
        aria-label={actionsCopy.name}
        aria-checked={enabled}
        aria-describedby="actions-tooltip"
        disabled={!ready || !actions.available || actions.pending}
        onClick={() => void actions.toggle()}
      >
        <i />
      </button>
      <kbd>{hotkeyLabel(hotkey)}</kbd>
      <span id="actions-tooltip" role="tooltip" className="actions-tooltip">
        {actionsCopy.tooltip(hotkey)}
      </span>
    </div>
  );
}

/** The toast after a change and Windows' block, below the header. */
export function ActionsNotices({ actions }: { actions: ActionsState }) {
  return (
    <>
      <div className="toast-region" aria-live="polite" aria-atomic="true">
        {actions.toast && (
          <span className="toast" key={actions.toast.id}>
            {actions.toast.text}
          </span>
        )}
      </div>
      {actions.blocked && (
        <div className="notice" role="alert">
          <p>{actionsCopy.blocked}</p>
          <button className="icon-button" aria-label={copy.close} onClick={actions.dismiss}>
            <X size={16} aria-hidden="true" />
          </button>
        </div>
      )}
      {actions.error && <p role="alert">{actions.error}</p>}
    </>
  );
}
