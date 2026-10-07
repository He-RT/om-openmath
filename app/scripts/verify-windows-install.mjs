// Connect to the installed application's real WebView2, never a browser mock.
import { chromium, expect } from '@playwright/test';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
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
  // The installed desktop must also exercise the new native kernel -> WebGL2 path.
  await first.fill(await readFile(new URL('../../docs/examples/watermelon.om',import.meta.url),'utf8'));
  await first.press('Shift+Enter');
  const scene=page.locator('.scene-view');
  await expect(scene).toBeVisible({timeout:30_000});
  await expect(scene.getByRole('alert')).toHaveCount(0);
  const canvas=scene.locator('canvas');
  const fingerprint=()=>canvas.evaluate(c=>{
    const gl=c.getContext('webgl2');if(!gl)throw new Error('Installed desktop needs real WebGL2');
    const data=new Uint8Array(c.width*c.height*4);gl.readPixels(0,0,c.width,c.height,gl.RGBA,gl.UNSIGNED_BYTE,data);
    let sum=0,colored=0;for(let i=0;i<data.length;i+=4){sum=(sum+data[i]*3+data[i+1]*7+data[i+2]*11)%1_000_000_007;if(data[i]<150||data[i+1]<150||data[i+2]<150)colored++;}return{sum,colored};
  });
  const original=await fingerprint();expect(original.colored).toBeGreaterThan(1000);
  await scene.getByRole('button',{name:'右转',exact:true}).click();
  await expect.poll(async()=>(await fingerprint()).sum).not.toBe(original.sum);
  await scene.getByRole('button',{name:'复位视窗',exact:true}).click();
  await expect.poll(async()=>(await fingerprint()).sum).toBe(original.sum);
  await scene.screenshot({path:`${output}/watermelon-native.png`});
  await scene.getByText('内核网格数据',{exact:true}).click();
  const meshData=scene.locator('details').filter({has:page.getByText('内核网格数据',{exact:true})}).locator('pre');
  await expect(meshData).toBeVisible();
  const geometry=JSON.parse(await meshData.textContent());
  expect(geometry.meshes).toHaveLength(16);expect(geometry.labels).toHaveLength(2);
  await scene.getByText('内核网格数据',{exact:true}).click();
  await scene.screenshot({path:`${output}/watermelon-native.png`});
  expect(errors).toEqual([]);
  await writeFile(`${output}/result.json`, JSON.stringify({
    installed: true, native: true, locale: 'zh-CN',
    checks: ['exact -3/1 roots', 'recorded steps', 'reactive 3 to 6', 'exact radical roots', 'native kernel full watermelon 16 meshes', 'actual WebGL2 pixels rotation reset'],
    errors,
  }, null, 2));
} catch(error) {
  const page=browser.contexts()[0]?.pages()[0];
  if(page){
    await page.screenshot({path:`${output}/failure-window.png`}).catch(()=>{});
    await writeFile(`${output}/failure-context.txt`,(await page.locator('body').innerText()).slice(0,16000)).catch(()=>{});
  }
  throw error;
} finally {
  await browser.close();
}
