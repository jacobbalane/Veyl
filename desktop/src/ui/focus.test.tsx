import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { keepFocusInside } from './focus';

afterEach(cleanup);

test('Tab wraps around inside a dialog in both directions', () => {
  render(
    <>
      <button>Outside</button>
      <div role="dialog" onKeyDown={keepFocusInside}>
        <button>First</button>
        <button disabled>Off</button>
        <button>Last</button>
      </div>
    </>,
  );
  const first = screen.getByRole('button', { name: 'First' });
  const last = screen.getByRole('button', { name: 'Last' });
  last.focus();
  fireEvent.keyDown(last, { key: 'Tab' });
  expect(document.activeElement).toBe(first);
  fireEvent.keyDown(first, { key: 'Tab', shiftKey: true });
  expect(document.activeElement).toBe(last);
  // Between the ends, the browser moves focus as usual.
  first.focus();
  fireEvent.keyDown(first, { key: 'Tab' });
  expect(document.activeElement).toBe(first);
});
