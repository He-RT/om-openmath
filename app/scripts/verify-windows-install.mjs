// Connect to the installed application's real WebView2, never a browser mock.
import { chromium, expect } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

const output = `test-results/windows-${process.env.OPENMATH_INSTALL_KIND ?? 'install'}`;
await mkdir(output, { recursive: true });
let browser;
let connectionError;
const deadline = Date.now() + 90_000;
while (Date.now() < deadline) {
  try {
    browser = await chromium.connectOverCDP('http://127.0.0.1:9222', { timeout: 5000 });
    break;
  } catch (error) {
    connectionError = error.message;
    await delay(1000);
  }
}
if (!browser) {
  await writeFile(`${output}/connection-failure.json`, JSON.stringify({ connectionError }, null, 2));
  throw new Error(`Installed OpenMath WebView2 connection failed: ${connectionError}`);
}
try {
  const context = browser.contexts()[0];
  if (!context) throw new Error('Missing native browser context');
  await expect.poll(() => context.pages().length, { timeout: 30_000 }).toBeGreaterThan(0);
  const page = context.pages()[0];
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await expect.poll(() => page.title()).toBe('OpenMath');
  if (!await page.evaluate(() => Boolean(globalThis.__TAURI_INTERNALS__))) {
    throw new Error('Expected the installed Tauri window');
  }
  // CI supplies an isolated zh-CN native config; this also verifies localization.
  await page.getByRole('button', { name: /二次方程/ }).click();
  await expect(page.locator('.solution-list')).toContainText('-3');
  await expect(page.locator('.solution-list')).toContainText('1');
  await page.locator('.solution-actions').getByRole('button', { name: /步骤/ }).click();
  await expect(page.locator('.step-node').first()).toBeVisible();
  await page.screenshot({ path: `${output}/exact-solutions.png`, fullPage: true });

  const first = page.getByRole('textbox', { name: '数学输入 1', exact: true });
  await first.fill('let a=2');
  await first.press('Shift+Enter');
  const second = page.getByRole('textbox', { name: '数学输入 2', exact: true });
  await second.fill('a+1');
  await second.press('Control+Enter');
  const result = page.locator('.notebook-cell').nth(1).locator('.cell-output');
  await expect(result).toContainText('3');
  await first.fill('let a=5');
  await first.press('Control+Enter');
  await expect(result).toContainText('6');
  await second.fill('solve(x^2==2,x)');
  await second.press('Control+Enter');
  const radicals = page.locator('.notebook-cell').nth(1).locator('.solution-list');
  await expect(radicals.locator('.solution-chip')).toHaveCount(2);
  await expect(radicals).toContainText('精确');
  await expect(radicals.locator('annotation').first()).toContainText('\\sqrt{2}');
  await expect(page.locator('.notebook-cell').nth(1)).toHaveAttribute('data-status', 'Done');
  expect(errors).toEqual([]);
  await page.screenshot({ path: `${output}/reactive-and-radicals.png`, fullPage: true });
  await writeFile(`${output}/result.json`, JSON.stringify({
    installed: true, native: true, locale: 'zh-CN',
    checks: ['exact -3/1 roots', 'recorded steps', 'reactive 3 to 6', 'exact radical roots'],
    errors,
  }, null, 2));
} finally {
  await browser.close();
}
