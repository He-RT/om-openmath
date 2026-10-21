import { chromium } from '@playwright/test';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';
const output = process.env.OPENMATH_PACKAGE_EVIDENCE ?? 'test-results/windows-shell';
await mkdir(output, {recursive: true});
let browser;
for (let attempt = 0; attempt < 90 && !browser; attempt++) {
  try { browser = await chromium.connectOverCDP('http://127.0.0.1:9222', {timeout: 2000}); }
  catch { await delay(1000); }
}
if (!browser) throw new Error('Installed WebView2 did not expose the CI-only inspection channel');
try {
  let page;
  for (let attempt=0; attempt<60 && !page; attempt++) {
    page=browser.contexts().flatMap(context=>context.pages()).find(item=>item.url().includes('tauri.localhost'));
    if(!page) await delay(500);
  }
  if (!page) throw new Error('Installed Tauri workbench did not open');
  await page.evaluate(await readFile(new URL('./package-smoke.js', import.meta.url), 'utf8'));
  await page.waitForFunction(() => ['passed','failed'].includes(window.__openmathPackageSmoke?.state), undefined, {timeout: 100_000});
  const result=await page.evaluate(()=>window.__openmathPackageSmoke);
  await writeFile(`${output}/result.json`, JSON.stringify(result,null,2)+'\n');
  await page.screenshot({path:`${output}/window.png`});
  if(result.state!=='passed')throw new Error(JSON.stringify(result));
  if(result.version.revision!==process.env.OPENMATH_SOURCE_REVISION)throw new Error('Installed source revision differs');
} finally { await browser.close(); }
