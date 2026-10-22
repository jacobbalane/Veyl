import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { App } from './App';
import { emptyLink, emptyLevels } from './api/types';

vi.mock('./api/native', () => ({
  browserPreview: true,
  getAppState: async () => ({
    version: '0.0.1',
    pcName: 'Test PC',
    actionsEnabled: false,
    link: structuredClone(emptyLink),
  }),
  uiReady: async () => {},
  watchLink: async () => () => {},
  getDiagnostics: async () => ({
    link: structuredClone(emptyLink),
    levels: structuredClone(emptyLevels),
  }),
}));
afterEach(cleanup);

test('preview keeps actions unavailable and allows navigating every foundation view', async () => {
  render(<App />);
  expect(await screen.findByText('v0.0.1')).toBeDefined();
  const actions = screen.getByRole('switch', { name: 'Actions' });
  expect(actions.getAttribute('aria-checked')).toBe('false');
  expect(actions.hasAttribute('disabled')).toBe(true);
  expect(screen.getByText('Preview — sample data')).toBeDefined();
  fireEvent.click(screen.getByRole('button', { name: 'Phones' }));
  expect(screen.getByText('Your phones')).toBeDefined();
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  expect(screen.getByDisplayValue('Test PC')).toBeDefined();
  fireEvent.click(screen.getByRole('button', { name: 'Help' }));
  expect(screen.getByText('Privacy')).toBeDefined();
});
