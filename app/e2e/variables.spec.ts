import { test, expect } from '@playwright/test';

for (const width of [1440, 390]) test(`accepted variables locate cells and reflect draft/error state at ${width}px`, async ({ page }) => {
  await page.setViewportSize({ width, height: 900 }); await page.goto('/');
  await page.getByRole('button', { name: '＋ Math', exact: true }).click();
  const editor = page.getByRole('textbox', { name: 'Math input 1', exact: true });
  await editor.fill('let a=2'); await editor.press('ControlOrMeta+Enter'); await editor.press('Escape');
  await expect(page.locator('.notebook-cell').first()).toHaveAttribute('data-status', 'Done');
  const openVariables = async () => {
    if (width < 760) await page.getByRole('button', { name: 'Assistant', exact: true }).click();
    await page.getByRole('tab', { name: 'Variables', exact: true }).click();
  };
  await openVariables();
  const table = page.locator('.variables-table');
  await expect(table.getByRole('columnheader')).toHaveText(['Name', 'Value', 'From']);
  await expect(table.getByRole('row', { name: 'a 2 Cell 1', exact: true })).toBeVisible();
  await table.getByRole('row', { name: 'a 2 Cell 1', exact: true }).press('Enter');
  if (width < 760) await expect(page.locator('.inspector-host')).not.toBeVisible();
  // Locating does not steal the notebook editor selection or run a cell.
  await editor.fill('let a=5');
  if (width < 760) await openVariables();
  await expect(table.getByRole('row', { name: 'a 2 Out of date Cell 1', exact: true })).toBeVisible();
  if (width < 760) await page.locator('.inspector').getByRole('button', { name: 'Close', exact: true }).click();
  await editor.press('ControlOrMeta+Enter'); await editor.press('Escape');
  if (width < 760) await openVariables();
  await expect(table.getByRole('row', { name: 'a 5 Cell 1', exact: true })).toBeVisible();
  if (width < 760) await page.locator('.inspector').getByRole('button', { name: 'Close', exact: true }).click();
  await editor.fill('let broken=1\n(');
  await editor.press('ControlOrMeta+Enter'); await editor.press('Escape');
  await expect(page.locator('.notebook-cell').first()).toHaveAttribute('data-status', 'Error');
  if (width < 760) await openVariables();
  await expect(table.getByRole('row', { name: 'broken Undefined, cell has an error Cell 1', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: `../target/plan-5/e5-variables-${width}.png`, fullPage: true });
});
