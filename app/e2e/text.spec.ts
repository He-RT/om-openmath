import { expect,test } from "@playwright/test";
const code='let α = 2\n  map(fn(x) => x^2 + α, [1, 2, 3])\n'+"# "+"horizontal code remains on one line · ".repeat(12);
const source='# Notes\n\nRead **carefully** and compare $x^2$ and \\( a+1=6 \\).\n\n\\[\n\\frac{1}{2}\n\\]\n\n```openmath\n'+code+'\n```\n\n```javascript\n<script>literal()</script>\n```';
for(const width of [1440,390]) test(`loaded text reads first and fenced code preserves original source at ${width}px`,async({page,context})=>{
  await context.grantPermissions(["clipboard-read","clipboard-write"]);
  await page.setViewportSize({width,height:900});await page.goto("/");
  await expect(page.getByRole("button",{name:"＋ Text",exact:true})).toBeVisible();
  await page.locator('input[accept*=".omnb"]').setInputFiles({name:'text.omnb',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,title:'Text and code',cells:[{id:'text',kind:'Text',source,dialect:'Modern'}]}))});
  await expect(page.getByRole("heading",{name:"Notes",exact:true})).toBeVisible();
  await expect(page.getByRole("textbox",{name:"Text input 1",exact:true})).toHaveCount(0);
  await expect(page.locator('.text-reading-content .katex')).toHaveCount(3);
  const block=page.locator('.text-cell .code-block[data-openmath="true"]'),pre=block.locator('pre');
  await expect(pre).toHaveText(code);expect(await pre.evaluate(node=>getComputedStyle(node).whiteSpace)).toBe('pre');
  expect(await pre.evaluate(node=>node.scrollWidth>node.clientWidth)).toBe(true);
  await expect(page.locator('.code-block[data-openmath="false"]').getByRole('button',{name:'Insert as cell',exact:true})).toHaveCount(0);
  await block.getByRole('button',{name:'Copy',exact:true}).click();expect(await page.evaluate(()=>navigator.clipboard.readText())).toBe(code);
  await block.getByRole('button',{name:'Insert as cell',exact:true}).click();
  await expect(page.getByRole('textbox',{name:'Math input 2',exact:true})).toContainText('map(fn(x)');
  await expect(page.locator('.notebook-cell').nth(1)).toHaveAttribute('data-status','Stale');
  await expect(page.locator('.notebook-cell').nth(1).locator('.cell-output')).toHaveCount(0);
  await page.locator('.text-reading-content p').first().dblclick();
  const editor=page.getByRole('textbox',{name:'Text input 1',exact:true});await expect(editor).toHaveValue(source);
  await editor.fill(source.replace('# Notes','# Updated notes'));
  await editor.press('ControlOrMeta+Enter');await expect(page.getByRole('heading',{name:'Updated notes',exact:true})).toBeVisible();
  await expect(editor).toHaveCount(0);expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
});
test('new text edits immediately, keeps its DOM/selection across breakpoints and exits on Escape',async({page})=>{
  await page.setViewportSize({width:1440,height:900});await page.goto('/');await page.getByRole('button',{name:'＋ Text',exact:true}).click();
  const editor=page.getByRole('textbox',{name:'Text input 1',exact:true});await expect(editor).toBeFocused();
  await editor.fill('# New paragraph\n\nSource stays.');await editor.press('Home');await editor.press('Shift+ArrowRight');
  const node=await editor.elementHandle(),selection=await editor.evaluate(element=>[(element as HTMLTextAreaElement).selectionStart,(element as HTMLTextAreaElement).selectionEnd]);
  await page.setViewportSize({width:390,height:900});
  await expect.poll(()=>page.locator('.inspector-host').evaluate(element=>element.matches(':modal'))).toBe(true);
  await page.keyboard.press('Escape');await expect(page.locator('.inspector-host')).not.toBeVisible();
  expect(await editor.evaluate((element,original)=>element===original,node)).toBe(true);
  expect(await editor.evaluate(element=>[(element as HTMLTextAreaElement).selectionStart,(element as HTMLTextAreaElement).selectionEnd])).toEqual(selection);
  await editor.press('Escape');await expect(page.getByRole('heading',{name:'New paragraph',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Edit Text 1',exact:true}).click();await expect(editor).toHaveValue('# New paragraph\n\nSource stays.');
});

test('actual numerical record output uses a structured status instead of claiming an exact scalar',async({page})=>{
  await page.goto('/');await page.locator('input[accept*=".omnb"]').setInputFiles({name:'numerical-record.omnb',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,title:'Numerical record',cells:[{id:'integral',kind:'Math',dialect:'Wolfram',source:'NIntegrate[Sin[x^2], {x, 0, 1}, Method -> "gauss_kronrod"]'}]}))});
  await page.getByRole('button',{name:'Run all',exact:true}).click();await expect(page.locator('.notebook-cell')).toHaveAttribute('data-presentation','structured');await expect(page.locator('.cell-status-line')).toContainText('Structured result');await expect(page.locator('.structured-value')).toContainText('0.310268');
});
