import {test,expect} from '@playwright/test';
import {readFile} from 'node:fs/promises';
test('real CSV/JSON exports every retained row, and PNG/SVG read back actual current parameters and geometry',async({page})=>{
 await page.setViewportSize({width:1280,height:960});await page.goto('/');await page.getByRole('button',{name:'＋ Math',exact:true}).click();
 const editor=page.getByRole('textbox',{name:'Math input 1',exact:true});
 let csv='x,说明\n';for(let i=0;i<40;i++)csv+=`${i},中文🙂 ${i}\n`;
 const run=async(source:string)=>{await editor.fill(source);await editor.press('ControlOrMeta+Enter');await editor.press('Escape');await expect(page.locator('.notebook-cell').first()).toHaveAttribute('data-status','Done');};
 await run(`parse_csv(${JSON.stringify(csv)})`);
 const download=async(selector:ReturnType<typeof page.getByRole>)=>{const waiting=page.waitForEvent('download');await selector.click();const d=await waiting;const path=await d.path();if(!path)throw new Error('No actual export');return {bytes:await readFile(path),name:d.suggestedFilename()};};
 const table=page.locator('.structured-value');await expect(table).toContainText('40 rows');
 const full=await download(table.getByRole('button',{name:'Export CSV',exact:true}));expect(full.bytes.toString()).toBe(csv.replaceAll('\n','\r\n'));expect(full.name).toMatch(/\.csv$/);
 const json=await download(table.getByRole('button',{name:'Export JSON',exact:true}));expect(JSON.parse(json.bytes.toString()).rows).toHaveLength(40);expect(JSON.parse(json.bytes.toString()).rows[39]['说明']).toBe('中文🙂 39');
 await run('let a=99;explore(plot(a*x,x:0..1),controls:{a:0..4},initial:{a:2})');
 const explore=page.locator('.explore-view');const slider=explore.getByRole('slider',{name:'Explore parameter a'});await expect(slider).toBeEnabled();await slider.press('End');await expect(explore).toHaveAttribute('aria-busy','false');
 const plot=explore.locator('.plot-view');const svg=await download(plot.getByRole('button',{name:'Export SVG',exact:true}));
 const source=svg.bytes.toString();expect(source).toContain('<metadata>');const raw=source.match(/<metadata>(.*?)<\/metadata>/s)![1]!.replaceAll('&quot;','"').replaceAll('&lt;','<').replaceAll('&gt;','>').replaceAll('&apos;',"'").replaceAll('&amp;','&');const meta=JSON.parse(raw);expect(meta.figure.parameters.a).toBe(4);
 for(const [x,y]of meta.figure.data.curves[0].segments.flat())expect(y).toBeCloseTo(4*x,12);
 const png=await download(plot.getByRole('button',{name:'Export PNG',exact:true}));expect(png.bytes.subarray(0,8)).toEqual(Buffer.from([137,80,78,71,13,10,26,10]));
 const {inflateSync}=await import('node:zlib');let at=8,idat=Buffer.alloc(0);let metadata='';while(at<png.bytes.length){const n=png.bytes.readUInt32BE(at),kind=png.bytes.toString('ascii',at+4,at+8),data=png.bytes.subarray(at+8,at+8+n);if(kind==='IDAT')idat=Buffer.concat([idat,data]);if(kind==='iTXt')metadata=data.subarray(13).toString();at+=n+12;}
 expect(inflateSync(idat)).toHaveLength(600*4001);expect(JSON.parse(metadata).figure.parameters.a).toBe(4);
 await page.screenshot({path:'test-results/plot-export.png',fullPage:true});
 await run('explore([[a,a^2]],controls:{a:0..4},initial:{a:3})');const matrix=await download(page.locator('.explore-data-export').getByRole('button',{name:'Export CSV',exact:true}));expect(matrix.bytes.toString()).toBe('3,9\r\n');
});
