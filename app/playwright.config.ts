import { defineConfig } from '@playwright/test';
const preview = process.env.OPENMATH_E2E_PREVIEW === '1';
export default defineConfig({
  testDir: './e2e',
  testIgnore: preview ? ['**/kernel.spec.ts'] : [],
  timeout: 60_000,
  use: { baseURL: 'http://127.0.0.1:5173', headless: true, launchOptions: { args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] } },
  webServer: { command: preview ? 'npm run preview' : 'npm run dev', url: 'http://127.0.0.1:5173', reuseExistingServer: !preview && !process.env.CI, timeout: 120_000 },
});
