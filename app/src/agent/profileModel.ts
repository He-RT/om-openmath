import type {ProfileConfig} from '../kernel/generated/ProfileConfig';
import type {BrowserAgentModel} from './runtimeTypes';
import {sha256} from './patch';
import {modelMetadataFromRegistry} from '../state/modelRegistry';
export interface AgentModelMetadata {providerName?:string;displayName?:string;supportsImages?:boolean;contextWindow?:number|null}
export function modelMetadata(name:string):AgentModelMetadata{
  const configured=modelMetadataFromRegistry(name);if(configured)return configured;
  try{const all=JSON.parse(localStorage.getItem('openmath-agent-model-metadata-v1')??'{}') as Record<string,AgentModelMetadata>;return all[name]??{};}catch{return {};}
}
export function configuredModels(profiles:ProfileConfig[]){return profiles.filter(profile=>{try{const url=new URL(profile.base_url);return (profile.kind==='openai_chat'||profile.kind==='anthropic')&&['http:','https:'].includes(url.protocol)&&!url.username&&!url.password&&!url.search&&!url.hash&&!!profile.model.trim()&&(profile.requires_api_key===false||!!profile.api_key);}catch{return false;}});}
/** Freeze only a configured profile. Secrets are consumed by transport and never returned as UI metadata. */
export async function prepareProfileModel(profile:ProfileConfig,revision:number):Promise<BrowserAgentModel>{
  const url=new URL(profile.base_url);if(!['http:','https:'].includes(url.protocol)||url.username||url.password||url.search||url.hash)throw new Error('Invalid model endpoint');
  if(profile.kind!=='openai_chat'&&profile.kind!=='anthropic')throw new Error('Unsupported model protocol');
  if(profile.requires_api_key!==false&&(!profile.api_key||profile.api_key==='***'))throw new Error('API key is unavailable; enter it again in model settings');
  const base=profile.base_url.replace(/\/+$/,'');const headers:[string,string][]=[['Content-Type','application/json']];
  if(profile.kind==='anthropic'){headers.push(['anthropic-version','2023-06-01']);if(profile.api_key)headers.push(['x-api-key',profile.api_key]);}
  else if(profile.api_key)headers.push(['Authorization','Bearer '+profile.api_key]);
  const merged=new Map(headers.map(([key,value])=>[key.toLowerCase(),[key,value] as [string,string]]));for(const [key,value] of Object.entries(profile.extra_headers))merged.set(key.toLowerCase(),[key,value]);
  if(profile.kind==='anthropic')merged.set('anthropic-dangerous-direct-browser-access',['anthropic-dangerous-direct-browser-access','true']);
  headers.splice(0,headers.length,...merged.values());
  const protectedFields=new Set(['model','messages','tools','tool_choice','stream','max_tokens','max_completion_tokens','temperature','functions','function_call','headers','url','api_key','prompt','suffix','stop','stop_sequences','response_format','system','options','base_url']);
  if(Object.keys(profile.extra_body??{}).some(key=>protectedFields.has(key)))throw new Error('Advanced parameters cannot replace agent input');
  const metadata=modelMetadata(profile.name),providerRef='provider-'+await sha256(profile.name),routeRef='route-'+await sha256(JSON.stringify({kind:profile.kind,url:base,revision}));
  return {apiModel:profile.model,displayName:metadata.displayName??profile.model,supportsTools:profile.supports_tools,supportsImages:metadata.supportsImages===true,temperature:profile.temperature,extraBody:structuredClone(profile.extra_body??{}),binding:{provider_ref:providerRef,route_ref:routeRef,model_ref:'model-'+await sha256(profile.name+':'+profile.model+':'+revision),adapter_version:'single-turn-v1',protocol:profile.kind==='anthropic'?'anthropic_messages':'chat_completions'},url:base+(profile.kind==='anthropic'?'/messages':'/chat/completions'),headers,maxOutputTokens:profile.max_tokens,timeoutMs:profile.timeout_ms,contextWindow:metadata.contextWindow??null};
}
