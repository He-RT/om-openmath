import {fixtureProfile} from './profileFixture';
import {test,expect,type Page} from '@playwright/test';
import {s1Response} from './agentLoopback';
async function setup(page:Page){
  await fixtureProfile(page,'https://ai-provider.invalid/v1','synthetic-ui-key');
  await page.addInitScript(()=>{const config=JSON.parse(localStorage.getItem('openmath-settings-v1')!);config.profiles.push({...config.profiles[0],name:'second',model:'second-loopback'},{...config.profiles[0],name:'text-only',model:'text-only-loopback',supports_tools:false});localStorage.setItem('openmath-settings-v1',JSON.stringify(config));});
}
async function accept(page:Page){await page.getByRole('checkbox',{name:'Remember for this session and destination',exact:true}).check();await page.getByRole('button',{name:'Send to AI',exact:true}).click();}
for(const width of [1440,390])test(`v2 assistant uses real source tools, fact outcomes and reversible turns at ${width}px`,async({page,context})=>{
  await page.setViewportSize({width,height:900});await setup(page);
  const bodies:{model:string;messages:{role:string;content?:string}[]}[]=[];
  await context.route('https://ai-provider.invalid/**',async route=>{const body=route.request().postDataJSON();bodies.push(body);await route.fulfill({status:200,contentType:'text/event-stream',headers:{'access-control-allow-origin':'*'},body:s1Response(body)});});
  await page.goto('/');if(width>=760)await page.getByRole('combobox',{name:'Run mode',exact:true}).selectOption('manual');
  await page.getByRole('button',{name:'＋ Math',exact:true}).click();await page.getByRole('textbox',{name:'Math input 1',exact:true}).fill('let a=2');
  await page.getByRole('button',{name:'＋ Math',exact:true}).click();await page.getByRole('textbox',{name:'Math input 2',exact:true}).fill('a+1');
  await page.getByRole('button',{name:'Run all',exact:true}).click();await expect(page.locator('.notebook-cell').nth(1)).toHaveAttribute('data-status','Done');
  if(width<760)await page.getByRole('button',{name:'Assistant',exact:true}).click();
  const input=page.getByRole('textbox',{name:'Message to assistant',exact:true});await input.fill('Please change a to 5 and update its dependent results.');await input.press('Enter');
  await expect(input).toHaveValue('Please change a to 5 and update its dependent results.');expect(bodies).toHaveLength(0);await accept(page);
  await expect(page.locator('.agent-end')).toContainText('Turn ended');await expect(input).toHaveValue('');
  await expect(page.locator('.agent-outcome')).toContainText('Ran 2 cells, no diagnostics');await expect(page.locator('.agent-outcome')).toContainText('Not saved to a file');
  await expect(page.locator('.agent-action[data-action-status=failed]')).toContainText('INVALID_ARGUMENT');await expect(page.locator('.action-summary')).toHaveAttribute('aria-expanded','false');
  await page.locator('.action-summary').click();await expect(page.locator('.agent-actions .agent-action')).toHaveCount(7);
  await page.screenshot({animations:'disabled',path:`../target/plan-5/f3-v2-${width}-light.png`});if(width===1440){await page.emulateMedia({colorScheme:'dark'});await page.screenshot({animations:'disabled',path:'../target/plan-5/f3-v2-1440-dark.png'});await page.emulateMedia({colorScheme:'light'});await page.setViewportSize({width:1000,height:900});await page.screenshot({animations:'disabled',path:'../target/plan-5/f3-v2-1000-light.png'});await page.setViewportSize({width,height:900});}
  await page.locator('.agent-action').filter({hasText:'Edit notebook'}).getByRole('button',{name:'View changes'}).click();await expect(page.locator('.agent-diff')).toContainText('let a=2');await expect(page.locator('.agent-diff')).toContainText('let a=5');
  await page.locator('.agent-outcome').getByRole('button',{name:'Undo turn changes',exact:true}).click();await expect(page.locator('.agent-outcome')).toContainText('Changes from this turn were reverted');
  await page.locator('.agent-outcome').getByRole('button',{name:'Redo',exact:true}).click();await expect(page.locator('.agent-outcome').getByRole('button',{name:'Undo turn changes',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Select model',exact:true}).click();await expect(page.getByRole('menuitemradio',{name:/text-only-loopback/})).toBeDisabled();await page.getByRole('menuitemradio',{name:/second-loopback/}).click();
  await input.fill('A follow-up about that result.');await input.press('Enter');await accept(page);await expect(page.locator('.agent-turn').last()).toContainText('Previous notebook work');
  expect(bodies.at(-1)!.model).toBe('second-loopback');expect(bodies.at(-1)!.messages.some(message=>message.role==='assistant'&&message.content?.includes('已修改'))).toBe(true);
  await expect(page.locator('.agent-turn').last().locator('.agent-signature')).toContainText('second-loopback');
  const before=await page.locator('.notebook-cell').count();await page.locator('.agent-turn').last().getByRole('button',{name:'Insert as cell',exact:true}).click();await expect(page.locator('.notebook-cell')).toHaveCount(before+1);await expect(page.locator('.notebook-cell').last()).toHaveAttribute('data-status','Stale');
  await input.fill('retain this draft');await page.getByRole('tab',{name:'Variables',exact:true}).click();await page.getByRole('tab',{name:'Assistant',exact:true}).click();await expect(input).toHaveValue('retain this draft');
  await page.getByRole('button',{name:'Context usage',exact:true}).click();await expect(page.getByRole('dialog',{name:'Context usage'})).toContainText('Input 120 / output 20');await page.getByRole('button',{name:'View full turn input'}).click();await expect(page.locator('.agent-full-input')).toContainText('笔记本操作说明');expect(await page.locator('.agent-full-input').textContent()).not.toContain('synthetic-ui-key');await page.getByRole('dialog',{name:'Context usage'}).getByRole('button',{name:'Close',exact:true}).click();
  await page.screenshot({animations:'disabled',path:`../target/plan-5/f3-assistant-${width}.png`});
  if(width===1440){await page.reload();await page.getByRole('button',{name:'Conversation history',exact:true}).click();await page.getByRole('button').filter({hasText:'Restore notebook and conversation'}).click();await expect(page.locator('.agent-turn')).toHaveCount(2);await expect(page.locator('.notebook-cell')).toHaveCount(2);await page.getByRole('textbox',{name:'Math input 1',exact:true}).fill('let a=7');await input.fill('After restoring the history');await input.press('Enter');await accept(page);await expect(page.locator('.agent-turn').last()).toContainText('Previous notebook work');}

});

test('new agent stop and provider failures retain factual state and allow new typing',async({page,context})=>{
  await setup(page);const gate=Promise.withResolvers<void>();let calls=0;
  await context.route('https://ai-provider.invalid/**',async route=>{calls++;if(calls===1)await gate.promise;await route.fulfill({status:401,contentType:'application/json',body:JSON.stringify({error:{message:'Synthetic authentication failure'}})}).catch(()=>{});});
  await page.goto('/');const input=page.getByRole('textbox',{name:'Message to assistant',exact:true});await input.fill('A read only question');await input.press('Enter');await accept(page);await expect(page.locator('.agent-composer .agent-stop')).toBeVisible();
  await expect.poll(()=>calls).toBe(1);await input.fill('draft written while running');await input.press('ControlOrMeta+.');await expect(page.locator('.agent-end')).toContainText('Stopped');gate.resolve();await expect(input).toHaveValue('draft written while running');expect(calls).toBe(1);
  await page.locator('.agent-composer').getByRole('button',{name:'Send',exact:true}).click();await expect(page.locator('.agent-turn').last().locator('.agent-end')).toContainText('Request failed');await expect(input).toHaveValue('');
  await input.fill('not yet accepted');await page.getByRole('button',{name:'New conversation',exact:true}).click();await expect(input).toHaveValue('not yet accepted');
});

test('legacy Ask displays read only while mathematical error repair remains an explicit proposal',async({page,context})=>{
  await setup(page);await context.route('https://ai-provider.invalid/**',async route=>{await route.fulfill({status:200,contentType:'text/event-stream',body:`data: ${JSON.stringify({choices:[{index:0,delta:{content:JSON.stringify({wolfram:'Solve[x^2==9,x]',explanation:'Proposed repair'})},finish_reason:'stop'}]})}\n\ndata: [DONE]\n\n`});});
  await page.goto('/');await page.locator('input[accept*=".omnb"]').setInputFiles({name:'legacy.omnb',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,title:'Legacy',cells:[{id:'legacy-ask',kind:'Ask',dialect:'Auto',source:'A retained old question'}]}))});await expect(page.locator('.legacy-ask')).toHaveText('A retained old question');await expect(page.getByRole('textbox',{name:'Question input 1',exact:true})).toHaveCount(0);await page.getByRole('button',{name:'More',exact:true}).click();await page.getByRole('button',{name:'New notebook',exact:true}).click();await page.getByRole('button',{name:'＋ Math',exact:true}).click();const math=page.getByRole('textbox',{name:'Math input 1',exact:true});await math.fill('solve(x^2==9,x');await math.press('ControlOrMeta+Enter');await expect(page.locator('.notebook-cell')).toHaveAttribute('data-status','Error');await page.getByRole('button',{name:'Fix with AI',exact:true}).click();await page.getByRole('button',{name:'Send to AI',exact:true}).click();await expect(page.locator('.repair-diff')).toContainText('solve');await expect(math).toContainText('solve(x^2==9,x');await page.locator('.repair-suggestion').getByRole('button',{name:'Apply',exact:true}).click();await expect(page.locator('.notebook-cell')).toHaveAttribute('data-status','Stale');await page.getByRole('button',{name:'Run 1',exact:true}).click();await expect(page.locator('.solution-list')).toContainText('3');
});

test('discussion sends configured image derivatives, retains arbitrary files and keeps rejected drafts',async({page,context})=>{
  await setup(page);await page.addInitScript(()=>{localStorage.setItem('openmath-agent-model-metadata-v1',JSON.stringify({deepseek:{supportsImages:true}}));const config=JSON.parse(localStorage.getItem('openmath-settings-v1')!);config.profiles[0].supports_tools=false;localStorage.setItem('openmath-settings-v1',JSON.stringify(config));});
  const bodies:{messages:{role:string;content:unknown}[]}[]=[];
  await context.route('https://ai-provider.invalid/**',async route=>{const body=route.request().postDataJSON();bodies.push(body);await route.fulfill({status:200,contentType:'text/event-stream',body:s1Response(body)});});
  await page.goto('/');await page.getByRole('button',{name:'＋ Math',exact:true}).click();await page.getByRole('textbox',{name:'Math input 1',exact:true}).fill('2+2');
  await page.getByRole('button',{name:'Assistant mode',exact:true}).click();await page.getByRole('menuitemradio',{name:/Discuss/}).click();
  await page.locator('.agent-composer input[type=file]').setInputFiles([{name:'reference.png',mimeType:'image/png',buffer:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAEUlEQVR4nGP4z8DwH4RBCAwBRcwH+conGc8AAAAASUVORK5CYII=','base64')},{name:'retained.bin',mimeType:'application/octet-stream',buffer:Buffer.from('ARBITRARY_FILE_ORIGINAL_ONLY')}]);
  await expect(page.locator('.agent-attachments')).toContainText('Cannot send to this model yet');
  const input=page.getByRole('textbox',{name:'Message to assistant',exact:true});await input.fill('Please inspect @cell9');await input.press('Enter');await accept(page);await expect(page.getByRole('alert')).toContainText('Unknown cell reference');await expect(input).toHaveValue('Please inspect @cell9');await expect(page.locator('.agent-composer .agent-file')).toHaveCount(2);expect(bodies).toHaveLength(0);
  await input.fill('Please inspect @cell1');await input.press('Enter');await expect(page.locator('.agent-end')).toContainText('Turn ended');await expect(page.locator('.agent-outcome')).toHaveCount(0);await expect(page.locator('.agent-user .agent-file')).toHaveCount(2);await expect(page.locator('.agent-user')).toContainText('Not sent to the model; original retained');await expect(input).toHaveValue('');
  const user=bodies[0]!.messages.findLast(message=>message.role==='user')!;expect(JSON.stringify(user)).toContain('data:image/png;base64,');expect(JSON.stringify(user)).not.toContain('ARBITRARY_FILE_ORIGINAL_ONLY');expect(JSON.stringify(user)).toContain('2+2');
  const stored=await page.evaluate(()=>new Promise<{count:number;original:boolean}>(resolve=>{const request=indexedDB.open('openmath.agent.v1');request.onsuccess=()=>{const db=request.result,tx=db.transaction('attachments'),read=tx.objectStore('attachments').getAll();read.onsuccess=()=>{const files=read.result as {name:string;original:Blob}[];void Promise.all(files.map(file=>file.original.text())).then(values=>{resolve({count:files.length,original:values.includes('ARBITRARY_FILE_ORIGINAL_ONLY')});db.close();});};};}));expect(stored).toEqual({count:2,original:true});
});

test('streamed blocks keep their DOM, upward reading does not jump, and the composer resizes to 35 percent',async({page})=>{
  const {createServer}=await import('node:http');
  const server=createServer((request,response)=>{
    response.setHeader('Access-Control-Allow-Origin','*');response.setHeader('Access-Control-Allow-Headers','Content-Type,Authorization');response.setHeader('Access-Control-Allow-Methods','POST,OPTIONS');if(request.method==='OPTIONS'){response.writeHead(204);response.end();return;}
    response.writeHead(200,{'Content-Type':'text/event-stream'});let count=0;
    const frame=(content:string,finish:string|null=null)=>`data: ${JSON.stringify({id:'streamed-response',model:'deepseek-flash',choices:[{index:0,delta:{content},finish_reason:finish}]})}\n\n`;
    response.write(frame('A stable first paragraph.\n\n'));const timer=setInterval(()=>{count++;response.write(frame(`Paragraph ${count}. ${'Read this recorded explanation. '.repeat(10)}\n\n`));if(count===24){clearInterval(timer);response.end(frame('', 'stop')+'data: [DONE]\n\n');}},75);response.on('close',()=>clearInterval(timer));
  });
  await new Promise<void>(resolve=>server.listen(0,'127.0.0.1',resolve));const port=(server.address() as import('node:net').AddressInfo).port;
  try{
    await fixtureProfile(page,`http://127.0.0.1:${port}/v1`,'synthetic-local-key');await page.goto('/');
    const input=page.getByRole('textbox',{name:'Message to assistant',exact:true});await input.fill('A long response');await input.press('Enter');await accept(page);
    await expect(page.locator('.agent-text')).toContainText('Paragraph 6');const block=await page.locator('.agent-text .markdown').first().elementHandle();const transcript=page.locator('.agent-transcript');await transcript.hover();await page.mouse.wheel(0,-10000);
    await expect(page.getByRole('button',{name:'New messages ⌄',exact:true})).toBeVisible();await expect(page.locator('.agent-end')).toContainText('Turn ended');expect(await transcript.evaluate(element=>element.scrollTop)).toBeLessThan(8);expect(await block!.evaluate(element=>element.isConnected)).toBe(true);
    await page.getByRole('button',{name:'New messages ⌄',exact:true}).click();await expect.poll(()=>transcript.evaluate(element=>element.scrollHeight-element.scrollTop-element.clientHeight)).toBeLessThan(36);
    await input.fill('line\n'.repeat(80));await page.setViewportSize({width:1280,height:600});await expect.poll(async()=>{const sizes=await page.locator('.assistant-panel').evaluate(panel=>({panel:panel.clientHeight,editor:panel.querySelector('textarea')!.clientHeight}));return sizes.editor<=sizes.panel*.35+1;}).toBe(true);expect(await input.evaluate(element=>element.scrollHeight>element.clientHeight)).toBe(true);
    const mode=page.getByRole('button',{name:'Assistant mode',exact:true});await mode.press('ArrowDown');await expect(page.getByRole('menuitemradio',{name:/Execute/})).toBeFocused();await page.getByRole('menuitemradio',{name:/Execute/}).press('ArrowDown');await page.getByRole('menuitemradio',{name:/Discuss/}).press('Enter');await expect(mode).toContainText('Discuss');await expect(mode).toBeFocused();
    const model=page.getByRole('button',{name:'Select model',exact:true});await model.click();await page.getByRole('textbox',{name:'Search models',exact:true}).fill('deepseek');await page.getByRole('textbox',{name:'Search models',exact:true}).press('Escape');await expect(model).toBeFocused();
  }finally{server.closeAllConnections();await new Promise<void>(resolve=>server.close(()=>resolve()));}
});
