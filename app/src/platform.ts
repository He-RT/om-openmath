/** A packaged Web shell uses native files independently of its WASM calculation worker. */
export function hasNativeFiles(kind: 'wasm' | 'tauri') {
  return kind === 'tauri' || '__TAURI_INTERNALS__' in window;
}
export function usesNativeKernel() {
  return '__TAURI_INTERNALS__' in window && import.meta.env.VITE_OPENMATH_WEB_SHELL !== '1';
}
