import js from '@eslint/js';
import globals from 'globals';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist/**', 'node_modules/**', 'src-tauri/**', 'src/kernel/wasm/**', 'playwright-report/**', 'test-results/**'] },
  js.configs.recommended,
  tseslint.configs.recommended,
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: { globals: globals.browser },
  },
  {
    files: ['*.{js,ts}'],
    languageOptions: { globals: globals.node },
  },
  { files: ['scripts/**/*.mjs', 'e2e/**/*.ts'], languageOptions: { globals: globals.node } },
  { files: ['scripts/package-smoke.js'], languageOptions: { globals: globals.browser } },
  // These callbacks execute in the installed WebView through Playwright.
  { files: ['scripts/check-installed-shell.mjs'], languageOptions: { globals: { window: 'readonly' } } },
);
