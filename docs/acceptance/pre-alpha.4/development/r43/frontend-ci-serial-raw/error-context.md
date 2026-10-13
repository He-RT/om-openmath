# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: science-scenarios.spec.ts >> complete watermelon is actual kernel geometry with skin cut rind seeds GPU rotation and OBJ readback
- Location: e2e/science-scenarios.spec.ts:17:1

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('.scene-view')
Expected: visible
Timeout: 30000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" locator('.scene-view') with timeout 30000ms
  - waiting for locator('.scene-view')

```

```yaml
- banner:
  - strong: OpenMath
  - text: /
  - textbox "Notebook title":
    - /placeholder: Untitled notebook
  - text: • Dialect
  - combobox "Dialect":
    - option "Automatic" [selected]
    - option "Modern"
    - option "Wolfram"
  - button "Run all" [disabled]: ▶ Run all
  - button "Interrupt": Interrupt ■
  - button "Save"
  - button "Command palette": ⌘K
  - button "Preferences": ☷
  - button "AI settings": AI
  - group: ⋯
- main "Untitled notebook":
  - text: "[ ] 01"
  - combobox "Math / Text 1":
    - option "Math" [selected]
    - option "Text"
    - option "Ask AI"
  - combobox "Dialect 1":
    - option "Automatic" [selected]
    - option "Modern"
    - option "Wolfram"
  - text: Running
  - button "Run 1" [disabled]: ▶
  - button "Move up 1": ↑
  - button "Move down 1": ↓
  - button "Delete cell 1": ×
  - textbox "Math input 1"
  - text: Complete the expression to preview it.
  - button "＋ Math"
  - button "＋ Text"
  - button "＋ Ask AI"
  - text: Shift Enter to run · ⌘/Ctrl K for commands Web · WASM
- complementary:
  - navigation:
    - button "Docs" [pressed]
    - button "Variables"
    - button "Steps"
    - button "Assistant"
  - button "Close": ×
  - button "Solve"
  - button "Factor"
  - button "Simplify"
  - button "Expand"
  - button "Reduce"
  - button "N"
  - button "Plot"
  - article:
    - heading "Solve" [level=2]
    - code: Solve[eqs, vars, domain, options]
    - paragraph: Solve equations while preserving source restrictions.
    - button "Solve[x^2==2,x] ↗":
      - code: Solve[x^2==2,x]
      - text: ↗
```

# Test source

```ts
  1  | import {test,expect} from '@playwright/test';
  2  | import {readFile} from 'node:fs/promises';
  3  | test('real L2 source shows high precision report and proportional markers and exports Unicode SVG',async({page})=>{
  4  |  await page.setViewportSize({width:1280,height:960});await page.goto('/');await page.getByRole('button',{name:'＋ Math',exact:true}).click();
  5  |  const editor=page.getByRole('textbox',{name:'Math input 1',exact:true});
  6  |  await editor.fill(await readFile(new URL('../../docs/examples/earth-moon-l2.om',import.meta.url),'utf8'));await editor.press('ControlOrMeta+Enter');await editor.press('Escape');
  7  |  const plot=page.locator('.plot-view');await expect(plot).toBeVisible();await expect(plot.getByRole('alert')).toHaveCount(0);
  8  |  await expect(page.getByRole('rowheader',{name:'barycentric_km',exact:true})).toBeVisible();
  9  |  await expect(plot.locator('svg circle')).toHaveCount(4);await expect(plot.locator('svg text').filter({hasText:'月球'})).toBeVisible();
  10 |  const xs=await plot.locator('svg circle').evaluateAll(nodes=>nodes.map(n=>Number(n.getAttribute('cx'))));
  11 |  expect((xs[3]!-xs[2]!)/(xs[2]!-xs[0]!)).toBeCloseTo(64514.9072421657/384400,10);
  12 |  await plot.screenshot({path:'test-results/scenario-l2.png'});
  13 |  const download=page.waitForEvent('download');await plot.getByRole('button',{name:'Export SVG',exact:true}).click();const file=await download;const path=await file.path();if(!path)throw new Error('No actual SVG');
  14 |  const svg=await readFile(path,'utf8');expect(svg).toContain('地球');expect(svg).toContain('月球');expect(svg).toContain('444244.22260083933');
  15 |  await plot.getByRole('button',{name:'Zoom in',exact:true}).click();await expect(plot).toHaveAttribute('aria-busy','false');await expect(plot.getByRole('alert')).toHaveCount(0);
  16 | });
  17 | test('complete watermelon is actual kernel geometry with skin cut rind seeds GPU rotation and OBJ readback',async({page})=>{
  18 |  await page.setViewportSize({width:1280,height:960});await page.goto('/');await page.getByRole('button',{name:'＋ Math',exact:true}).click();
  19 |  const editor=page.getByRole('textbox',{name:'Math input 1',exact:true});
  20 |  await editor.fill(await readFile(new URL('../../docs/examples/watermelon.om',import.meta.url),'utf8'));await editor.press('ControlOrMeta+Enter');await editor.press('Escape');
> 21 |  const scene=page.locator('.scene-view');await expect(scene).toBeVisible({timeout:30_000});await expect(scene.getByRole('alert')).toHaveCount(0);
     |                                                              ^ Error: expect(locator).toBeVisible() failed
  22 |  const canvas=scene.locator('canvas');await expect(canvas).toBeVisible();
  23 |  const pixels=()=>canvas.evaluate((c:HTMLCanvasElement)=>{const gl=c.getContext('webgl2')!;const a=new Uint8Array(c.width*c.height*4);gl.readPixels(0,0,c.width,c.height,gl.RGBA,gl.UNSIGNED_BYTE,a);let green=0,red=0,dark=0,sum=0;for(let i=0;i<a.length;i+=4){const r=a[i]!,g=a[i+1]!,b=a[i+2]!;if(g>r*1.7&&g>b*1.3)green++;if(r>g*1.6&&r>b*1.1)red++;if(r<65&&g<65&&b<65)dark++;sum=(sum+r*3+g*7+b*11)%1_000_000_007;}return {green,red,dark,sum};});
  24 |  const first=await pixels();expect(first.green).toBeGreaterThan(500);expect(first.red).toBeGreaterThan(200);expect(first.dark).toBeGreaterThan(20);
  25 |  await scene.screenshot({path:'test-results/scenario-watermelon.png'});
  26 |  await scene.getByRole('button',{name:'Rotate right',exact:true}).click();await expect.poll(async()=>(await pixels()).sum).not.toBe(first.sum);
  27 |  await scene.getByRole('button',{name:'Reset view',exact:true}).click();await expect.poll(async()=>(await pixels()).sum).toBe(first.sum);
  28 |  const download=page.waitForEvent('download');await scene.getByRole('button',{name:'Export OBJ',exact:true}).click();const file=await download;const path=await file.path();if(!path)throw new Error('No actual OBJ');
  29 |  const obj=await readFile(path,'utf8');const data=JSON.parse(obj.split('\n').find(v=>v.startsWith('# scene_json '))!.slice(13)).scene;
  30 |  expect(data.meshes).toHaveLength(16);expect(data.labels).toHaveLength(2);
  31 |  await scene.getByText('Kernel mesh data',{exact:true}).click();
  32 |  const meshData=scene.locator('details').filter({has:page.getByText('Kernel mesh data',{exact:true})}).locator('pre');
  33 |  await expect(meshData).toBeVisible();
  34 |  const displayed=JSON.parse((await meshData.textContent())!);
  35 |  expect(displayed).toEqual(data);
  36 |  await scene.getByText('Kernel mesh data',{exact:true}).click();
  37 |  expect(obj.split('\n').filter(v=>v.startsWith('f ')).length).toBe(data.meshes.reduce((n:number,m:{triangles:unknown[]})=>n+m.triangles.length,0));
  38 |  await page.setViewportSize({width:390,height:900});const close=page.locator('.inspector').getByRole('button',{name:'Close',exact:true});if(await close.isVisible())await close.click();await canvas.scrollIntoViewIfNeeded();expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  39 |  await scene.screenshot({path:'test-results/scenario-watermelon-narrow.png'});
  40 | });
  41 | 
```