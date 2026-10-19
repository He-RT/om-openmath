import type {LlmConfig} from '../kernel/generated/LlmConfig';
import type {ProfileConfig} from '../kernel/generated/ProfileConfig';
import {publicSettings} from './browserSettings';
import {newProfile,features,validationError} from '../components/settings/presets';
export interface RegisteredModel {id:string;apiName:string;displayName:string;tools:boolean;images:boolean;contextLimit:number|null}
export interface RegisteredProvider {id:string;name:string;baseURL:string;kind:ProfileConfig['kind'];requiresKey:boolean;models:RegisteredModel[]}
export interface ModelRegistryDocument {version:1;providers:RegisteredProvider[]}
export interface AssistantPreferences {sendShortcut:'enter'|'command';defaultMode:'execute'|'discuss';defaultModel:string}
export interface ModelDraft extends RegisteredModel {temperature:number;maxTokens:number;timeoutMs:number;json:boolean;extraBody:Record<string,unknown>;parametersSource:string}
export interface ProviderDraft extends Omit<RegisteredProvider,'models'> {key:string|null;remember:boolean;sourceProfile:string|null;headers:Record<string,string>;headersSource:string;models:ModelDraft[]}
export const defaultAssistantPreferences:AssistantPreferences={sendShortcut:'enter',defaultMode:'execute',defaultModel:''};
export const settingsChanged='openmath-model-settings-changed';
export function assistantPreferences():AssistantPreferences{
  const value=publicSettings().assistant;return {sendShortcut:value?.sendShortcut==='command'?'command':'enter',defaultMode:value?.defaultMode==='discuss'?'discuss':'execute',defaultModel:typeof value?.defaultModel==='string'?value.defaultModel:''};
}
export function modelMetadataFromRegistry(id:string){
  const registry=publicSettings().registry;for(const provider of registry?.providers??[])for(const model of provider.models)if(model.id===id)return {providerName:provider.name,displayName:model.displayName,supportsImages:model.images,contextWindow:model.contextLimit};return undefined;
}
export function newModel():ModelDraft{return {id:'model-'+crypto.randomUUID(),apiName:'',displayName:'',tools:true,images:false,contextLimit:null,temperature:.2,maxTokens:8192,timeoutMs:60000,json:false,extraBody:{},parametersSource:'{}'};}
export function newProvider():ProviderDraft{return {id:'provider-'+crypto.randomUUID(),name:'',baseURL:'',kind:'openai_chat',requiresKey:true,key:null,remember:false,sourceProfile:null,headers:{},headersSource:'{}',models:[newModel()]};}
function draftModel(model:RegisteredModel,profile:ProfileConfig):ModelDraft{return {...model,apiName:profile.model,tools:profile.supports_tools,temperature:profile.temperature,maxTokens:profile.max_tokens,timeoutMs:profile.timeout_ms,json:profile.supports_json_mode,extraBody:profile.extra_body??{},parametersSource:JSON.stringify(profile.extra_body??{},null,2)};}
/** Use this application's current configuration only. Unconfigured built-in examples are not accounts. */
export function registryDraft(llm:LlmConfig,remembered:Set<string>):ProviderDraft[]{
  const registry=publicSettings().registry;
  if(registry?.version===1&&Array.isArray(registry.providers))return registry.providers.map(provider=>{
    const pairs=provider.models.flatMap(model=>{const profile=llm.profiles.find(profile=>profile.name===model.id);return profile?[{model,profile}]:[];}),first=pairs[0]?.profile;
    return {...provider,baseURL:first?.base_url??provider.baseURL,kind:first?.kind??provider.kind,requiresKey:first?.requires_api_key!==false,key:first?.api_key??null,sourceProfile:first?.name??null,headers:first?.extra_headers??{},headersSource:JSON.stringify(first?.extra_headers??{},null,2),remember:pairs.some(pair=>remembered.has(pair.profile.name)),models:pairs.map(({model,profile})=>draftModel(model,profile))};
  });
  return llm.profiles.filter(profile=>profile.requires_api_key===false||!!profile.api_key).map(profile=>{
    let host=profile.name;try{host=new URL(profile.base_url).host;}catch{/* Validation is shown when the draft is saved. */}
    return {id:'provider-'+profile.name,name:host,baseURL:profile.base_url,kind:profile.kind,requiresKey:profile.requires_api_key!==false,key:profile.api_key,sourceProfile:profile.name,headers:profile.extra_headers,headersSource:JSON.stringify(profile.extra_headers,null,2),remember:remembered.has(profile.name),models:[draftModel({id:profile.name,apiName:profile.model,displayName:profile.model,tools:profile.supports_tools,images:false,contextLimit:null},profile)]};
  });
}
export function buildModelSettings(base:LlmConfig,providers:ProviderDraft[],assistant:AssistantPreferences){
  const registry:ModelRegistryDocument={version:1,providers:providers.map(provider=>({id:provider.id,name:provider.name,baseURL:provider.baseURL,kind:provider.kind,requiresKey:provider.requiresKey,models:provider.models.map(({id,apiName,displayName,tools,images,contextLimit})=>({id,apiName,displayName:displayName||apiName,tools,images,contextLimit}))}))};
  const inheritance:Record<string,string>={},remembered:string[]=[],profiles=providers.flatMap(provider=>provider.models.map(model=>{
    if(provider.sourceProfile&&provider.key==='***')inheritance[model.id]=provider.sourceProfile;if(provider.remember)remembered.push(model.id);
    return {...newProfile(model.id),kind:provider.kind,base_url:provider.baseURL.trim(),model:model.apiName.trim(),api_key:provider.key,api_key_env:null,requires_api_key:provider.requiresKey,temperature:model.temperature,max_tokens:model.maxTokens,timeout_ms:model.timeoutMs,supports_tools:model.tools,supports_json_mode:model.json,extra_headers:jsonObject(provider.headersSource,true) as Record<string,string>,extra_body:jsonObject(model.parametersSource)};
  }));
  const llm={...structuredClone(base),profiles};for(const feature of features){const mapped=profiles.find(profile=>profile.name===llm[feature]);if(!mapped||feature!=='complete'&&(!['openai_chat','anthropic'].includes(mapped.kind)||feature==='chat'&&!mapped.supports_tools))llm[feature]='';}
  const fallback=profiles.find(profile=>['openai_chat','anthropic'].includes(profile.kind)&&profile.supports_tools),preferred=profiles.find(profile=>profile.name===assistant.defaultModel);
  if(!llm.chat)llm.chat=preferred?.supports_tools?preferred.name:fallback?.name??'';
  const nextAssistant={...assistant,defaultModel:preferred?.name??fallback?.name??profiles.find(profile=>['openai_chat','anthropic'].includes(profile.kind))?.name??''};
  return {llm,inheritance,remembered,registry,assistant:nextAssistant};
}
export function modelSettingsError(value:ReturnType<typeof buildModelSettings>):string|null{
  if(value.registry.providers.length>64||value.llm.profiles.length>512)return 'Too many providers or models';
  const ids=new Set<string>();for(const provider of value.registry.providers){if(!provider.name.trim()||provider.name.length>256||ids.has(provider.id))return 'Provider names are required';ids.add(provider.id);if(!provider.models.length)return 'Add at least one model to each provider';for(const model of provider.models)if(model.contextLimit!==null&&(!Number.isSafeInteger(model.contextLimit)||model.contextLimit<1||model.contextLimit>100000000))return 'Invalid context limit';}
  const protectedFields=new Set(['model','messages','tools','tool_choice','stream','max_tokens','max_completion_tokens','temperature','functions','function_call','headers','url','api_key']);
  for(const name of ['prompt','suffix','stop','stop_sequences','response_format','system','options','base_url'])protectedFields.add(name);
  if(value.llm.profiles.some(profile=>Object.keys(profile.extra_body??{}).some(key=>protectedFields.has(key))))return 'Advanced parameters cannot replace the model, messages, tools or transport';
  const preferred=value.llm.profiles.find(profile=>profile.name===value.assistant.defaultModel);if(preferred&&value.assistant.defaultMode==='execute'&&!preferred.supports_tools)return 'The default model needs tool calling for Execute mode';
  if(value.llm.profiles.some(profile=>profile.kind==='anthropic'&&profile.temperature>1))return 'Anthropic temperature must be between 0 and 1';
  return validationError(value.llm);
}
export interface ModelSettingsChange {llm:LlmConfig;inheritance:Record<string,string>;remembered:string[];registry:ModelRegistryDocument;assistant:AssistantPreferences}

export function jsonObject(source:string,headers=false):Record<string,unknown>{const value=JSON.parse(source||'{}') as unknown;if(!value||typeof value!=='object'||Array.isArray(value)||headers&&Object.values(value).some(item=>typeof item!=='string'))throw new Error('Expected a JSON object'+(headers?' with string header values':''));return value as Record<string,unknown>;}
