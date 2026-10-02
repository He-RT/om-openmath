import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  plugins: [react(), wasm()],
  clearScreen: false,
  server: { host: '127.0.0.1', port: 5173, strictPort: true },
  test: {
    exclude: ['e2e/**', 'node_modules/**'],
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
  },
});
