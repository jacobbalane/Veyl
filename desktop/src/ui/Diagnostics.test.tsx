import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { emptyLevels, emptyLink, type Diagnostics as DiagnosticsValue } from '../api/types';
import { Diagnostics } from './Diagnostics';

const diagnostics = vi.hoisted(() => ({ value: undefined as DiagnosticsValue | undefined }));
vi.mock('../api/native', () => ({
  browserPreview: false,
  getDiagnostics: async () => diagnostics.value,
  copyDiagnostics: async () => '',
}));
afterEach(cleanup);

function withLatency(latency: DiagnosticsValue['levels']['latency']): DiagnosticsValue {
  return {
    link: structuredClone(emptyLink),
    levels: { ...structuredClone(emptyLevels), latency },
    source: null,
  };
}

test('poke delay shows the median and p95 with the number of pokes measured', async () => {
  diagnostics.value = withLatency({ pokes: 12, medianMs: 147.6, p95Ms: 212.2 });
  render(<Diagnostics />);
  const delay = await screen.findByText('Poke delay (median / p95)');
  expect(delay.nextElementSibling?.textContent).toBe('148 / 212 ms');
  expect(screen.getByText('Pokes measured').nextElementSibling?.textContent).toBe('12');
});

test('poke delay shows a dash before the first poke', async () => {
  diagnostics.value = withLatency({ pokes: 0, medianMs: null, p95Ms: null });
  render(<Diagnostics />);
  const delay = await screen.findByText('Poke delay (median / p95)');
  expect(delay.nextElementSibling?.textContent).toBe('—');
  expect(screen.getByText('Pokes measured').nextElementSibling?.textContent).toBe('0');
});
