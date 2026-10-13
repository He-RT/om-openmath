import {test,expect} from '@playwright/test';
import {readFile} from 'node:fs/promises';
test('actual kernel 3D mesh uses real WebGL2, rotates zooms resets and exports original OBJ',async({page})=>{
 await page.setViewportSize({width:1280,height:960});await page.goto('/');await page.getByRole('button',{name:'＋ Math',exact:true}).click();
 const editor=page.getByRole('textbox',{name:'Math input 1',exact:true});
 await editor.fill('parametric_plot([cos(u)*sin(v),sin(u)*sin(v),cos(v)],u:0..2*pi,v:0..pi,mesh_points:16,color:fn(p,u,v)=>[0.1,0.5+0.3*cos(12*u),0.2])');await editor.press('ControlOrMeta+Enter');await editor.press('Escape');
 const scene=page.locator('.scene-view');await expect(scene).toBeVisible();await expect(scene.getByRole('alert')).toHaveCount(0);
 const canvas=scene.locator('canvas');await expect(canvas).toBeVisible();
 const hash=()=>canvas.evaluate((c:HTMLCanvasElement)=>{
  const gl=c.getContext('webgl2')!;const data=new Uint8Array(c.width*c.height*4);gl.readPixels(0,0,c.width,c.height,gl.RGBA,gl.UNSIGNED_BYTE,data);
  let sum=0,count=0;for(let i=0;i<data.length;i+=4){sum=(sum+data[i]!*3+data[i+1]!*7+data[i+2]!*11)%1_000_000_007;if(data[i]!<150||data[i+1]!<150||data[i+2]!<150)count++;}return{sum,count};
 });
 const initial=await hash();expect(initial.count).toBeGreaterThan(2000);
 await scene.getByRole('button',{name:'Rotate right',exact:true}).click();await expect.poll(async()=>(await hash()).sum).not.toBe(initial.sum);
 await scene.getByRole('button',{name:'Zoom in',exact:true}).click();const zoomed=await hash();expect(zoomed.count).toBeGreaterThan(initial.count);
 await scene.getByRole('button',{name:'Reset view',exact:true}).click();await expect.poll(async()=>(await hash()).sum).toBe(initial.sum);
 await page.screenshot({path:'test-results/scene-sphere.png',fullPage:true});
 const download=page.waitForEvent('download');await scene.getByRole('button',{name:'Export OBJ',exact:true}).click();const file=await download;const path=await file.path();if(!path)throw new Error('No actual OBJ');const source=await readFile(path,'utf8');
 const metadata=JSON.parse(source.split('\n').find(line=>line.startsWith('# scene_json '))!.slice(13));expect(metadata.scene.meshes[0].triangles.length).toBeGreaterThan(100);
 const vertices=source.split('\n').filter(line=>line.startsWith('v ')).map(line=>line.split(' ').slice(1).map(Number));for(const p of vertices)expect(p[0]!**2+p[1]!**2+p[2]!**2).toBeCloseTo(1,11);
 await editor.fill('explore(plot(a*x^2-y^2,x:-1..1,y:-1..1,mesh_points:12),controls:{a:0.5..3},initial:{a:1})');await editor.press('ControlOrMeta+Enter');await editor.press('Escape');
 const explore=page.locator('.explore-view'),slider=explore.getByRole('slider',{name:'Explore parameter a'});await expect(slider).toBeEnabled();await scene.getByRole('button',{name:'Rotate left',exact:true}).click();await slider.press('End');await expect(explore).toHaveAttribute('aria-busy','false');
 await expect(scene.getByRole('alert')).toHaveCount(0);await expect(scene).toContainText('actual triangles');
 await page.setViewportSize({width:390,height:900});const close=page.locator('.inspector').getByRole('button',{name:'Close',exact:true});if(await close.isVisible())await close.click();await canvas.scrollIntoViewIfNeeded();expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
 await page.screenshot({path:'test-results/scene-narrow.png',fullPage:true});
});
