import {rolldown} from '../../app/node_modules/rolldown/dist/index.mjs';
import {fileURLToPath} from 'node:url';
import fs from 'node:fs/promises';
const root=fileURLToPath(new URL('../..',import.meta.url));
const bundle=await rolldown({input:root+'/ios/Agent/runtime.mjs',platform:'neutral',
  plugins:[{name:'mobile-pi-host',resolveId(id){if(id==='@earendil-works/pi-ai')return root+'/ios/Agent/pi-host.mjs';if(id==='@earendil-works/pi-agent-core')return root+'/app/node_modules/@earendil-works/pi-agent-core/dist/agent.js';}}]});
const {output}=await bundle.generate({format:'iife'});await bundle.close();
if(output.length!==1||output[0].type!=='chunk')throw new Error('Mobile runtime must be one static interpreter bundle');
const artifacts=new Map([['mobile-agent.js',output[0].code],
  ['Pi-MIT.txt',await fs.readFile(root+'/macos/OpenMathNative/Resources/Licenses/Agent/earendil-works__pi-7c10bd433749.txt','utf8')],
  ['TypeBox-MIT.txt',await fs.readFile(root+'/app/node_modules/typebox/license','utf8')],
  ['mobile-prompt.json',await fs.readFile(root+'/agent/prompts/generated/prompt.json','utf8')]]);
const schema=JSON.parse(await fs.readFile(root+'/docs/design/agent-tools.schema.json','utf8'));
const expand=value=>Array.isArray(value)?value.map(expand):value&&typeof value==='object'?value.$ref?expand(schema.$defs[value.$ref.split('/').at(-1)]):Object.fromEntries(Object.entries(value).map(([key,v])=>[key,expand(v)])):value;
const names=schema.properties.tool.enum;
const declarations=names.map(name=>({name,description:'OpenMath '+name+'. Follow the provided schema; references are document-scoped. No shell, file paths or external URLs.',parameters:expand(schema.$defs[name+'_args'])}));
artifacts.set('mobile-tools.json',JSON.stringify(declarations));
for(const [name,contents] of artifacts){
  const file=root+'/ios/OpenMath/Resources/'+name;
  if(process.argv.includes('--check')){if(await fs.readFile(file,'utf8')!==contents)throw new Error('Stale mobile resource: '+name);}
  else await fs.writeFile(file,contents);
}
console.log('Pinned Pi 1.0.4 interpreter resources '+(process.argv.includes('--check')?'match':'generated')+': '+output[0].code.length+' JS bytes');
