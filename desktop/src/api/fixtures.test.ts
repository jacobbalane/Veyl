import { expect, test } from 'vitest';
import appJson from './fixtures/app-state.json';
import diagnosticsJson from './fixtures/diagnostics.json';
import pairingJson from './fixtures/pairing.json';
import type { AppState, Diagnostics, PairingEvent } from './types';

test('Rust fixtures fit the typed desktop API and contain metadata only', () => {
  const app: AppState = appJson;
  const diagnostics: Diagnostics = diagnosticsJson;
  const pairing: PairingEvent = pairingJson;
  expect(app.actionsEnabled).toBe(false);
  expect(app.link.connection.state).toBe('browsing');
  expect(diagnostics.levels.reference.preDbfs).toBe(-120);
  expect(diagnostics.link.pairing).toBeNull();
  expect(pairing.sessionId).toBe('test-comparison');
  expect(pairing.code).toMatch(/^\d{6}$/);
  expect(Object.keys(diagnostics.levels)).not.toContain('samples');
  expect(Object.keys(app.link)).not.toContain('keys');
});
