import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

const here = (path: string) => fileURLToPath(new URL(path, import.meta.url));

export default defineConfig({
  base: './',
  plugins: [react()],
  server: { host: '127.0.0.1', port: 3000, strictPort: true, fs: { allow: [here('..')] } },
  build: {
    rollupOptions: {
      input: {
        home: here('index.html'),
        docs: here('docs/index.html'),
        changelog: here('changelog/index.html'),
      },
    },
  },
  test: { environment: 'node', include: ['src/**/*.test.{ts,tsx}'] },
});
