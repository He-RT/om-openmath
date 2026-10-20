import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';
import wasm from 'vite-plugin-wasm';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const version = (JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8')) as { version: string }).version;
const revision = process.env.OPENMATH_SOURCE_REVISION ?? 'development';
if (revision !== 'development' && !/^[a-f0-9]{40}$/.test(revision)) throw new Error('Invalid source revision');
const buildInfo = { version, revision };

export default defineConfig({
  plugins: [react(), wasm(), {
    name: 'openmath-build-info',
    generateBundle() { this.emitFile({ type: 'asset', fileName: 'version.json', source: JSON.stringify(buildInfo, null, 2) + '\n' }); },
  }],
  define: { __OPENMATH_BUILD__: JSON.stringify(buildInfo) },
  clearScreen: false,
  server: {
    host: '127.0.0.1', port: 5173, strictPort: true,
    fs: { allow: [
      fileURLToPath(new URL('./', import.meta.url)),
      fileURLToPath(new URL('../agent/prompts/', import.meta.url)),
    ] },
  },
  test: {
    exclude: ['e2e/**', 'node_modules/**'],
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
  },
});
