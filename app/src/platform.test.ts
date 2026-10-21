import { afterEach, expect, test, vi } from 'vitest';
import { hasNativeFiles, usesNativeKernel } from './platform';
afterEach(() => { vi.unstubAllEnvs(); delete (window as Window & {__TAURI_INTERNALS__?: unknown}).__TAURI_INTERNALS__; });
test('packaged WASM retains native files without selecting the legacy native kernel', () => {
  Object.defineProperty(window, '__TAURI_INTERNALS__', {value: {}, configurable: true});
  vi.stubEnv('VITE_OPENMATH_WEB_SHELL', '1');
  expect(hasNativeFiles('wasm')).toBe(true);
  expect(usesNativeKernel()).toBe(false);
  vi.stubEnv('VITE_OPENMATH_WEB_SHELL', '');
  expect(usesNativeKernel()).toBe(true);
  delete (window as Window & {__TAURI_INTERNALS__?: unknown}).__TAURI_INTERNALS__;
  expect(hasNativeFiles('wasm')).toBe(false);
  expect(usesNativeKernel()).toBe(false);
});
